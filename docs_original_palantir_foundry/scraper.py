#!/usr/bin/env python3
# -*- coding: utf-8 -*-

from __future__ import annotations

import argparse
import json
import logging
import mimetypes
import re
import sys
import time
from pathlib import Path
from typing import Dict, List, Optional, Set, Tuple
from urllib.parse import urljoin, urlparse, unquote

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    handlers=[logging.StreamHandler(sys.stdout)],
)
log = logging.getLogger("foundry-scraper")

BASE_URL = "https://www.palantir.com/docs/foundry/"
DOMAIN = "www.palantir.com"
DOCS_PREFIX = "/docs/foundry"

HEADERS = {
    "User-Agent": (
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
        "AppleWebKit/537.36 (KHTML, like Gecko) "
        "Chrome/124.0.0.0 Safari/537.36"
    ),
    "Accept-Language": "en-US,en;q=0.9",
}

PRIMARY_ROOTS = [
    "AI Platform (AIP)",
    "Data connectivity & integration",
    "Model connectivity & development",
    "Ontology building",
    "Developer toolchain",
    "Use case development",
    "Observability",
    "Analytics",
    "Product delivery",
    "Security & governance",
    "Management & enablement",
]

PRIMARY_ROOT_SET = set(PRIMARY_ROOTS)

ROOT_SLUG_TO_SECTION = {
    "aip": "AI Platform (AIP)",
    "data-integration": "Data connectivity & integration",
    "data-connection": "Data connectivity & integration",
    "model-integration": "Model connectivity & development",
    "integrate-models": "Model connectivity & development",
    "ontology": "Ontology building",
    "object-link-types": "Ontology building",
    "dev-toolchain": "Developer toolchain",
    "building-pipelines": "Developer toolchain",
    "functions": "Developer toolchain",
    "transforms-python": "Developer toolchain",
    "architecture-center": "Developer toolchain",
    "devops": "Developer toolchain",
    "workshop": "Use case development",
    "observability": "Observability",
    "evaluate-models": "Observability",
    "analytics": "Analytics",
    "app-building": "Product delivery",
    "security": "Security & governance",
    "administration": "Management & enablement",
    "authentication": "Management & enablement",
}

BLOCKED_ROOT_SLUGS = {
    "announcements",
    "api-reference",
    "custom-docs",
    "carbon",
    "consumer-mode",
    "code-workbook",
    "contour",
    "quiver",
    "slate",
    "getting-started",
    "index",
}

AUTH_GROUP_BY_SLUG = {
    "overview": ["Authentication", "Overview"],
    "saml-getting-started": ["Authentication", "SAML", "Getting started"],
    "saml-azure-ad": ["Authentication", "SAML", "Entra ID (Azure AD)"],
    "saml-okta": ["Authentication", "SAML", "Okta"],
    "saml-other-idp": ["Authentication", "SAML", "Other identity providers"],
    "saml-provider-update": ["Authentication", "SAML", "SAML provider updates in Control Panel"],
    "oidc-getting-started": ["Authentication", "OIDC", "Getting started"],
    "user-directory": ["Authentication", "Self-service user directory", "Manage users within your enrollment"],
    "multi-factor-auth": ["Authentication", "Self-service user directory", "Multi-factor authentication"],
    "intake-forms": ["Authentication", "Self-service user directory", "Intake forms"],
    "org-assignment": ["Authentication", "Self-service user directory", "Organization assignment"],
    "group-assignment": ["Authentication", "Self-service user directory", "Group assignment"],
    "test-provider-integration": ["Authentication", "Self-service user directory", "Enable and test identity provider integration"],
    "host-settings": ["Authentication", "Self-service user directory", "Host settings"],
}

def clean_text(text: str) -> str:
    if not text:
        return ""
    text = text.replace("\xa0", " ")
    text = re.sub(r"[↗↙↘↖→←↑↓⌄⌃▾▸◂•]+", " ", text)
    text = re.sub(r"\s+", " ", text).strip()
    return text

def normalize_label(text: str) -> str:
    text = clean_text(text)
    text = re.sub(r"^[\-\+\*]\s*", "", text).strip()
    return text

def sanitize_component(text: str, fallback: str = "item") -> str:
    text = normalize_label(text)
    text = text.replace("/", " - ")
    text = re.sub(r'[<>:"\\|?*\x00-\x1f]', "", text)
    text = re.sub(r"\s+", " ", text).strip(" .")
    return text or fallback

def normalized_url(url: str) -> str:
    clean = url.split("#")[0].split("?")[0]
    return clean.rstrip("/") + "/"

def is_valid_docs_url(url: str) -> bool:
    parsed = urlparse(url)
    return (
        parsed.scheme in ("http", "https")
        and parsed.netloc == DOMAIN
        and parsed.path.startswith(DOCS_PREFIX + "/")
    )

def docs_parts(url: str) -> List[str]:
    path = urlparse(url).path
    path = re.sub(r"^/docs/foundry/?", "", path).strip("/")
    return [p for p in path.split("/") if p]

def root_slug(url: str) -> str:
    parts = docs_parts(url)
    return parts[0] if parts else ""

def leaf_slug(url: str) -> str:
    parts = docs_parts(url)
    return parts[-1] if parts else "overview"

def is_allowed_docs_url(url: str) -> bool:
    slug = root_slug(url)
    if not slug:
        return True
    return slug not in BLOCKED_ROOT_SLUGS

def create_requests_session():
    try:
        import requests
    except ImportError:
        log.error("Falta requests: pip install requests")
        sys.exit(1)
    s = requests.Session()
    s.headers.update(HEADERS)
    return s

class CrawlerState:
    def __init__(self, output_dir: Path):
        self._file = output_dir / ".scraper_state.json"
        self.visited: Set[str] = set()
        self.queue: List[str] = [BASE_URL]
        self.semantic_map: Dict[str, List[str]] = {}
        self._load()

    def _load(self):
        if self._file.exists():
            data = json.loads(self._file.read_text(encoding="utf-8"))
            self.visited = set(data.get("visited", []))
            self.queue = data.get("queue", [BASE_URL])
            self.semantic_map = data.get("semantic_map", {})
            log.info("Estado recuperado: %s visitadas, %s en cola.", len(self.visited), len(self.queue))

    def save(self):
        payload = {
            "visited": sorted(self.visited),
            "queue": self.queue,
            "semantic_map": self.semantic_map,
        }
        self._file.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")

    def mark_visited(self, url: str):
        self.visited.add(normalized_url(url))

    def next_url(self) -> Optional[str]:
        while self.queue:
            url = normalized_url(self.queue.pop(0))
            if url:
                return url
        return None

    def add_urls(self, urls: List[str]):
        for url in urls:
            clean = normalized_url(url)
            if clean not in self.visited and clean not in self.queue and is_allowed_docs_url(clean):
                self.queue.append(clean)

def semantic_path_from_url(url: str) -> Optional[List[str]]:
    parts = docs_parts(url)
    if not parts:
        return ["Index", "Overview"]

    slug = parts[0]
    leaf = parts[-1]

    if slug not in ROOT_SLUG_TO_SECTION:
        return None

    root = ROOT_SLUG_TO_SECTION[slug]

    if slug == "authentication":
        mapped = AUTH_GROUP_BY_SLUG.get(leaf)
        if mapped:
            return [root] + mapped
        if leaf == "overview":
            return [root, "Authentication", "Overview"]
        return [root, "Authentication", leaf.replace("-", " ").title()]

    if slug == "administration":
        if leaf == "overview":
            return [root, "Overview"]
        return [root, leaf.replace("-", " ").title()]

    if len(parts) == 1 or leaf == slug or leaf == "overview":
        return [root, "Overview"]

    return [root, leaf.replace("-", " ").title()]

def semantic_filepath(url: str, output_dir: Path) -> Optional[Path]:
    labels = semantic_path_from_url(url)
    if not labels:
        return None
    labels = [sanitize_component(x) for x in labels]
    dirs = labels[:-1]
    leaf = labels[-1]
    return output_dir.joinpath(*dirs) / f"{leaf}.md"

def screenshot_path_for_md(md_filepath: Path) -> Path:
    return md_filepath.parent / f"{md_filepath.stem}.screenshot.png"

def assets_dir_for_md(md_filepath: Path) -> Path:
    return md_filepath.parent / f"{md_filepath.stem}_assets"

def guess_extension_from_url_or_type(url: str, content_type: str = "") -> str:
    path_ext = Path(urlparse(url).path).suffix.lower()
    if path_ext in {".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp", ".bmp", ".avif"}:
        return ".jpg" if path_ext == ".jpeg" else path_ext
    if content_type:
        guessed = mimetypes.guess_extension(content_type.split(";")[0].strip())
        if guessed:
            return ".jpg" if guessed == ".jpe" else guessed
    return ".bin"

def pick_image_source(img_tag) -> str:
    for key in ["src", "data-src", "data-original", "data-lazy-src"]:
        value = img_tag.get(key)
        if value and value.strip():
            return value.strip()
    srcset = img_tag.get("srcset") or img_tag.get("data-srcset")
    if srcset:
        first = srcset.split(",")[0].strip().split(" ")[0].strip()
        if first:
            return first
    return ""

def is_probably_icon(img_tag) -> bool:
    cls = " ".join(img_tag.get("class", [])).lower()
    src = (img_tag.get("src") or "").lower()
    if "icon" in cls or "favicon" in src:
        return True
    return False

def download_image(session, image_url: str, asset_dir: Path, index: int) -> Optional[Path]:
    try:
        resp = session.get(image_url, timeout=30, stream=True)
        resp.raise_for_status()
    except Exception:
        return None

    ext = guess_extension_from_url_or_type(image_url, resp.headers.get("Content-Type", ""))
    filepath = asset_dir / f"img_{index:03d}{ext}"

    try:
        asset_dir.mkdir(parents=True, exist_ok=True)
        with open(filepath, "wb") as fh:
            for chunk in resp.iter_content(chunk_size=8192):
                if chunk:
                    fh.write(chunk)
        return filepath
    finally:
        try:
            resp.close()
        except Exception:
            pass

def process_images_in_main(main, page_url: str, md_filepath: Path, session, max_images: int) -> int:
    images = main.find_all("img")
    seen = set()
    saved = 0
    asset_dir = assets_dir_for_md(md_filepath)

    for img in images:
        if saved >= max_images:
            break
        if is_probably_icon(img):
            continue
        raw = pick_image_source(img)
        if not raw or raw.startswith("data:"):
            continue
        absolute = urljoin(page_url, raw)
        if absolute in seen:
            continue
        seen.add(absolute)
        local = download_image(session, absolute, asset_dir, saved + 1)
        if not local:
            continue
        rel = local.relative_to(md_filepath.parent)
        img["src"] = str(rel).replace("\\", "/")
        for attr in ["srcset", "data-src", "data-srcset", "data-original", "data-lazy-src"]:
            if attr in img.attrs:
                del img[attr]
        saved += 1
    return saved

def extract_content(html: str, url: str, md_filepath: Path, session, max_images_per_page: int, screenshot_relpath: Optional[str]):
    try:
        from bs4 import BeautifulSoup
        import markdownify as md_lib
    except ImportError:
        log.error("Faltan dependencias: pip install beautifulsoup4 markdownify")
        sys.exit(1)

    soup = BeautifulSoup(html, "html.parser")

    title = ""
    og = soup.find("meta", property="og:title")
    if og and og.get("content"):
        title = clean_text(og["content"])
    if not title:
        h1 = soup.find("h1")
        title = clean_text(h1.get_text(" ", strip=True)) if h1 else leaf_slug(url).replace("-", " ").title()

    breadcrumb = []
    for a in soup.select("nav[aria-label*='breadcrumb'] a, .breadcrumb a, [data-testid='breadcrumb'] a"):
        txt = clean_text(a.get_text(" ", strip=True))
        if txt:
            breadcrumb.append(txt)

    main = (
        soup.find("main")
        or soup.find("article")
        or soup.find("div", {"id": "content"})
        or soup.find("div", class_=re.compile(r"content|article|docs", re.I))
        or soup.body
    )

    md_body = ""
    if main:
        for tag in main.select(
            "nav, header, footer, script, style, noscript, "
            ".sidebar, .toc, .navigation, .nav-menu, "
            "[aria-hidden='true'], .cookie-banner, "
            ".feedback, .edit-page, .page-nav, .related-pages"
        ):
            tag.decompose()

        process_images_in_main(main, url, md_filepath, session, max_images_per_page)

        md_body = md_lib.markdownify(
            str(main),
            heading_style="ATX",
            bullets="-",
            newline_style="backslash",
        )

    md_body = re.sub(r"\n{3,}", "\n\n", md_body).strip()

    if screenshot_relpath:
        md_body = (
            "## Captura de pantalla\n\n"
            f"![Screenshot]({screenshot_relpath})\n\n---\n\n"
            + md_body
        )

    return {"title": title, "breadcrumb": breadcrumb, "markdown": md_body}

def build_frontmatter(title: str, url: str, breadcrumb: List[str]) -> str:
    safe_title = title.replace('"', '\\"')
    lines = [
        "---",
        f'title: "{safe_title}"',
        f'source: "{url}"',
        f'scraped_at: "{time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())}"',
    ]
    if breadcrumb:
        lines.append(f'breadcrumb: "{" > ".join(breadcrumb)}"')
    lines.append("---")
    return "\n".join(lines)

def save_markdown(filepath: Path, title: str, url: str, breadcrumb: List[str], body: str):
    filepath.parent.mkdir(parents=True, exist_ok=True)
    content = f"{build_frontmatter(title, url, breadcrumb)}\n\n# {title}\n\n{body}\n"
    filepath.write_text(content, encoding="utf-8")
    log.info("  Guardado: %s", filepath)

def expand_navigation(page):
    selectors = [
        "button:has-text('Capabilities')",
        "[role='tab']:has-text('Capabilities')",
        "text=Capabilities",
    ]
    for selector in selectors:
        try:
            loc = page.locator(selector).first
            if loc.count() > 0:
                loc.click(timeout=1000)
                page.wait_for_timeout(300)
                break
        except Exception:
            pass

    for _ in range(8):
        clicked = 0
        for selector in [
            "aside button[aria-expanded='false']",
            "nav button[aria-expanded='false']",
            "[role='treeitem'][aria-expanded='false']",
            "summary",
        ]:
            try:
                loc = page.locator(selector)
                count = min(loc.count(), 40)
                for i in range(count):
                    try:
                        loc.nth(i).click(timeout=500)
                        page.wait_for_timeout(80)
                        clicked += 1
                    except Exception:
                        pass
            except Exception:
                pass
        if clicked == 0:
            break

def scroll_page(page):
    last_height = 0
    for _ in range(8):
        try:
            page.evaluate("window.scrollTo(0, document.body.scrollHeight)")
            page.wait_for_timeout(500)
            h = page.evaluate("document.body.scrollHeight")
            if h == last_height:
                break
            last_height = h
        except Exception:
            break
    try:
        page.evaluate("window.scrollTo(0, 0)")
    except Exception:
        pass

def collect_links(page) -> List[str]:
    try:
        hrefs = page.eval_on_selector_all("a[href]", "els => els.map(e => e.href)")
    except Exception:
        return []
    out = []
    for href in hrefs:
        clean = normalized_url(href)
        if is_valid_docs_url(clean) and is_allowed_docs_url(clean):
            out.append(clean)
    return sorted(set(out))

def crawl_with_playwright(output_dir: Path, delay: float, max_pages: int, max_images_per_page: int, screenshots_enabled: bool):
    try:
        from playwright.sync_api import sync_playwright, TimeoutError as PWTimeout
    except ImportError:
        log.error("Playwright no instalado. pip install playwright && playwright install chromium")
        sys.exit(1)

    session = create_requests_session()
    state = CrawlerState(output_dir)

    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        context = browser.new_context(
            user_agent=HEADERS["User-Agent"],
            locale="en-US",
            extra_http_headers={"Accept-Language": "en-US,en;q=0.9"},
            viewport={"width": 1600, "height": 1200},
        )
        page = context.new_page()
        page.set_default_timeout(30000)

        while True:
            url = state.next_url()
            if url is None:
                break
            if url in state.visited:
                continue
            if not is_allowed_docs_url(url):
                continue
            if max_pages and len(state.visited) >= max_pages:
                break

            path = semantic_filepath(url, output_dir)
            if path is None:
                continue

            state.mark_visited(url)
            log.info("[%s] -> %s", len(state.visited), url)

            try:
                page.goto(url, wait_until="networkidle", timeout=30000)
                try:
                    page.wait_for_selector("main, article, h1", timeout=8000)
                except PWTimeout:
                    pass
                expand_navigation(page)
                scroll_page(page)
                html = page.content()
            except Exception as e:
                log.warning("  Error cargando %s: %s", url, e)
                state.save()
                continue

            screenshot_relpath = None
            if screenshots_enabled:
                try:
                    shot = screenshot_path_for_md(path)
                    shot.parent.mkdir(parents=True, exist_ok=True)
                    page.screenshot(path=str(shot), full_page=True, animations="disabled")
                    screenshot_relpath = shot.name
                except Exception as e:
                    log.warning("  Error screenshot %s", e)

            data = extract_content(
                html=html,
                url=url,
                md_filepath=path,
                session=session,
                max_images_per_page=max_images_per_page,
                screenshot_relpath=screenshot_relpath,
            )
            save_markdown(path, data["title"], url, data["breadcrumb"], data["markdown"])

            links = collect_links(page)
            state.add_urls(links)

            time.sleep(delay)
            state.save()

        browser.close()

    state.save()

def generate_index(output_dir: Path):
    readme = output_dir / "README.md"
    md_files = sorted(output_dir.rglob("*.md"))
    md_files = [f for f in md_files if f.name != "README.md"]

    lines = [
        "# Palantir Foundry — Documentación técnica",
        "",
        f"> Generado automáticamente el {time.strftime('%Y-%m-%d')}",
        f"> Fuente: {BASE_URL}",
        "",
        "## Índice",
        "",
    ]

    current = None
    for f in md_files:
        rel = f.relative_to(output_dir)
        folder = str(rel.parent).replace("\\", "/")
        if folder != current:
            current = folder
            lines.append(f"### {folder}")
            lines.append("")
        title = f.stem
        try:
            for line in f.read_text(encoding="utf-8").splitlines():
                if line.startswith("title:"):
                    title = line.replace("title:", "", 1).strip().strip('"')
                    break
        except Exception:
            pass
        lines.append(f"- [{title}]({str(rel).replace(chr(92), '/')})")

    lines.append("")
    readme.write_text("\n".join(lines), encoding="utf-8")
    log.info("Indice generado: %s", readme)

def parse_args():
    parser = argparse.ArgumentParser(description="Foundry docs scraper con estructura semántica controlada")
    parser.add_argument("--output", default="./foundry-docs")
    parser.add_argument("--delay", type=float, default=2.0)
    parser.add_argument("--max-pages", type=int, default=0)
    parser.add_argument("--max-images-per-page", type=int, default=6)
    parser.add_argument("--no-screenshots", action="store_true")
    parser.add_argument("--index-only", action="store_true")
    return parser.parse_args()

def main():
    args = parse_args()
    output_dir = Path(args.output).expanduser().resolve()
    output_dir.mkdir(parents=True, exist_ok=True)

    if not args.index_only:
        crawl_with_playwright(
            output_dir=output_dir,
            delay=args.delay,
            max_pages=args.max_pages,
            max_images_per_page=args.max_images_per_page,
            screenshots_enabled=not args.no_screenshots,
        )

    generate_index(output_dir)
    log.info("Listo. Documentación guardada en: %s", output_dir)

if __name__ == "__main__":
    main()

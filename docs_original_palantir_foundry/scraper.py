#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Palantir Foundry Docs Scraper v3 — Parser CSS-based + Paralelización
=====================================================================

Dependencias:
    pip install playwright requests beautifulsoup4 markdownify
    playwright install chromium

Uso:
    python foundry_docs_scraper.py --output ./foundry-docs
    python foundry_docs_scraper.py --output ./foundry-docs --workers 6 --delay 0.5
    python foundry_docs_scraper.py --output ./foundry-docs --no-screenshots --workers 8
    python foundry_docs_scraper.py --output ./foundry-docs --max-pages 50
    python foundry_docs_scraper.py --output ./foundry-docs --index-only
"""
from __future__ import annotations

import argparse
import asyncio
import json
import logging
import mimetypes
import re
import sys
import time
from pathlib import Path
from typing import Dict, List, Optional, Set, Tuple
from urllib.parse import urljoin, urlparse

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    handlers=[logging.StreamHandler(sys.stdout)],
)
log = logging.getLogger("foundry-scraper")

# ─────────────────────────────────────────────────────────
# Constantes
# ─────────────────────────────────────────────────────────
BASE_URL    = "https://www.palantir.com/docs/foundry/"
DOMAIN      = "www.palantir.com"
DOCS_PREFIX = "/docs/foundry"

UA = (
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
    "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"
)
HEADERS = {"User-Agent": UA, "Accept-Language": "en-US,en;q=0.9"}

# Secciones del sidebar de Foundry con su URL de entrada
SECTIONS: Dict[str, str] = {
    "AI Platform (AIP)":              "https://www.palantir.com/docs/foundry/aip/overview/",
    "Data connectivity & integration":"https://www.palantir.com/docs/foundry/data-connection/overview/",
    "Model connectivity & development":"https://www.palantir.com/docs/foundry/model-integration/overview/",
    "Ontology building":              "https://www.palantir.com/docs/foundry/ontology/overview/",
    "Developer toolchain":            "https://www.palantir.com/docs/foundry/dev-toolchain/overview/",
    "Use case development":           "https://www.palantir.com/docs/foundry/workshop/overview/",
    "Observability":                  "https://www.palantir.com/docs/foundry/observability/overview/",
    "Analytics":                      "https://www.palantir.com/docs/foundry/analytics/overview/",
    "Product delivery":               "https://www.palantir.com/docs/foundry/app-building/overview/",
    "Security & governance":          "https://www.palantir.com/docs/foundry/security/overview/",
    "Management & enablement":        "https://www.palantir.com/docs/foundry/administration/overview/",
}

# Clases CSS del sidebar de Palantir (estables en el diseño actual)
CSS_HEADER = "ptcom-design__header__ryw7sh"       # separador visual (Applications, Enablement…)
CSS_PANEL  = "ptcom-design__panel__ryw7sh"         # carpeta expandible
CSS_PANEL_TITLE = "ptcom-design__panelTitle__ryw7sh"
CSS_PANEL_LIST  = "ptcom-design__panelList__ryw7sh"

# ─────────────────────────────────────────────────────────
# Utilidades
# ─────────────────────────────────────────────────────────
def clean(text: str) -> str:
    if not text:
        return ""
    text = text.replace("\xa0", " ")
    text = re.sub(r"[↗↙↘↖→←↑↓⌄⌃▾▸◂•]+", " ", text)
    return re.sub(r"\s+", " ", text).strip()

def sanitize(text: str, fallback: str = "page") -> str:
    text = clean(text).replace("/", " - ")
    text = re.sub(r'[<>:"\\|?*\x00-\x1f]', "", text)
    text = re.sub(r"\s+", " ", text).strip(" .")
    return text or fallback

def norm_url(url: str) -> str:
    return url.split("#")[0].split("?")[0].rstrip("/") + "/"

def is_docs_url(url: str) -> bool:
    try:
        p = urlparse(url)
        return (
            p.scheme in ("http", "https")
            and p.netloc == DOMAIN
            and p.path.startswith(DOCS_PREFIX + "/")
            and "release-notes" not in p.path
        )
    except Exception:
        return False

def posix(path: Path) -> str:
    return str(path).replace("\\", "/")


# ─────────────────────────────────────────────────────────
# Parser del sidebar (basado en clases CSS reales de Palantir)
# ─────────────────────────────────────────────────────────
def _panel_label(li) -> str:
    pt = li.find(class_=CSS_PANEL_TITLE)
    if pt:
        for svg in pt.find_all("svg"):
            svg.decompose()
        return clean(pt.get_text(" ", strip=True))
    return ""

def parse_sidebar(ul, depth: int = 0) -> List[dict]:
    """
    Parsea recursivamente el UL del sidebar usando clases CSS de Palantir.
    Retorna lista de dicts con: depth, label, href, is_group, is_header
    """
    results = []
    for li in ul.children:
        if not hasattr(li, "name") or li.name != "li":
            continue
        classes = li.get("class") or []

        # Separador visual (Applications, Enablement, Management & enablement…)
        if CSS_HEADER in classes:
            label = clean(li.get_text(" ", strip=True))
            results.append({"depth": depth, "label": label, "href": None,
                             "is_group": False, "is_header": True})
            continue

        # Panel = carpeta expandible con subítems
        if CSS_PANEL in classes:
            label = _panel_label(li)
            if not label:
                continue
            results.append({"depth": depth, "label": label, "href": None,
                             "is_group": True, "is_header": False})
            sub_ul = li.find(class_=CSS_PANEL_LIST)
            if sub_ul:
                results.extend(parse_sidebar(sub_ul, depth + 1))
            continue

        # Ítem de página directa
        a = next(
            (c for c in li.children if hasattr(c, "name") and c.name == "a" and c.get("href")),
            None,
        )
        if a and "/docs/foundry" in a.get("href", "") and "release-notes" not in a["href"]:
            label = clean(a.get_text(" ", strip=True))
            if label:
                results.append({"depth": depth, "label": label, "href": a["href"],
                                 "is_group": False, "is_header": False})
    return results


def build_url_map(tree: List[dict], root_section: str) -> Dict[str, List[str]]:
    """
    Convierte el árbol del sidebar en url -> [ruta, de, carpetas, ..., nombre_pagina]

    Regla clave: los separadores (is_header=True) cambian el contexto visual actual
    (se convierten en subcarpeta). Los paneles (is_group=True) son carpetas.
    Los ítems con href son páginas .md.
    """
    url_map: Dict[str, List[str]] = {}
    group_stack: List[Tuple[int, str]] = []  # (depth, label)
    current_header: Optional[str] = None

    for item in tree:
        depth   = item["depth"]
        label   = item["label"]
        href    = item.get("href")
        is_group   = item["is_group"]
        is_header  = item.get("is_header", False)

        if is_header:
            current_header = label
            group_stack = [(d, l) for d, l in group_stack if d < depth]
            continue

        # Ajustar el stack al depth actual
        group_stack = [(d, l) for d, l in group_stack if d < depth]

        if is_group:
            group_stack.append((depth, label))
            continue

        if not href:
            continue

        base = [root_section]
        if current_header:
            base.append(current_header)

        path = base + [l for _, l in group_stack] + [label]
        url_map[norm_url("https://www.palantir.com" + href)] = path

    return url_map


# ─────────────────────────────────────────────────────────
# Extracción del sidebar desde una URL real
# ─────────────────────────────────────────────────────────
async def scrape_sidebar(browser, entry_url: str, root_section: str) -> Dict[str, List[str]]:
    from bs4 import BeautifulSoup

    log.info("[sidebar] %s", root_section)
    ctx = await browser.new_context(user_agent=UA, locale="en-US",
                                    viewport={"width": 1600, "height": 1200})
    page = await ctx.new_page()
    try:
        await page.goto(entry_url, wait_until="networkidle", timeout=40_000)
        try:
            await page.wait_for_selector("aside, nav", timeout=10_000)
        except Exception:
            pass

        # Expandir todos los paneles colapsados
        for _ in range(20):
            clicked = 0
            for sel in ["aside button[aria-expanded='false']",
                        "nav button[aria-expanded='false']"]:
                locs = page.locator(sel)
                n = min(await locs.count(), 150)
                for i in range(n):
                    try:
                        await locs.nth(i).click(timeout=300)
                        await page.wait_for_timeout(40)
                        clicked += 1
                    except Exception:
                        pass
            if clicked == 0:
                break

        await page.wait_for_timeout(600)
        html = await page.content()
    finally:
        await ctx.close()

    soup = BeautifulSoup(html, "html.parser")

    # Encontrar el nav/aside con más links de docs
    best, best_n = None, 0
    for cand in soup.find_all(["aside", "nav"]):
        n = sum(1 for a in cand.find_all("a", href=True)
                if "/docs/foundry" in a.get("href", ""))
        if n > best_n:
            best_n, best = n, cand

    if not best:
        log.warning("  Sin sidebar en %s", entry_url)
        return {}

    # Encontrar el UL principal (el de mayor número de links)
    main_ul, best_ul_n = None, 0
    for ul in best.find_all("ul"):
        n = sum(1 for a in ul.find_all("a", href=True)
                if "/docs/foundry" in a.get("href", ""))
        if n > best_ul_n:
            best_ul_n, main_ul = n, ul

    if not main_ul:
        log.warning("  Sin UL principal en %s", entry_url)
        return {}

    tree = parse_sidebar(main_ul, depth=0)
    url_map = build_url_map(tree, root_section)
    log.info("  → %s URLs para '%s'", len(url_map), root_section)
    return url_map


# ─────────────────────────────────────────────────────────
# Estado persistente del crawl
# ─────────────────────────────────────────────────────────
class State:
    def __init__(self, output_dir: Path):
        self._f = output_dir / ".scraper_state.json"
        self.visited:   Set[str]             = set()
        self.queue:     List[str]            = []
        self.url_map:   Dict[str, List[str]] = {}
        self.sidebars_done: Set[str]         = set()
        self._load()

    def _load(self):
        if not self._f.exists():
            return
        try:
            d = json.loads(self._f.read_text("utf-8"))
            self.visited       = set(d.get("visited", []))
            self.queue         = d.get("queue", [])
            self.url_map       = d.get("url_map", {})
            self.sidebars_done = set(d.get("sidebars_done", []))
            log.info("Estado: %s visitadas, %s cola, %s rutas, %s sidebars.",
                     len(self.visited), len(self.queue),
                     len(self.url_map), len(self.sidebars_done))
        except Exception as e:
            log.warning("Estado no legible: %s", e)

    def save(self):
        self._f.parent.mkdir(parents=True, exist_ok=True)
        self._f.write_text(json.dumps({
            "visited":       sorted(self.visited),
            "queue":         self.queue,
            "url_map":       self.url_map,
            "sidebars_done": sorted(self.sidebars_done),
        }, ensure_ascii=False, indent=2), "utf-8")

    def add_urls(self, urls):
        for u in urls:
            c = norm_url(u)
            if c and is_docs_url(c) and c not in self.visited and c not in self.queue:
                self.queue.append(c)

    def merge(self, new_map: Dict[str, List[str]]):
        for url, path in new_map.items():
            if url not in self.url_map:
                self.url_map[url] = path
        self.add_urls(list(new_map.keys()))

    def get_path(self, url: str) -> Optional[List[str]]:
        return self.url_map.get(norm_url(url))


# ─────────────────────────────────────────────────────────
# Descarga de imágenes
# ─────────────────────────────────────────────────────────
def make_session():
    try:
        import requests
    except ImportError:
        log.error("pip install requests")
        sys.exit(1)
    s = requests.Session()
    s.headers.update(HEADERS)
    return s

def _ext(url: str, ct: str = "") -> str:
    e = Path(urlparse(url).path).suffix.lower()
    if e in {".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp", ".bmp", ".avif"}:
        return ".jpg" if e == ".jpeg" else e
    g = mimetypes.guess_extension((ct.split(";")[0]).strip())
    return (".jpg" if g == ".jpe" else g) if g else ".bin"

def _src(img) -> str:
    for k in ("src", "data-src", "data-original", "data-lazy-src"):
        v = (img.get(k) or "").strip()
        if v:
            return v
    ss = img.get("srcset") or img.get("data-srcset", "")
    if ss:
        first = ss.split(",")[0].strip().split(" ")[0].strip()
        if first:
            return first
    return ""

def _is_icon(img) -> bool:
    cls = " ".join(img.get("class", [])).lower()
    src = (img.get("src") or "").lower()
    return "icon" in cls or "favicon" in src or "logo" in src

def download_images(session, main, page_url: str, md_path: Path, max_n: int) -> int:
    seen: Set[str] = set()
    n = 0
    asset_dir = md_path.parent / f"{md_path.stem}_assets"
    for img in main.find_all("img"):
        if n >= max_n:
            break
        if _is_icon(img):
            continue
        raw = _src(img)
        if not raw or raw.startswith("data:"):
            continue
        abs_url = urljoin(page_url, raw)
        if abs_url in seen:
            continue
        seen.add(abs_url)
        try:
            r = session.get(abs_url, timeout=30, stream=True)
            r.raise_for_status()
            ext = _ext(abs_url, r.headers.get("Content-Type", ""))
            fp = asset_dir / f"img_{n+1:03d}{ext}"
            asset_dir.mkdir(parents=True, exist_ok=True)
            with open(fp, "wb") as fh:
                for chunk in r.iter_content(8192):
                    if chunk:
                        fh.write(chunk)
            img["src"] = posix(fp.relative_to(md_path.parent))
            for attr in ("srcset", "data-src", "data-srcset", "data-original", "data-lazy-src"):
                img.attrs.pop(attr, None)
            n += 1
        except Exception:
            pass
    return n


# ─────────────────────────────────────────────────────────
# Extracción de contenido → Markdown
# ─────────────────────────────────────────────────────────
def html_to_md(html: str, url: str, md_path: Path,
               session, max_imgs: int, shot_rel: Optional[str]) -> dict:
    from bs4 import BeautifulSoup
    import markdownify as mdlib

    soup = BeautifulSoup(html, "html.parser")

    # Título
    title = ""
    og = soup.find("meta", property="og:title")
    if og and og.get("content"):
        title = clean(og["content"])
    if not title:
        h1 = soup.find("h1")
        title = clean(h1.get_text(" ", strip=True)) if h1 else urlparse(url).path.rstrip("/").split("/")[-1]

    # Breadcrumb
    bc = [clean(a.get_text(" ", strip=True))
          for a in soup.select("nav[aria-label*='breadcrumb'] a, .breadcrumb a")
          if clean(a.get_text(" ", strip=True))]

    # Contenido principal
    main = (soup.find("main") or soup.find("article")
            or soup.find("div", {"id": "content"}) or soup.body)
    body = ""
    if main:
        for t in main.select(
            "nav, header, footer, script, style, noscript, "
            ".sidebar, .toc, .navigation, [aria-hidden='true'], "
            ".cookie-banner, .feedback, .edit-page, .page-nav"
        ):
            t.decompose()
        n_imgs = download_images(session, main, url, md_path, max_imgs)
        if n_imgs:
            log.info("    imgs: %s", n_imgs)
        body = mdlib.markdownify(str(main), heading_style="ATX",
                                 bullets="-", newline_style="backslash")

    body = re.sub(r"\n{3,}", "\n\n", body).strip()
    if shot_rel:
        body = f"## Captura de pantalla\n\n![Screenshot]({shot_rel})\n\n---\n\n" + body

    return {"title": title, "breadcrumb": bc, "body": body}


def save_md(path: Path, title: str, url: str, bc: List[str], body: str):
    path.parent.mkdir(parents=True, exist_ok=True)
    ts = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    safe_title = title.replace('"', '\\"')
    fm = ["---", f'title: "{safe_title}"', f'source: "{url}"', f'scraped_at: "{ts}"']
    if bc:
        fm.append(f'breadcrumb: "{" > ".join(bc)}"')
    fm.append("---")
    path.write_text("\n".join(fm) + f"\n\n# {title}\n\n{body}\n", "utf-8")
    log.info("  ✓ %s", path)


def url_to_path(components: List[str], output_dir: Path) -> Path:
    parts = [sanitize(c) for c in components]
    return output_dir.joinpath(*parts[:-1]) / f"{parts[-1]}.md"


# ─────────────────────────────────────────────────────────
# Worker paralelo: procesa una URL
# ─────────────────────────────────────────────────────────
async def process_url(
    url: str,
    path_components: List[str],
    browser,
    session,
    output_dir: Path,
    screenshots: bool,
    max_imgs: int,
    semaphore: asyncio.Semaphore,
) -> bool:
    async with semaphore:
        ctx = await browser.new_context(user_agent=UA, locale="en-US",
                                        viewport={"width": 1600, "height": 1200})
        page = await ctx.new_page()
        page.set_default_timeout(30_000)
        try:
            await page.goto(url, wait_until="networkidle", timeout=30_000)
            try:
                await page.wait_for_selector("main, article, h1", timeout=8_000)
            except Exception:
                pass
            # Scroll para lazy-load
            last = 0
            for _ in range(6):
                await page.evaluate("window.scrollTo(0, document.body.scrollHeight)")
                await page.wait_for_timeout(400)
                h = await page.evaluate("document.body.scrollHeight")
                if h == last:
                    break
                last = h
            await page.evaluate("window.scrollTo(0, 0)")
            html = await page.content()

            md_path = url_to_path(path_components, output_dir)

            shot_rel = None
            if screenshots:
                try:
                    shot = md_path.parent / f"{md_path.stem}.screenshot.png"
                    shot.parent.mkdir(parents=True, exist_ok=True)
                    await page.screenshot(path=str(shot), full_page=True, animations="disabled")
                    shot_rel = shot.name
                except Exception as e:
                    log.warning("  screenshot error: %s", e)

            data = html_to_md(html, url, md_path, session, max_imgs, shot_rel)
            save_md(md_path, data["title"], url, data["breadcrumb"], data["body"])
            return True

        except Exception as e:
            log.warning("  ERROR %s: %s", url, e)
            return False
        finally:
            await ctx.close()


# ─────────────────────────────────────────────────────────
# Crawl principal con paralelización
# ─────────────────────────────────────────────────────────
async def crawl(output_dir: Path, workers: int, delay: float,
                max_pages: int, max_imgs: int, screenshots: bool):
    try:
        from playwright.async_api import async_playwright
    except ImportError:
        log.error("pip install playwright && playwright install chromium")
        sys.exit(1)

    session = make_session()
    state   = State(output_dir)

    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)

        # ── FASE 1: Extraer sidebars (secuencial, 1 pestaña cada vez) ──
        log.info("=" * 60)
        log.info("FASE 1 — Extrayendo sidebars (%s secciones)", len(SECTIONS))
        log.info("=" * 60)
        for section, entry in SECTIONS.items():
            if section in state.sidebars_done:
                log.info("  [ya extraído] %s", section)
                continue
            url_map = await scrape_sidebar(browser, entry, section)
            state.merge(url_map)
            state.sidebars_done.add(section)
            state.save()
            await asyncio.sleep(1.0)

        log.info("URLs totales en mapa semántico: %s", len(state.url_map))

        # ── FASE 2: Crawl paralelo del contenido ──
        log.info("=" * 60)
        log.info("FASE 2 — Descarga paralela (workers=%s)", workers)
        log.info("=" * 60)

        semaphore = asyncio.Semaphore(workers)
        counter   = {"done": 0, "total": len(state.url_map)}

        # Construir lista de tareas pendientes
        pending = []
        for url, path in state.url_map.items():
            if norm_url(url) not in state.visited:
                pending.append((norm_url(url), path))

        # Si hay URLs en la cola que no están en el mapa semántico, intentar inferir
        for url in state.queue:
            u = norm_url(url)
            if u not in state.visited and u not in {p for p, _ in pending}:
                # Intentar asignar sección por prefijo de path
                parts = [x for x in urlparse(u).path.replace(DOCS_PREFIX + "/", "").split("/") if x]
                if not parts:
                    continue
                section = None
                for sec, entry in SECTIONS.items():
                    ep = [x for x in urlparse(entry).path.replace(DOCS_PREFIX + "/", "").split("/") if x]
                    if ep and parts[0] == ep[0]:
                        section = sec
                        break
                if section:
                    leaf = parts[-1].replace("-", " ").title()
                    pending.append((u, [section, leaf]))

        if max_pages:
            pending = pending[:max_pages]

        log.info("Páginas a descargar: %s", len(pending))

        async def worker_task(url: str, path_comp: List[str]):
            ok = await process_url(url, path_comp, browser, session,
                                   output_dir, screenshots, max_imgs, semaphore)
            state.visited.add(url)
            counter["done"] += 1
            log.info("[%s/%s] %s", counter["done"], len(pending),
                     "✓" if ok else "✗ ERROR")
            if delay > 0:
                await asyncio.sleep(delay)
            if counter["done"] % 10 == 0:
                state.save()

        tasks = [asyncio.create_task(worker_task(u, p)) for u, p in pending]
        await asyncio.gather(*tasks)

        await browser.close()

    state.save()
    log.info("Crawl completado: %s páginas.", len(state.visited))


# ─────────────────────────────────────────────────────────
# Índice README
# ─────────────────────────────────────────────────────────
def build_index(output_dir: Path):
    files = sorted(f for f in output_dir.rglob("*.md") if f.name != "README.md")
    lines = [
        "# Palantir Foundry — Documentación técnica",
        "",
        f"> Generado: {time.strftime('%Y-%m-%d')}",
        f"> Fuente: {BASE_URL}",
        "",
    ]
    cur_dir = None
    for f in files:
        rel    = f.relative_to(output_dir)
        folder = str(rel.parent).replace("\\", "/")
        if folder != cur_dir:
            cur_dir = folder
            depth   = folder.count("/")
            lines.append(f"\n{'#' * min(depth + 3, 6)} {folder}\n")
        title = f.stem
        try:
            for line in f.read_text("utf-8").splitlines():
                if line.startswith("title:"):
                    title = line.replace("title:", "", 1).strip().strip('"')
                    break
        except Exception:
            pass
        lines.append(f"- [{title}]({posix(rel)})")
    readme = output_dir / "README.md"
    readme.write_text("\n".join(lines) + "\n", "utf-8")
    log.info("README generado: %s archivos", len(files))


# ─────────────────────────────────────────────────────────
# CLI
# ─────────────────────────────────────────────────────────
def parse_args():
    ap = argparse.ArgumentParser(
        description="Foundry docs scraper v3",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Ejemplos:
  python foundry_docs_scraper.py --output ./foundry-docs
  python foundry_docs_scraper.py --output ./foundry-docs --workers 6 --delay 0.5
  python foundry_docs_scraper.py --output ./foundry-docs --no-screenshots --workers 8
  python foundry_docs_scraper.py --output ./foundry-docs --max-pages 30
  python foundry_docs_scraper.py --output ./foundry-docs --index-only
        """
    )
    ap.add_argument("--output",    default="./foundry-docs", help="Carpeta de salida")
    ap.add_argument("--workers",   type=int, default=4,      help="Pestañas paralelas (default: 4)")
    ap.add_argument("--delay",     type=float, default=1.0,  help="Segundos entre requests por worker (default: 1.0)")
    ap.add_argument("--max-pages", type=int, default=0,      help="Límite de páginas (0=sin límite)")
    ap.add_argument("--max-images-per-page", type=int, default=6, help="Máx imágenes por página")
    ap.add_argument("--no-screenshots", action="store_true", help="Sin capturas de pantalla")
    ap.add_argument("--index-only",     action="store_true", help="Solo regenerar README.md")
    return ap.parse_args()


def main():
    args = parse_args()
    out  = Path(args.output).expanduser().resolve()
    out.mkdir(parents=True, exist_ok=True)

    if not args.index_only:
        asyncio.run(crawl(
            output_dir  = out,
            workers     = args.workers,
            delay       = args.delay,
            max_pages   = args.max_pages,
            max_imgs    = args.max_images_per_page,
            screenshots = not args.no_screenshots,
        ))

    build_index(out)
    log.info("Listo → %s", out)


if __name__ == "__main__":
    main()

# Documentation Website

The OpenFoundry documentation website is a self-contained VitePress site under `docs/`, modeled after the structure used in `OxiCloud/docs` and adapted to this repository.

## Local Development

Install documentation dependencies:

```bash
cd docs
npm ci
```

Run the site locally:

```bash
npm run docs:dev
```

Build the static site:

```bash
npm run docs:build
```

Preview the built output:

```bash
npm run docs:preview
```

## Site Layout

- `docs/.vitepress/config.mts`: VitePress site configuration
- `docs/public/`: static assets such as the documentation logo
- `docs/guide/`: developer and contributor flows
- `docs/architecture/`: system structure and runtime topology
- `docs/operations/`: deployment and CI/CD
- `docs/reference/`: repository and contract reference

## Publication Model

The documentation site is published to GitHub Pages through `.github/workflows/deploy-docs.yml`.

The deployment workflow:

- triggers on pushes to `main` that touch `docs/**`
- builds the VitePress site inside `docs/`
- uploads `docs/.vitepress/dist`
- deploys the artifact with GitHub Pages

The configured base path is:

```text
/OpenFoundry/
```

That matches the expected repository-site URL shape for `https://diocrafts.github.io/OpenFoundry/`.

## Updating the Docs

The expected authoring flow is:

1. Edit or add Markdown pages in `docs/`
2. Run `npm run docs:build` locally
3. Commit and push to `main`
4. Let the Pages workflow rebuild and publish the website

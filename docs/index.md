---
layout: home

hero:
  name: OpenFoundry
  text: Technical Documentation
  tagline: Architecture, repository layout, delivery pipelines, and operating model for the OpenFoundry monorepo.
  image:
    src: /logo.svg
    alt: OpenFoundry
  actions:
    - theme: brand
      text: Start Here
      link: /guide/
    - theme: alt
      text: Architecture
      link: /architecture/
    - theme: alt
      text: CI/CD
      link: /operations/ci-cd

features:
  - title: Monorepo Map
    details: Understand where the frontend, Rust services, shared crates, protobuf contracts, SDKs, infra, smoke tests, and benchmarks live.
  - title: Executable Architecture
    details: Use the smoke suites, generated contracts, and workspace automation as the source of truth for how the platform fits together.
  - title: Operations Ready
    details: Follow documented deployment modes, CI workflows, runbooks, and GitHub Pages automation for ongoing maintenance.
---

## What This Site Covers

This documentation is focused on the repository itself rather than on end-user product usage.

It answers four practical questions:

- How is the monorepo organized?
- How do the services and contracts fit together?
- How do contributors run, test, and validate changes?
- How does the project publish artifacts such as docs, SDKs, and release outputs?

## Recommended Reading Order

1. [Guide](/guide/) for contributor orientation.
2. [Repository Map](/guide/repository-map) for folder-level navigation.
3. [Architecture Overview](/architecture/) for runtime and contract boundaries.
4. [CI/CD](/operations/ci-cd) for delivery and release automation.

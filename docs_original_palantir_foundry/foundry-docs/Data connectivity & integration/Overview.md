---
title: "Palantir"
source: "https://www.palantir.com/docs/foundry/data-integration/overview/"
scraped_at: "2026-04-26T16:19:29Z"
---

# Palantir

## Captura de pantalla

![Screenshot](Overview.screenshot.png)

---

[Data connectivity & integration](/docs/foundry/data-integration/overview/)[Overview](/docs/foundry/data-integration/overview/)

![data integration overview](Overview_assets/img_001.svg)

# Data connectivity and integration

Foundry provides a highly configurable set of data connectivity and integration tools that extend far beyond typical extract-transform-load (ETL) or extract-load-transform (ELT) solutions. Foundry is designed to reduce the cost of data integration over time through a rich suite of capabilities that act as a force multiplier for data teams. While commodity cloud services provide storage and compute for basic pipelines and experimentation, many additional layers of capability are required to manage, deliver, and validate datasets for critical operations. Foundry is designed to serve as the data integration backbone for the most complex environments in the world.

## Connecting data

This begins with an extensible data connection framework that establishes connections with all types of source systems - structured, unstructured, or semi-structured – and with all key data transfer approaches, such as batch, micro-batch, or streaming. This functionality is integrated with the platform’s data transformation and data management functionality, which includes full lineage of data versions, granular security for collaborative management of data extraction, and branching of data sync configurations.

[Learn more about connecting to data in Foundry.](/docs/foundry/data-integration/connecting-to-data/)

## Data transformation

For **data transformation**, Foundry provides an extensible, scalable *build system for data* which leverages multimodal compute to produce output datasets. Foundry’s compute-agnostic “Build” framework provides fully integrated security and data lineage, and enables mixing-and-matching of third-party compute runtimes. Foundry also includes an integrated suite of data transformation authoring, change management, data quality, pipeline scheduling, and metadata introspection capabilities that work cohesively to provide a "mission control" for data engineers.

[Learn more about transforming data with Foundry.](/docs/foundry/data-integration/data-pipeline/)

## Pipeline management

Foundry’s **pipeline management** capabilities combine change management, data quality, and data loading features.

The Pipeline Builder application enables fast, flexible, and scalable delivery of data pipelines while providing robustness and security. [Learn more about Pipeline Builder.](/docs/foundry/pipeline-builder/overview/)

Data engineers can define a rigorous release process for production pipelines, including health checks that guarantee only fully compliant data will be deployed to production. Where issues are found, the platform provides diagnostics on the discrepancies detected.

Diagnostics are available both in Foundry's integrated analysis and modeling tools, as well as to any third-party tools accessing the outputs via REST APIs or other interfaces.

[Learn more about maintaining and managing pipelines in Foundry.](/docs/foundry/maintaining-pipelines/overview/)

[NEXTConnecting to data

→](/docs/foundry/data-integration/connecting-to-data/)

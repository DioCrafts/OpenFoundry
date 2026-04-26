---
title: "Palantir"
source: "https://www.palantir.com/docs/foundry/model-integration/overview/"
scraped_at: "2026-04-26T16:20:06Z"
---

# Palantir

## Captura de pantalla

![Screenshot](Overview.screenshot.png)

---

[Model connectivity & development](/docs/foundry/model-integration/overview/)[Overview](/docs/foundry/model-integration/overview/)

![model integration overview](Overview_assets/img_001.svg)

# Model connectivity and development

In the Palantir platform, **models** are artifacts that encapsulate any machine learning logic. Models can be leveraged across workflows in data pipelines, Ontology, and application layers to enable a wide variety of different use cases.

Palantir takes an expansive approach to enabling model integration. Models can either be developed within the platform on integrated data, or externally and imported as libraries, artifacts, containers, or as APIs.

Once integrated, models can be used with platform tools for inference, deployment, governance, ML Ops, and operationalization. Every step in the model creation and consumption process is subject to platform guarantees around lineage, security, versioning, reproducibility, and auditing.

## Sources

Models of any form can be developed in-platform (for example, via open-source tools such as scikit-learn, TensorFlow, and OR-tools, or via custom libraries), imported from an external environment (such as notebooks, third-party data science products, or container registries), or configured as externally hosted APIs.

Learn more about [model development within the platform](/docs/foundry/integrate-models/model-adapter-overview/) and [integration of external models](/docs/foundry/integrate-models/external-model-connection/).

## Modeling Objectives

**Modeling Objectives** serve as the "mission control" for streamlining model management, evaluation, review, release, and deployment for a defined problem. Beyond a convenient user interface, Modeling Objectives provide a governance and permissions layer, an automation layer (e.g. uniform evaluation of model candidates), and a CI/CD layer for continuous and downtime-free deployment for models.

Objectives enable the complete **model lifecycle** for any modeling problem, including those not traditionally addressed by ML Ops tools, such as simulation and optimization.

Using Objectives, organizations can go beyond simply creating a data pipeline to safely deploy models in an operational capacity for users and systems that make decisions. Once operationalized, Foundry enables ML feedback loops from production data, outcomes, applications, and user actions. These feedback loops provide modeling teams with a powerful data asset for monitoring, understanding, and improving production performance, as well as adapting to new circumstances.

Learn more about [Modeling Objectives](/docs/foundry/model-integration/objectives/).

## Models in the Ontology

The ontology is the operational layer of the Palantir platform. The Ontology connects digital assets to their real-world counterparts to enable all types of different use cases.

Once a model has been integrated into the platform, the model can be deployed using Modeling Objectives and registered for use within the ontology layer. This allows operational interactive workflows to be backed by high-trust models to power, quick, critical decision-making.

[Learn more about the Ontology](/docs/foundry/ontology/overview/) and how to [integrate models with it](/docs/foundry/functions/functions-on-models/).

[NEXTGetting started

→](/docs/foundry/model-integration/getting-started/)

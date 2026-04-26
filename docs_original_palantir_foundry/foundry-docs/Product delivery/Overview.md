---
title: "Palantir"
source: "https://www.palantir.com/docs/foundry/app-building/overview/"
scraped_at: "2026-04-26T16:19:15Z"
---

# Palantir

## Captura de pantalla

![Screenshot](Overview.screenshot.png)

---

[Use case development](/docs/foundry/app-building/overview/)[Overview](/docs/foundry/app-building/overview/)

![Application building header image.](Overview_assets/img_001.svg)

# Use case development

The Palantir platform was designed to empower a diverse community of builders with a collection of powerful tools for use case development, including application building tools, workflow building tools, integrated [analytics tools](/docs/foundry/app-building/analytics-operations/), and [developer tools](/docs/foundry/dev-toolchain/overview/). Each of these leverages the power of Foundry’s core security, lineage, data, and compute primitives, allowing teams to focus on delivering operational capability rather than managing infrastructure. Critically, each tool in the Palantir platform is designed to continuously, safely enrich a consistent set of data and model assets (contained within the [Ontology](/docs/foundry/ontology/overview/)). This enables knowledge to compound as operational workflows are scaled out across the enterprise.

Palantir Learning portal

Understand scoping use cases for Foundry and AIP at [learn.palantir.com ↗](https://learn.palantir.com/scoping-use-cases-for-foundry-aip).

## Application building

The primary application building tools in the Palantir platform are [Workshop](#workshop) and [Slate](#slate).

Beyond these built-in tools, it is possible to create custom, bespoke applications on top of the Palantir platform using the [developer toolchain and Ontology SDK (OSDK)](#developer-toolchain).

### Workshop

**Workshop** is a flexible, object-oriented application building tool. Workshop leverages the semantic primitives (e.g., objects, links) and the kinetic primitives (e.g., Actions, Functions) within the [Ontology](/docs/foundry/ontology/overview/) to enable the rapid delivery of highly interactive desktop and mobile applications. The application building experience in Workshop empowers users to create powerful applications out of no-code, low-code, and code-based widgets. No technical expertise is required to start building with widgets and weaving objects, links, and actions into user-driven workflows that go far beyond dashboards or passive visualizations. Meanwhile, code-based enrichment with [Functions](/docs/foundry/functions/overview/) can be seamlessly embedded within Workshop widgets to allow for complex interactions, cascading processes, and complex data capture.

[Learn more about Workshop.](/docs/foundry/workshop/overview/)

### Slate

**Slate** provides builders with a flexible set of tools to quickly create operational applications and interactive dashboards. Slate enables application developers to construct dynamic and responsive applications with a drag-and-drop interface, reducing development time and cost. Slate includes capabilities that are seamlessly integrated with the Foundry Ontology, but also enables developers to fully customize applications using HTML, CSS, and JavaScript. With custom Slate applications, stakeholders at all levels of an organization can rapidly explore and understand their data in order to make better-informed decisions.

[Learn more about Slate.](/docs/foundry/slate/overview/)

## Workflow building and management

The primary workflow building and management tools in the Palantir platform are [Workflow Lineage](#workflow-lineage), [Automate](#automate), [Solution Designer](#solution-designer), and [Use Cases](#use-cases).

### Workflow Lineage

**Workflow Lineage** provides an interactive workspace for understanding and managing applications and their underlying processes. With Workflow Lineage, you can explore workflows and view details on objects, actions, functions, large language models, and applications. Workflow Lineage is particularly useful for application builders that are creating, debugging or maintaining workflows. The graph of provenance, deeper property and workshop widget/variable provenance, and upgrade tooling are all helpful when making changes to or extending a workflow.

### Automate

**Automate** gives you a single entry point for setting up and executing all business automation in the platform. The Automate application allows users to define conditions and effects; conditions are checked continuously, and effects are executed automatically when the specified conditions were met.

[Learn more about Automate.](/docs/foundry/automate/overview/)

### Carbon

**Carbon** enables the configuration of tailored platform experiences, known as workspaces, for specific user groups. Carbon can provide a focused experience for less technical users that need to carry out critical operational workflows. Each Carbon workspace is a curated collection of applications and resources that can be configured to optimize a given set of operational, end-user workflows. For example, an aircraft parts maintenance workspace might consist of a Workshop application containing a dynamically updated list of parts requiring maintenance, along with Ontology-driven Actions for triaging each part; another application that is used to investigate each part's maintenance issue; and a Quiver analysis showing maintenance trends over time. Carbon allows the rich tapestry of Foundry applications and analytical capabilities to be integrated into focused, operational experiences.

[Learn more about Carbon.](/docs/foundry/carbon/overview/)

### Solution Designer

**Solution Designer** is an interactive tool for creating architectural representations of solutions built using the Palantir platform, including representations for first and third-party integration points, links to platform resources, on-demand access to documentation and best practices, and more.

[Learn more about Solution Designer.](/docs/foundry/solution-designer/overview/)

### Use Cases

The Use Cases application allows builders to organize their work within a single operational interface. By combining the file system view with an ontology management view, developers can access a curated view focused on the work for which they are responsible.

[Learn more about the Use Cases application.](/docs/foundry/use-cases/use-case-overview/)

## Developer toolchain

The Palantir [developer toolchain](/docs/foundry/dev-toolchain/overview/) enables you to build your own applications on top of the Palantir platform using your own tools.

The centerpiece of the Palantir developer toolchain is the [Ontology SDK (OSDK)](/docs/foundry/ontology-sdk/overview/). You can generate an Ontology-specific SDK with the Developer Console, available in the Palantir platform. The Ontology SDK is created either as a NPM (Node Package Manager) package for TypeScript or as Pip or Conda for Python, and it only contains a pre-selected subset of your Ontology. The SDK lets you access object types, apply actions to update data in the Ontology, call functions, and run AIP Logic functions for [AIP-enabled](/docs/foundry/aip/enable-aip-features/) enrollments. The Developer Console also includes Ontology-specific documentation for the entities chosen for your application. Applications use the OAuth flow as a public or confidential client to access the data.

[Learn more about Ontology SDK.](/docs/foundry/ontology-sdk/overview/)

[NEXTWhat is an operational application?

→](/docs/foundry/app-building/operational-apps/)

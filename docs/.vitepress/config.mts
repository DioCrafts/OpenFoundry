import { defineConfig } from "vitepress";

export default defineConfig({
  title: "OpenFoundry",
  description: "Technical documentation for the OpenFoundry monorepo",

  base: "/OpenFoundry/",

  sitemap: {
    hostname: "https://diocrafts.github.io/OpenFoundry",
    lastmodDateOnly: false,
  },

  markdown: {
    image: {
      lazyLoading: true,
    },
  },

  lastUpdated: true,

  ignoreDeadLinks: [
    /^https?:\/\/localhost/,
  ],

  locales: {
    root: {
      label: "English",
      lang: "en",
    },
  },

  head: [
    ["link", { rel: "icon", href: "/OpenFoundry/logo.png" }],
  ],

  themeConfig: {
    logo: "/logo.png",

    search: {
      provider: "local",
    },

    editLink: {
      pattern: "https://github.com/DioCrafts/OpenFoundry/tree/main/docs/:path",
      text: "Edit this page on GitHub",
    },

    nav: [
      { text: "Home", link: "/" },
      {
        text: "Capabilities",
        items: [
          { text: "AI Platform (AIP)", link: "/ai-platform/" },
          { text: "Data connectivity & integration", link: "/data-connectivity/" },
          { text: "Model connectivity & development", link: "/model-connectivity/" },
          { text: "Ontology building", link: "/ontology-building/" },
          { text: "Developer toolchain", link: "/developer-toolchain/" },
          { text: "Use case development", link: "/use-case-development/" },
          { text: "Observability", link: "/observability/" },
          { text: "Analytics", link: "/analytics/" },
          { text: "Product delivery", link: "/product-delivery/" },
          { text: "Security & governance", link: "/security-governance/" },
          { text: "Management & enablement", link: "/management-enablement/" },
        ],
      },
      { text: "Getting started", link: "/getting-started/" },
      { text: "Architecture center", link: "/architecture-center/" },
      { text: "Platform updates", link: "/platform-updates/" },
    ],

    sidebar: {
      "/": [
        {
          text: "Capabilities",
          items: [
            { text: "AI Platform (AIP)", link: "/ai-platform/" },
            { text: "Data connectivity & integration", link: "/data-connectivity/" },
            { text: "Model connectivity & development", link: "/model-connectivity/" },
            { text: "Ontology building", link: "/ontology-building/" },
            { text: "Developer toolchain", link: "/developer-toolchain/" },
            { text: "Use case development", link: "/use-case-development/" },
            { text: "Observability", link: "/observability/" },
            { text: "Analytics", link: "/analytics/" },
            { text: "Product delivery", link: "/product-delivery/" },
            { text: "Security & governance", link: "/security-governance/" },
            { text: "Management & enablement", link: "/management-enablement/" },
          ],
        },
        {
          text: "Ontology building",
          items: [
            { text: "Overview", link: "/ontology-building/" },
            { text: "Why create an Ontology?", link: "/ontology-building/why-create-an-ontology" },
            { text: "Core concepts", link: "/ontology-building/core-concepts" },
            { text: "Ontology-aware applications", link: "/ontology-building/ontology-aware-applications" },
            { text: "Define ontologies", link: "/ontology-building/define-ontologies" },
            { text: "Object and link types", link: "/ontology-building/object-and-link-types" },
            { text: "Object types", link: "/ontology-building/object-types/" },
            { text: "Properties", link: "/ontology-building/properties/" },
            { text: "Interfaces", link: "/ontology-building/interfaces/" },
            { text: "Interfaces and shared properties", link: "/ontology-building/interfaces-and-shared-properties" },
            { text: "Action types", link: "/ontology-building/action-types" },
            { text: "Functions", link: "/ontology-building/functions" },
            { text: "Functions by runtime", link: "/ontology-building/functions-runtime/" },
            { text: "Rules and simulation", link: "/ontology-building/rules-and-simulation" },
            { text: "Object sets and search", link: "/ontology-building/object-sets-and-search" },
            { text: "Indexing and materialization", link: "/ontology-building/indexing-and-materialization" },
            { text: "Object edits and conflict resolution", link: "/ontology-building/object-edits-and-conflict-resolution" },
            { text: "Object permissioning", link: "/ontology-building/object-permissioning" },
            { text: "Applications", link: "/ontology-building/applications" },
            { text: "Applications catalog", link: "/ontology-building/applications-catalog/" },
            { text: "Ontology architecture", link: "/ontology-building/ontology-architecture/" },
            { text: "Ontology Manager", link: "/ontology-building/ontology-manager" },
            { text: "Semantic search", link: "/ontology-building/semantic-search" },
          ],
        },
        {
          text: "Developer toolchain",
          items: [
            { text: "Overview", link: "/developer-toolchain/" },
            { text: "Local workflows", link: "/developer-toolchain/local-workflows" },
            { text: "Contracts and SDK generation", link: "/developer-toolchain/contracts-and-sdk-generation" },
            { text: "CI and quality gates", link: "/developer-toolchain/ci-and-quality-gates" },
            { text: "Code repositories and platform scaffolding", link: "/developer-toolchain/code-repositories-and-platform-scaffolding" },
            { text: "Code repositories", link: "/developer-toolchain/code-repositories/" },
            { text: "Project init", link: "/developer-toolchain/project-init/" },
            { text: "Plugin SDK", link: "/developer-toolchain/plugin-sdk/" },
            { text: "Marketplace and packaging", link: "/developer-toolchain/marketplace-and-packaging/" },
          ],
        },
        {
          text: "Use case development",
          items: [
            { text: "Overview", link: "/use-case-development/" },
            { text: "Application builder", link: "/use-case-development/application-builder" },
            { text: "Workflow composition", link: "/use-case-development/workflow-composition" },
            { text: "Operational experiences", link: "/use-case-development/operational-experiences" },
            { text: "Object Explorer equivalent", link: "/use-case-development/object-explorer-equivalent/" },
            { text: "Workshop equivalent", link: "/use-case-development/workshop-equivalent/" },
            { text: "Maps, reports, and notebooks", link: "/use-case-development/maps-reports-notebooks/" },
          ],
        },
        {
          text: "Security & governance",
          items: [
            { text: "Overview", link: "/security-governance/" },
            { text: "Identity and access", link: "/security-governance/identity-and-access" },
            { text: "Policies and authorization", link: "/security-governance/policies-and-authorization" },
            { text: "Restricted views and data controls", link: "/security-governance/restricted-views-and-data-controls" },
            { text: "Audit and traceability", link: "/security-governance/audit-and-traceability" },
            { text: "ABAC and CBAC model", link: "/security-governance/abac-and-cbac-model/" },
            { text: "Policy evaluation flows", link: "/security-governance/policy-evaluation-flows/" },
            { text: "Audit model", link: "/security-governance/audit-model/" },
          ],
        },
        {
          text: "Getting started",
          items: [
            { text: "Overview", link: "/getting-started/" },
            { text: "Repository map", link: "/guide/repository-map" },
            { text: "Local development", link: "/guide/local-development" },
            { text: "Quality gates", link: "/guide/quality-gates" },
            { text: "Documentation website", link: "/guide/documentation-website" },
          ],
        },
        {
          text: "Architecture center",
          items: [
            { text: "Overview", link: "/architecture-center/" },
            { text: "Monorepo structure", link: "/architecture/monorepo" },
            { text: "Runtime topology", link: "/architecture/runtime-topology" },
            { text: "Services and ports", link: "/architecture/services-and-ports" },
            { text: "Contracts and SDKs", link: "/architecture/contracts-and-sdks" },
            { text: "Capability map", link: "/architecture/capability-map" },
          ],
        },
        {
          text: "Platform updates",
          items: [
            { text: "Overview", link: "/platform-updates/" },
            { text: "Release notes", link: "/platform-updates/release-notes" },
          ],
        },
      ],
    },

    socialLinks: [
      { icon: "github", link: "https://github.com/DioCrafts/OpenFoundry" },
    ],

    footer: {
      message: "Released under the Apache 2.0 License.",
      copyright: "Copyright © 2026 DioCrafts",
    },
  },
});

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
    ["link", { rel: "icon", href: "/OpenFoundry/logo.svg" }],
  ],

  themeConfig: {
    logo: "/logo.svg",

    search: {
      provider: "local",
    },

    editLink: {
      pattern: "https://github.com/DioCrafts/OpenFoundry/tree/main/docs/:path",
      text: "Edit this page on GitHub",
    },

    nav: [
      { text: "Home", link: "/" },
      { text: "Guide", link: "/guide/" },
      { text: "Architecture", link: "/architecture/" },
      { text: "Operations", link: "/operations/" },
      { text: "Reference", link: "/reference/" },
    ],

    sidebar: {
      "/": [
        {
          text: "Guide",
          items: [
            { text: "Start Here", link: "/guide/" },
            { text: "Repository Map", link: "/guide/repository-map" },
            { text: "Local Development", link: "/guide/local-development" },
            { text: "Quality Gates", link: "/guide/quality-gates" },
            { text: "Documentation Website", link: "/guide/documentation-website" },
          ],
        },
        {
          text: "Architecture",
          items: [
            { text: "Overview", link: "/architecture/" },
            { text: "Monorepo Structure", link: "/architecture/monorepo" },
            { text: "Runtime Topology", link: "/architecture/runtime-topology" },
            { text: "Services and Ports", link: "/architecture/services-and-ports" },
            { text: "Contracts and SDKs", link: "/architecture/contracts-and-sdks" },
            { text: "Capability Map", link: "/architecture/capability-map" },
          ],
        },
        {
          text: "Operations",
          items: [
            { text: "Operations Overview", link: "/operations/" },
            { text: "Deployment Model", link: "/operations/deployment" },
            { text: "Deployment Modes", link: "/operations/deployment-modes" },
            { text: "CI/CD", link: "/operations/ci-cd" },
            { text: "Runbooks and Recovery", link: "/operations/runbooks" },
          ],
        },
        {
          text: "Reference",
          items: [
            { text: "Reference Index", link: "/reference/" },
            { text: "Repository Layout", link: "/reference/repository-layout" },
            { text: "API, SDK, and MCP", link: "/reference/api-sdk-mcp" },
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

---
title: "Palantir"
source: "https://www.palantir.com/docs/foundry/administration/configure-cors/"
scraped_at: "2026-04-26T16:21:06Z"
---

# Palantir

## Captura de pantalla

![Screenshot](Configure Cors.screenshot.png)

---

[Management & enablement](/docs/foundry/administration/overview/)Enrollment settings[Configure CORS (cross-origin resource sharing)](/docs/foundry/administration/configure-cors/)

# Configure CORS (cross-origin resource sharing)

CORS (cross-origin resource sharing) is a security feature that enables web pages to access APIs hosted on a different origin. CORS works by allowing specific HTTP headers to be added to the request and response, which permits cross-origin requests to go through. This feature is essential for legitimate cross-origin requests and prevents malicious external websites from accessing sensitive information.

You can configure CORS policies for your Foundry enrollment to enable certain external origins to access your resources, supporting other workflows such as accessing [Foundry public APIs](/docs/foundry/api/general/overview/introduction/) and integrating [third party applications](/docs/foundry/platform-security-third-party/third-party-apps-overview/).

# Configure CORS policies

CORS policy configuration in Control Panel is a new feature that relies on proper network infrastructure. If the feature is not yet available in your enrollment, contact your Palantir representative for assistance.

To begin configuring policies, head to the **CORS** tab in Control Panel. This feature is available to users with the Information Security Officer or Enrollment Administrator role. These roles are granted by Enrollment Administrators, [in the **Enrollment permissions** tab of Control Panel](/docs/foundry/administration/enrollments-and-organizations-permissions/).

![CORS extension](Configure Cors_assets/img_001.png)

As seen above, you can specify the allowed origins for your enrollment's hosts to permit these origins to load resources when making HTTP requests to your hosts.

When adding an origin, note the following rules:

- Non-localhost origins are required to start with the "https://" protocol followed by a complete domain. However, for localhost, the "http://" protocol is also supported.
- Ports are allowed only for localhost origins. To specify a port, add a colon ':' at the end of the origin address followed by the port number.
- You can use an asterisk to serve as a wildcard to represent any subdomain (for example, https://\*.palantir.com).

Select **Save** once you have configured the CORS policies for your enrollment's host(s); your new policies should take effect within five minutes.

[←

PREVIOUSConfigure domains and certificates](/docs/foundry/administration/configure-domains-and-certificates/)

[NEXTInternal dataset export

→](/docs/foundry/administration/internal-dataset-export/)

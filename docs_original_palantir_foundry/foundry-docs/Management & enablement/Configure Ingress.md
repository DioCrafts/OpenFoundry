---
title: "Palantir"
source: "https://www.palantir.com/docs/foundry/administration/configure-ingress/"
scraped_at: "2026-04-26T16:21:34Z"
---

# Palantir

## Captura de pantalla

![Screenshot](Configure Ingress.screenshot.png)

---

[Management & enablement](/docs/foundry/administration/overview/)Enrollment settings[Configure ingress](/docs/foundry/administration/configure-ingress/)

# Configure network ingress

Network ingress refers to connections that are initiated from outside Foundry. When using our managed Software as a Service (SaaS) platform, Control Panel offers allowlist configuration to define from where such connections can be established.

Appropriate network ingress rules must be configured in Control Panel for users to log in and browse Foundry, and for processes that require reaching into Foundry, [such as setting up an agent for Data Connection](/docs/foundry/data-connection/set-up-agent/#configure-agent-network-access).

The ability to configure network ingress in Control Panel may not yet be available for your enrollment.

## Configure network ingress in Control Panel

The ability to configure network ingress allowlists is available in the **Network ingress** tab of Control Panel. This feature is available to users with the Information Security Officer or Enrollment Administrator role. These roles are granted by Enrollment Administrators, [in the **Enrollment permissions** tab of Control Panel](/docs/foundry/administration/enrollments-and-organizations-permissions/).

![Network ingress configuration](Configure Ingress_assets/img_001.png)

Two types of rules are supported:

- In the **Allowed IP address ranges** section: The ability to specify IPv4 ranges, in CIDR notation, from where ingress connections can be established. A maximum of 500 CIDRs blocks can be configured.
- In the **Allowed countries** section: The ability to specify countries from where ingress connections can be established.

Rules are additive; connections can be established if satisfying either an IP-based rule or a country rule.

A user connecting through a Virtual Private Network (VPN) is allowed based on the egress IP of the VPN.

### Considerations: Country-based allowlisting

When using country-based allowlisting rather than strict IP-based allowlisting, be sure to understand the convenience and security tradeoffs. While authentication is still required, using broad network allowlisting may greatly increase the risk for:

- Identity-based attacks
  - Examples: Authentication material spillage, account takeovers, brute force, and credential theft.
- Social engineering and web-based attacks
  - Examples: Man-in-the-middle attacks, DNS poisoning, and other targeted phishing.
- Exploitation of underlying infrastructure and applications
  - Example: Zero-day exploits.

Palantir recommends strict IP allowlisting as a defense-in-depth control intended to significantly reduce these risks by denying adversaries the network access required to take offensive action.

Country-based allowlisting works by geotagging the IPs of incoming connections. This behavior is subject to potential data quality issues from the third-party used by Palantir to drive this feature. False positives and false negatives may occur, which is expected for IP geotagging tools.

### Make a change request

Given the sensitive nature of configuring network ingress, all ingress changes must go through an [Approvals workflow](/docs/foundry/approvals/overview/). After finishing your modifications, select the **Request changes** option in the bottom-right corner of the page and provide a justification for the change. Separate approvals request will be made for each domain that has proposed changes to its ingress configuration. By default, administrators are able to approve their own ingress change requests. However, the approvals workflow ensures changes are reviewed before going into effect and provides a history of all modifications.

The following image shows an example of the dialog when requesting changes to the ingress configurations of two domains.

![Ingress requesting changes.](Configure Ingress_assets/img_002.png)

Requests appear in [Control Panel's Approvals inbox](/docs/foundry/administration/control-panel-approvals/#approvals-inbox).

IP addresses originating from certain locations may be automatically denied by Palantir. If this occurs for an IP address you want to allow, contact your Palantir representative.

### Advanced settings

Under **Advanced settings**, you can toggle **Palantir access** on or off; toggling **Palantir access** on enables ingress networking access from Palantir's corporate network without having to explicitly allow Palantir's corporate IPs.

![Advanced settings](Configure Ingress_assets/img_003.png)

**Palantir access** should usually be turned on if you are being supported by Palantir engineers who access your enrollment through a dedicated authentication provider and from the Palantir network. Note that Palantir access is via VPN and is not specific to one geographical region.

Similar to IP and country-level allowlisting, the **Palantir access** setting is additive: if your network ingress configuration allows connections from the United States, and this toggle is enabled, access will be possible from corporate network locations outside the United States.

[←

PREVIOUSConfigure egress certificates](/docs/foundry/administration/configure-egress-certificates/)

[NEXTConfigure VPN ingress

→](/docs/foundry/administration/configure-vpn-ingress/)

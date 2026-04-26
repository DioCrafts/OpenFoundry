---
title: "Palantir"
source: "https://www.palantir.com/docs/foundry/administration/configure-outbound-applications/"
scraped_at: "2026-04-26T16:21:44Z"
---

# Palantir

## Captura de pantalla

![Screenshot](Configure Outbound Applications.screenshot.png)

---

[Management & enablement](/docs/foundry/administration/overview/)Organization settings[Configure outbound applications](/docs/foundry/administration/configure-outbound-applications/)

# Configure outbound applications with OAuth 2.0 authorization code grants

Outbound applications provide a way for administrators to manage OAuth 2.0 connections to external systems from workflows built in Foundry. Outbound applications are managed at the Organization level in Control Panel.

An outbound application represents a bundle of configurations required for Foundry to behave as an OAuth 2.0 *client*. The application makes requests to another system that can behave as an OAuth 2.0 *server*.

Outbound applications in Foundry only support the [OAuth 2.0 authorization code grant ↗](https://datatracker.ietf.org/doc/html/rfc6749#section-4.1) type.
To set up an OAuth 2.0 client credentials grant, follow [these resources](/docs/foundry/data-connection/webhooks-reference/#client-credentials-grant) to manually establish the handshake process within a webhook.

Private network support

Outbound applications support connections to OAuth 2.0 servers on private networks using [agent proxy egress policies](/docs/foundry/administration/configure-egress/#agent-proxy-egress-policies). This allows Foundry to route traffic through a [Data Connection agent](/docs/foundry/data-connection/core-concepts/#agents) installed in your network. See the [egress policy configuration option documentation](#configuration-options) for details.

## Create an outbound application

To create an outbound application, first navigate to the **Outbound applications** in Control Panel, under **Organization settings**. If you have access to multiple Organizations, be sure to select the Organization in which you wish to create your outbound application. All users of this Organization with permission to set up sources in Data Connection will be able to select any outbound application as an authorization mechanism for their connection.

To view the **Outbound applications** page, you need access to the "Manage outbound applications" workflow, which is granted by default to the `Organization administrator` role in Control Panel.

Learn more about [configuring roles](/docs/foundry/platform-security-management/manage-roles/).

Select **New application** and provide the required inputs to set up your outbound application. Detailed configuration options are listed below.

### Configuration options

The following configuration options are available when creating an outbound application.

| Option | Required? | Description |
| --- | --- | --- |
| Authorization page URL | Yes | The [authorization endpoint ↗](https://datatracker.ietf.org/doc/html/rfc6749#section-3.1) for the OAuth 2.0 provider. Generally, this URL is similar to `https://oauth2-server.com/authorize` and should be available in public documentation for most SaaS offerings. |
| Token endpoint URL | Yes | The [token endpoint ↗](https://datatracker.ietf.org/doc/html/rfc6749#section-3.2) for the OAuth 2.0 provider. Generally, this URL is similar to `https://oauth2-server.com/token` and should be available in public documentation for most SaaS offerings. |
| Egress policy | Yes | To connect to the OAuth 2.0 server, you must create and attach an egress policy that allows connecting to the token endpoint URL. Use a [direct connection egress policy](/docs/foundry/administration/configure-egress/#direct-connection-egress-policies) for servers accessible from Foundry's network, or an [agent proxy egress policy](/docs/foundry/administration/configure-egress/#agent-proxy-egress-policies) for servers on a private network. [Egress](/docs/foundry/administration/configure-egress/) is managed per enrollment. |
| Application name | Yes | A user-facing name for this outbound application. The name is visible when selecting the outbound application in Data Connection while configuring authentication for a source. |
| Description | No | A description of the outbound application. |
| Approval prompt | Yes | This text will be displayed when using the authorization grant flow before opening a pop-up window to display the login prompt for the external OAuth 2.0 provider. |
| Client ID | Yes | The [client identifier ↗](https://datatracker.ietf.org/doc/html/rfc6749#section-2.2) from the OAuth 2.0 provider. Note that some external systems may use a different term to refer to client ID. |
| Client secret | Yes | The [client secret ↗](https://datatracker.ietf.org/doc/html/rfc6749#section-2.3) from the OAuth 2.0 provider. Note that some external systems may use a different term to refer to a client secret. |
| Scopes | Yes | The list of scopes configured in on the OAuth 2.0 server corresponding to the provided client ID. The list entered here must match what is listed in the administration interface for the external system. Scopes may be left blank to indicate that an empty list of scopes should be provided when authorizing with the OAuth 2.0 server. |
| Access token expiration | No | An optional access token expiration. Normally, this value is returned by the OAuth 2.0 server as part of the authorization grant flow. Any value entered here will be ignored if the server provides an access token expiration. |
| Enable PKCE | No | An optional toggle. When enabled, the outbound application will use [proof key for code exchange ↗](https://datatracker.ietf.org/doc/html/rfc7636) (PKCE), an extension to the authorization code grant that protects against CSRF and authorization code injection attacks with an additional security handshake. This setting is highly recommended for public clients, and some OAuth 2.0 servers enforce it. |

We recommend using a [direct connection](/docs/foundry/data-connection/architecture/#foundry-worker-with-direct-connection-policies) when setting up an outbound application if the OAuth 2.0 server is reachable from Foundry directly. For cloud-hosted Foundry instances, this means a server that can be reached through a direct Internet connection.

If the OAuth 2.0 server is on a private network, you can use an [agent proxy egress policy](/docs/foundry/administration/configure-egress/#agent-proxy-egress-policies) to route traffic through a [Data Connection agent](/docs/foundry/data-connection/core-concepts/#agents) installed in your network. This allows Foundry to connect to the OAuth 2.0 server as if it were directly reachable.

![A view in Control Panel of the configuration options available for outbound applications.](Configure Outbound Applications_assets/img_001.png)

### (Legacy) Custom webhook-based OAuth 2.0 handshakes

Legacy

Custom webhook-based OAuth 2.0 configuration is in the [legacy](/docs/foundry/platform-overview/development-life-cycle/) phase of development, and no additional development is expected. Full support remains available. This configuration may not be available on your enrollment. If the OAuth 2.0 server implements the standard Authorization Code grant flow, use the [standard configuration options](#configuration-options) with a [direct connection](/docs/foundry/administration/configure-egress/#direct-connection-egress-policies) or [agent proxy egress policy](/docs/foundry/administration/configure-egress/#agent-proxy-egress-policies) instead.

If the OAuth 2.0 server requires a non-standard token handshake, you can use custom webhooks to implement the OAuth 2.0 flow manually. This approach routes handshake requests through a [REST API](/docs/foundry/available-connectors/rest-apis/) source using an [agent worker](/docs/foundry/data-connection/core-concepts/#agent-worker) to connect to the server.

![A view in Control Panel of the configuration options available for on-premise outbound applications.](Configure Outbound Applications_assets/img_002.png)

| Option | Required? | Description |
| --- | --- | --- |
| Source connection | Yes | Choose a [REST API source](/docs/foundry/available-connectors/rest-apis/) that is configured to use an agent worker to connect to the OAuth 2.0 server inside your network. |
| Webhook to get token | Yes | A webhook that calls the [token endpoint ↗](https://datatracker.ietf.org/doc/html/rfc6749#section-3.2) for the OAuth 2.0 server to fetch a token. See the [section below](#token-webhook) for more information. |
| Webhook to get refresh token | No | A webhook that calls the [token endpoint ↗](https://datatracker.ietf.org/doc/html/rfc6749#section-3.2) for the OAuth 2.0 server to refresh a token. See the [section below](#refresh-token-webhook) for more information. |
| Authorization page URL | Yes | The [authorization endpoint ↗](https://datatracker.ietf.org/doc/html/rfc6749#section-3.1) for the OAuth 2.0 provider. Generally, this URL is similar to `https://oauth2-server.com/authorize`. |

To set up a custom webhook-based handshake:

1. Create a [REST API source](/docs/foundry/available-connectors/rest-apis/) that can connect to your OAuth 2.0 server. For example, you might configure your source with the server domain and client secret:

![A REST API source configured with the domain my-outh-server.com and a hidden client secret.](Configure Outbound Applications_assets/img_003.png)

2. Create a webhook on that REST API source that calls the `/token` endpoint of the OAuth 2.0 server. You can optionally create a second webhook for the refresh flow.

Use the instructions in the following sections to create the token and refresh token webhooks.

#### Token webhook

The token webhook should implement a request to the `/token` endpoint on the OAuth 2.0 server to fetch a valid token given a `client_id`, `redirect_uri`, and `authorization_code`. The token webhook must have the input and output parameters listed below. Learn more about [webhook parameters](/docs/foundry/data-connection/webhooks-reference/#configuring-inputs-and-outputs).

Below is an example request configuration showing a standard OAuth 2.0 server token request:

![An OAuth 2.0 server token request with key-value pairs.](Configure Outbound Applications_assets/img_004.png)

| Input parameter | Required? | Type |
| --- | --- | --- |
| `client_id` | Yes | String |
| `redirect_uri` | Yes | String |
| `authorization_code` | Yes | String |
| `code_verifier` | No | String |

You must also set `grant_type=authorization_code` as one of the entries in the webhook form body. `code_verifier` is required if **Enable PKCE** is toggled on in your outbound application. Review the documentation for your OAuth 2.0 server for additional required configurations.

| Output parameter | Required? | Type | Comments |
| --- | --- | --- | --- |
| `access_token` | Yes | String |  |
| `scope` | Yes | String |  |
| `refresh_token` | No | String | Not all OAuth 2.0 servers or applications will be configured to allow automatic token refresh. If no refresh token is returned, users will be prompted to re-authorize the application whenever the original token expires. |
| `expires_in` | No | String | Not all OAuth 2.0 servers will return this parameter. If this parameter is not present or a value is not returned, the access token expiration in the outbound application will be used instead. |

#### Refresh token webhook

After creating the token webhook, you must create a second webhook for the refresh token flow using the input and output parameters listed below. The webhook should implement a request to the `/token` endpoint to fetch a new token given a `refresh_token` and `client_id`.

Below is an example request configuration showing a standard OAuth 2.0 server refresh request:

![An OAuth 2.0 server refresh request with key-value pairs.](Configure Outbound Applications_assets/img_005.png)

| Input parameter | Required? | Type |
| --- | --- | --- |
| `client_id` | Yes | String |
| `refresh_token` | Yes | String |

You must also set `grant_type=refresh_token` as one of the entries in the webhook form body. Review the documentation for your OAuth 2.0 server for additional required configurations.

The available output parameters are the same as the parameters for the [token webhook](#token-webhook).

| Output parameter | Required? | Type | Comments |
| --- | --- | --- | --- |
| `access_token` | Yes | String |  |
| `scope` | Yes | String |  |
| `refresh_token` | No | String | Not all oauth servers or applications will be configured to allow automatic token refresh. If no refresh token is returned, users will be prompted to re-authorize the application whenever the original token expires. |
| `expires_in` | No | String | Not all OAuth 2.0 servers will return this parameter. If this parameter is not present or a value is not returned, the access token expiration in the outbound application will be used instead. |

## Manage outbound applications

Once you create an outbound application, it will be available to all users in that Organization to use as the authorization method for a [REST API source](/docs/foundry/available-connectors/rest-apis/) in [Data Connection](/docs/foundry/data-integration/overview/). The following options are available for administrators with permisssions to manage outbound applications.

### Delete an outbound application

Deleting an outbound application will permanently remove stored tokens and refresh tokens for all users who have authorized the application. The application configuration, including the client secret, will also be permanently deleted. This action cannot be reversed.

### Reset an outbound application

Resetting an outbound application will permanently remove stored tokens and refresh tokens for all users who have authorized the application. This effectively returns the application to the state just after initial setup when no user has yet completed the interactive authorization flow. The next time a user attempts to perform a workflow that requires a token from this outbound application, they will be prompted to complete the authorization flow again.

### Enable an outbound application

Enabling an outbound application means that users may perform the interactive authorization flow to this external system. Disabling an application will prevent any previously stored tokens and refresh tokens from being used. However, these tokens will not be deleted and may be used again after the application has been re-enabled.

New users who have not previously authorized the application will not be able to perform the interactive authorization flow while the outbound application is disabled.

Newly created applications will be enabled by default and may only be disabled after being created.

## Use outbound applications in Data Connection

For details on how to use outbound applications in Functions, Actions, Workshop, and AIP chatbots, see [Supported workflows](#supported-workflows) below.

Non-interactive workflows

Outbound applications work with automated workflows such as Automate and scheduled Functions as long as the user has a valid token. A user must first complete the authorization flow in an interactive UI context (such as Workshop or AIP Studio). If the refresh token expires before the automated workflow runs, the user must re-authorize through an interactive workflow.

Outbound applications do not support usage by users from multiple Organizations. If users in multiple organizations need to authorize against the same OAuth 2.0 server, you must create a separate outbound application with a different REST API source for users in each Organization.

Once an outbound application is created and enabled, it may be used as the authentication method for a domain in a [REST API source](/docs/foundry/available-connectors/rest-apis/). When configuring a domain, select **OAuth 2.0** and then select the desired outbound application from the dropdown.

![The Domain section of the REST API source configuration. The authorization is set to OAuth 2.0 from the dropdown menu. ](Configure Outbound Applications_assets/img_006.png)

Any webhooks using a domain with OAuth 2.0 configured will result in an interactive prompt for each user the first time they attempt to run it. Webhooks are most often invoked through [Actions](/docs/foundry/action-types/overview/) from [Workshop](/docs/foundry/workshop/overview/); in this case, when running an Action in Workshop, the user will see a pop-up window showing the authentication page of the OAuth 2.0 server, prompting them to authorize Foundry to interact with this system on their behalf.

In order to call OAuth 2.0 backed webhooks in a Function-backed Action, navigate to the rules configuration page of your Action and call the webhook directly as the first rule followed by a Function to process the webhook response. You can also configure the webhook call to take [inputs from a function](/docs/foundry/action-types/webhooks/#input-parameters).

If a token refresh workflow is configured, users are unlikely to see this prompt again unless the authorization is revoked in the external system or the outbound application is reset. If no refresh workflow is configured, the end user will see the authentication pop-up anytime the resulting token expires. Tokens often expire within minutes or hours, and we encourage use of the refresh flow for a better user experience.

## Supported workflows

Once a REST API source is configured with an outbound application, the OAuth token is available across the following Foundry features:

- [Functions](/docs/foundry/functions/api-calls/#use-oauth-20-with-outbound-applications): Use OAuth tokens from your source in Python and TypeScript functions to authenticate with external systems.
- [Function-backed actions](/docs/foundry/action-types/function-actions-overview/): Actions whose backing functions call an OAuth-configured source pass the token automatically.
- [Webhook side effects](/docs/foundry/action-types/webhooks/): Webhooks on an OAuth-configured REST API source include the token in outgoing requests.
- [Workshop](/docs/foundry/workshop/actions-overview/): Actions applied in Workshop that require OAuth tokens trigger an authorization prompt if no valid token exists, or work automatically if the user has already authorized.
- [AIP chatbots](/docs/foundry/chatbot-studio/tools/): Agents that invoke OAuth-backed action tools use the same token lifecycle as other workflows.

[←

PREVIOUSConfigure remote Marketplace stores](/docs/foundry/administration/configure-remote-marketplace-stores/)

[NEXTConfigure platform experience

→](/docs/foundry/administration/configure-platform-experience/)

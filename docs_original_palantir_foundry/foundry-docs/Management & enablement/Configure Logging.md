---
title: "Palantir"
source: "https://www.palantir.com/docs/foundry/administration/configure-logging/"
scraped_at: "2026-04-26T16:21:39Z"
---

# Palantir

## Captura de pantalla

![Screenshot](Configure Logging.screenshot.png)

---

[Management & enablement](/docs/foundry/administration/overview/)Organization settings[Configure logging](/docs/foundry/administration/configure-logging/)

# Configure logging [Beta]

Beta

Log exporting is in the [beta](/docs/foundry/platform-overview/development-life-cycle/) phase of development and may not be available on your enrollment. Functionality may change during active development. Contact Palantir Support for more information.

Applications across Foundry emit logs that provide visibility into their operation, including transform runs, build failures, and other system events.

Foundry logs contain information about:

- Application and function execution events
- Transform and pipeline runs
- Build and deployment activities
- System errors and warnings

Additionally, any user-added logs written in Code Repositories will also be emitted.

These logs can be exported to a Foundry streaming dataset for real-time monitoring and analysis. This feature is only available to Organization Administrators, who can configure a destination where logs from Foundry applications will flow into a Foundry streaming dataset.

Along with log export management, Ontology and AIP in platform log access is managed by Organization Administrators and Information Security Officers.

To start managing the log observability settings for your organization:

1. Navigate to the Control Panel application.
2. In the top banner, select the relevant organization from the dropdown.
3. In the left-hand sidebar, select **Log observability settings** under the **Organization** section. Alternatively, in the home page, search for `Log observability settings` and select the relevant page in the search results.

![The Control Panel search interface showing the log observability settings option.](Configure Logging_assets/img_001.png)

## Export Foundry logs

Foundry logs can be exported, per-organization, directly into a Foundry streaming dataset.

After defining an export configuration, log entries will be continuously written to the streaming dataset from the time the configuration is created. You can process and analyze the data in real-time using Foundry's extensive suite of data transformation and visualization tools or export the logs to external monitoring systems.

### Export permissions

To export Foundry logs, a user must be an **Organization Administrator**. This role provides the necessary permissions to configure log exporting destinations for the organization. See [organization permissions](/docs/foundry/platform-security-management/manage-orgs-and-spaces/#organizations) for more details about organization-level permissions.

Log data might contain sensitive information. We recommend applying [security markings](/docs/foundry/security/markings/) to the output streaming dataset to control access to this data. You can apply markings directly during export creation or add them to the dataset afterward.

### Define an export configuration

Foundry logs can be exported from resources belonging to a set of projects, or from all resources in the organization. Some limitations apply when defining an export configuration:

- Each organization can have a maximum of 50 export configurations.
- Each project can be included in at most three export configurations.
  - This limit includes configurations that export logs from all projects.
- When selecting a set of projects for a configuration, you can include up to 100 projects per configuration.

We recommend exporting logs from resources in specific projects rather than all organizational resources. This will simplify access control for the resulting dataset.

Follow the steps below to define an export configuration:

1. Select **Create log export**, which will open a dialog to define a configuration.

![The log export configurations page displaying existing configurations and a button to create new exports.](Configure Logging_assets/img_002.png)

2. Select the projects to export relevant logs from. You can filter the project list by space using the dropdown, or search for projects by name. Projects that are already included in other export configurations display a count indicating the number of existing exports.

![The log scope selection interface showing a list of projects to select for log export.](Configure Logging_assets/img_003.png)

3. Specify the name and location of the streaming dataset where logs will be exported. The location must be within the Foundry file system and cannot be a user-created folder.

![The export location configuration dialog showing fields to specify the dataset name and location.](Configure Logging_assets/img_004.png)

4. Select a log schema format. Foundry provides two schema types: an internal Palantir format that provides the payload as a serialized JSON string, or the OpenTelemetry (OTel) protocol format with the payload serialized as a protocol buffer binary. For more details, see [Log schema](#log-schema).

![The schema selection interface showing available log format options.](Configure Logging_assets/img_005.png)

5. In the **Apply markings** step, select the security markings to apply to the output streaming dataset. This step is optional but recommended to control access to log data, as that data may contain sensitive information. Selected markings will be applied to the resulting log dataset after the export configuration is created.

![The apply markings step showing a search interface for selecting security markings to apply to the log dataset.](Configure Logging_assets/img_006.png)

6. Select **Create configuration** after reviewing the summary and access requirements. The summary displays the selected markings along with the other configuration details.

![The summary step displaying selected projects, export location, schema, applied markings, and access requirements confirmation.](/docs/resources/foundry/administration/log-exporting-review-access-requirements.png)

7. After the configuration is created, it will appear in the list of existing configurations. Any applied markings are displayed alongside the export configuration details.

Note that when first configuring a log export, it may take up to five minutes for logs to begin streaming into the dataset at the export location.

### Disable log exporting

To disable log exporting, delete any listed log storage configurations by selecting the trash icon on the right side of the configuration. The resulting dataset will continue to exist, but no new logs will be exported to it. Additionally, if the streaming dataset is in the trash or permanently deleted, logs will stop being exported.

### Analyze log data

Log storage datasets can contain high volumes of streaming data, so we recommend filtering the dataset appropriately before performing analysis. Consider time-based filtering to focus on relevant log entries for your monitoring and troubleshooting needs.

Foundry provides many powerful tools for performing log analysis, such as [Transforms](/docs/foundry/code-workbook/transforms-overview/) and [Pipeline Builder](/docs/foundry/pipeline-builder/overview/).

## In platform log access for Ontology and AIP workflows

Ontology and AIP in platform log access is managed at the project level. The log access controls for in platform viewing are distinct from the permissions configured for the export streaming dataset. These controls pertain specifically to functions, actions, automation workflows, and language models called from enabled projects.

To manage the in platform log access controls by project:

1. Navigate to the **Log access** tab. This tab displays the projects with log reading enabled for your organization, if any. By default, log access is disabled for all projects within an organization.

![The log access tab displaying projects with enabled log reading permissions.](/docs/resources/foundry/administration/log-access-observability-settings.png)

2. To enable log access for a project, select **Add project**, which will open a dialog to configure telemetry log visibility for a project.

![The project selection dialog for configuring telemetry log visibility settings.](/docs/resources/foundry/administration/configure-telemetry-log-visibility-for-project.png)

3. Once a project is selected, a second dialog will appear to enable log reading for the project and apply necessary markings.

![The confirmation dialog showing the selected project with options to enable log reading.](/docs/resources/foundry/administration/project-selected-to-configure-log-visibility.png)

4. After log access is enabled for the project, it will appear in the list of enabled projects.

![The updated list showing all projects with log access enabled.](/docs/resources/foundry/administration/project-with-enabled-log-access.png)

5. You should then see the updated policy reflected throughout the platform, with log viewing enabled for all source executor resources residing in the project.

![The platform interface showing the applied log access policy for the project.](/docs/resources/foundry/administration/project-policy-displayed-in-platform.png)

To view logs of a workflow execution (with seven day retention) you did not invoke, you will need:

- **Log reading enabled** for the project the source executor resides in.
- **Markings access** to any markings configured with the project log access settings.
- **Edit permission** on the source executor the logs were emitted under.

Independent of the log access status managed by the administrator, users are able to see logs of executions they ran within the past 24 hours.

To learn more about in platform log access, see our [in platform log permissioning](/docs/foundry/aip-observability/log-permissioning/) documentation.

## Log schema

Foundry application logs are structured logs that follow a consistent schema, making them suitable for programmatic analysis and monitoring.

Specifically, Foundry logs contain information about various application events, including transform executions, build processes, system errors, and other operational activities. The log structure varies depending on the type of application or service that generates the log entry.

Application-specific information is captured within structured fields that provide context about the operation being performed, including execution details, error information, and performance metrics.

Foundry provides two schema types:

- An internal Palantir format that provides the payload as a serialized JSON string
- The OpenTelemetry (OTel) protocol provides logs in [OTel protocol ↗](https://opentelemetry.io/docs/specs/otel/overview/) format, with the payload serialized as a protocol buffer binary for ingestion downstream to an OTel collector.

![The schema selection interface showing available log format options.](/docs/resources/foundry/administration/configure-logging-select-schema.png)

## Log attribution

Foundry application logs are generated by various services and applications within the platform. When logs are streamed to the configured dataset, they are filtered to include only logs relevant to the organization that configured the log storage configuration.

Log exports configured for a given organization will receive logs from applications and services used by that organization. This includes transform executions, builds, and other activities performed within the organization's context.

The RID of the resource that generated the log is included in the log entry and can be used to trace the log back to the resource.

## Log guarantees

Foundry logs *are not* [audit logs](/docs/foundry/security/audit-logs-overview/). There is no guarantee of 100% reliable log delivery.

Log contents produced by Foundry are subject to change without notice.

[←

PREVIOUSConfigure Workshop](/docs/foundry/administration/configure-workshop/)

[NEXTConfigure user and group visibility

→](/docs/foundry/administration/configure-user-and-group-visibility/)

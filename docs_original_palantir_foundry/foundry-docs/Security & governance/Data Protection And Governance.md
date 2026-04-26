---
title: "Palantir"
source: "https://www.palantir.com/docs/foundry/security/data-protection-and-governance/"
scraped_at: "2026-04-26T16:20:24Z"
---

# Palantir

## Captura de pantalla

![Screenshot](Data Protection And Governance.screenshot.png)

---

[Security & governance](/docs/foundry/security/overview/)[Data protection and governance](/docs/foundry/security/data-protection-and-governance/)

# Data protection and governance

Foundry empowers users to integrate, transform, analyze, and operationalize data across an enterprise, while securely collaborating and sharing data products. An important part of every data strategy and initiative is ensuring lawful, legitimate, compliant, and appropriate use of the data. Data protection and data governance are central concepts to using Foundry.

- **Data protection** refers to the range of technical and organizational means available to ensure that the processing of personal data is at all times limited to that which is necessary and proportional to achieve a legitimate processing outcome.
- **Data governance** refers to the management of enterprise data across the entire lifecycle of processing, from ingestion to access to deletion.

While privacy regulations vary around the world, most regulations require systematic and programmatic means of understanding what data an organization has in order to appropriately use and handle the data. Data protection and data governance go hand-in-hand in Foundry. This documentation outlines best practices and tools available to our customers to help facilitate adequate and appropriate data protection and data governance when processing data, including sensitive or personal data, in Palantir Foundry.

This document is meant for data administrators, data governance owners, data owners, and users who are working with sensitive data on the platform and want to understand Foundry's capabilities to protect this data.

If you have questions about data protection, data governance, or protecting sensitive data, Palantir has a [Privacy and Civil Liberties (PCL) team ↗](https://www.palantir.com/pcl/) that can provide guidance on handling sensitive data in Palantir Foundry. Contact your Palantir representative to be connected with Palantir's PCL team.

## Data protection principles

Platform administrators and users should seek to handle enterprise data, and in particular personal or sensitive data, responsibly at all times. The Fair Information Practice Principles (FIPPs) provide a useful set of guidelines that serve as foundational principles of working with personal data that enforces privacy protection.

### Fair Information Practice Principles (FIPPs)

Below, we provide an overview of the FIPPs, based on the [OECD Guidelines on the Protection of Privacy and Transborder Flows of Personal Data ↗](https://www.oecd.org/en/topics/sub-issues/privacy-and-personal-data-protection.html):

- **The Collection Limitation Principle.** There should be limits to the collection of personal data and any such data should be obtained by lawful and fair means and, where appropriate, with the knowledge or consent of the data subject.
- **The Data Quality Principle.** Personal data should be relevant to the purposes for which they are to be used and, to the extent necessary for those purposes, should be accurate, complete and kept up-to-date.
- **The Purpose Specification Principle.** The purposes for which personal data are collected should be specified no later than at the time of data collection and the subsequent use limited to the fulfillment of those purposes or such others as are not incompatible with those purposes and as are specified on each occasion of change of purpose.
- **The Use Limitation Principle**. Personal data should not be disclosed, made available or otherwise used for purposes other than those specified, except a) with the consent of the data subject, or b) by the authority of law.
- **The Security Safeguards Principle.** Personal data should be protected by reasonable security safeguards against such risks as loss or unauthorized access, destruction, use, modification or disclosure of data.
- **The Openness Principle.** There should be a general policy of openness about developments, practices and policies with respect to personal data. Means should be readily available of establishing the existence and nature of personal data and the main purposes of their use, as well as the identity and usual residence of the data controller.
- **The Individual Participation Principle.** An individual should have the right:
  - to obtain from a data controller, or otherwise, confirmation of whether or not the data controller has data relating to him/her/them;
  - to have data relating to him/her/them communicated to him/her/them, within a reasonable time, at a charge, if any, that is not excessive; in a reasonable manner, and in a form that is readily intelligible to him/her/them;
  - to be given reasons if a request made under subparagraphs (a) and (b) is denied and to be able to challenge such denial; and
  - to challenge data relating to him/her/them and, if the challenge is successful, to have the data erased, rectified, completed or amended;
- **The Accountability Principle.** A data controller should be accountable for complying with measures which give effect to the principles stated above.

Among the common themes across these FIPPs is the need for **Data Minimization**, where data should only be collected ("Collection Limitation Principle") and used ("Use Limitation Principle") for explicit and authorized purposes ("Purpose Specification Principle"). Additionally, data must always be handled with security assurance in mind ("Security Safeguards Principle").

#### FIPPs Example

For example, consider the following scenario.

A financial institution (FI) may apply FIPPs when considering how to handle a new customer program required for providing a banking service.

As the FI sets up the program, the financial institution would only seek to collect personal information necessary for running the program from customers who are enrolling in it ("Collection Limitation Principle"), where all purposes are provided upfront ("Purpose Specification Principle") and the details and method of data handling are openly disclosed ("Openness Principle").

As the data is prepared for users, data owners and preparers ensure the data is regularly maintained and reviewed so that any decision pertaining to the data uses accurate up-to-date information ("Data Quality Principle") with assurances that all data is securely stored ("Security Safeguards Principle"). System-wide processes like regular audit reviews ensure the data is only used for pre-specified purposes ("Accountability Principle"). Once the data is ready for users, only authorized users who work on the data for the approved purposes have access to that data ("Use Limitation").

Meanwhile, on the consumer side, the FI allows consumers to regularly request access, delete, or correct information ("Individual Participation Principle"). Furthermore, beyond FIPPs, it may be required to adhere to other financial sector requirements on how long the data needs to be retained for compliance reasons.

In example alone, many complexities and considerations in handling data are involved. The best practices outlined in this documentation will provide an overview of the wide range of technical tools in Palantir Foundry that help operationalize these foundational principles when working with sensitive data, including personal data.

### Beyond FIPPs

FIPPs are just a starting point for evaluating the privacy of personal or sensitive information. Principles of fairness, non-discrimination, and ethics may also be relevant to personal data processing. Different legal, regulatory, and administrative requirements may vary by jurisdiction, sector, and general norms. Consult a legal counsel or privacy expert to advise on relevant requirements.

## Sensitive data classification

Identifying sensitive data is a critical first step. Context matters because whether certain data is considered sensitive or not depends on the relevant privacy regulations and norms.

**Sensitive data** here is defined as any data that is broadly classified and/or requires extra security. Some laws formally designate specific data elements as sensitive (for example, the EU's General Data Protection Regulation), whereas others are determined by the data owners or by common recognition regardless of legal status (such as Social Security Numbers). Whether data is classified as sensitive generally depends largely on the type or classification of data (e.g., personally identifiable), the types of workflows (such as limited to specific purposes), or any content (such as sensitive enterprise information) that may trigger restricted access controls.

One common example of sensitive data is **Personally Identifiable Information (PII)**, which includes direct identifiers and other information about individuals that can be used to re-identify individuals or single them out.

Examples of sensitive information include:

- **Contact information:** Name, email, phone number
- **ID numbers:** Social Security number (SSN), license number, medical record number, tax identification numbers (TIN)
- **Biometrics:** Facial signatures, fingerprints, images of individuals, DNA
- **Dates:** Birth date, admission or discharge dates
- **Location information:** Home address, office address, wearables location data, cell phone locations
- **Health information:** Past, present, or future physical or mental health or condition, medications, treatments and diagnoses
- **Financial information:** Income or assets, medical bills or payments, account numbers
- **Other sensitive information:** Phone logs, IP addresses

Depending on the jurisdiction or field, sensitive data might be classified differently. Below are examples of some relevant data protection and privacy regulation definitions:

### EU General Data Protection Regulation (GDPR)

EU General Data Protection Regulation (GDPR) is one regulation which explicitly defines personal data. To summarize, the GDPR defines personal data as any piece of data that can identify an individual and also classifies characteristics like race and political opinions as sensitive. The formal definition is below:

[Article 4(1) ↗](https://gdpr-info.eu/art-4-gdpr/) of the GDPR classifies personal data as:

> any information relating to an identified or identifiable natural person ("data subject"); an identifiable natural person is one who can be identified, directly or indirectly, in particular by reference to an identifier such as a name, an identification number, location data, an online identifier or to one or more factors specific to the physical, physiological, genetic, mental, economic, cultural or social identity of that natural person.

Furthermore, [Art. 9(1) ↗](https://gdpr-info.eu/art-9-gdpr/) of the GDPR highlights the following types of personal data as special categories of personal data warranting additional layers of care and protection:

> personal data revealing racial or ethnic origin, political opinions, religious or philosophical beliefs, or trade union membership, and the processing of genetic data, biometric data for the purpose of uniquely identifying a natural person, data concerning health or data concerning a natural person’s sex life.

### US Health Insurance Portability and Accountability Act (HIPAA)

The US' Health Insurance Portability and Accountability Act (HIPAA) outlined in HHS’s [detailed guidance ↗](https://www.hhs.gov/hipaa/for-professionals/privacy/special-topics/de-identification/index.html) is a common framework for handling healthcare data in the US. HIPAA applies to data held by health care providers and insurers, but it does not necessarily apply to other processors of the same data. To confirm, consult a legal counsel or privacy expert to advise on relevant requirements.

Under HIPAA,

> Protected health information is information, including demographic information, which relates to:
>
> - the individual’s past, present, or future physical or mental health or condition,
> - the provision of health care to the individual, or
> - the past, present, or future payment for the provision of health care to the individual, and that identifies the individual or for which there is a reasonable basis to believe can be used to identify the individual. Protected health information includes many common identifiers (e.g., name, address, birth date, Social Security Number) when they can be associated with the health information listed above.

For example, a medical record, laboratory report, or hospital bill handled by a HIPAA-covered entity would be PHI because each document would contain a patient’s name and/or other identifying information associated with the health data content.

### Other

Other relevant international data protection frameworks which could apply to your organization or project include:

- Brazil - Lei Geral de Proteção de Dados (LGPD)
- Japan - Act on the Protection of Personal Information (AAPI)
- Canada - Personal Information Protection and Electronic Documents Act (PIPEDA)
- California, US - California Consumer Privacy Act (CCPA) / California Privacy Rights Act (CPRA)
- Virginia, US - Virginia Consumer Data Protection Act (CDPA)
- Colorado, US - Colorado Privacy Act (CPA)

The above list is not comprehensive and explicit definitions will vary depending on local jurisdiction, region, or sector. Consult your legal, compliance, and/or data protection team(s) to understand relevant classifications of data and specific handling instructions based on applicable rules and regulations.

## Data governance oversight

Before you start working with data in Palantir Foundry, make sure you identify relevant subject-matter experts (SMEs) and accountable authorities for data protection and data governance at your organization. Handling enterprise data, including personal and other sensitive data, comes with it a series of considerations: from legal and regulatory requirements to operationalizing organizational rules set by the platform/data owner(s).

Below are a series of common procedures to follow or check in on with your organization before starting to process data in Palantir Foundry:

**Identify a governance committee/SMEs or designate a data governance lead**

- A governance committee does not need to be a formalized group and may take different forms for every organization. It is important to understand who in the organization is the responsible party or parties to think through, adjudicate, and help problem solve data governance and data protection questions as use of the platform evolves, use cases expand, etc.
- The responsibility for ensuring data is handled appropriately typically lies with the respective data owner(s). This includes determining how basic principles of data processing outlined below should be applied to specific workflows, but also ensuring that system-wide data governance requirements can be met, such as ensuring accountable use of the platform with [audit logs](/docs/foundry/security/audit-logs-overview/) and configuring relevant [retention policies](#configure-data-retention-and-deletion).
- These data governance requirements are commonly defined by an organization’s Data Protection Office (DPO), Information Security, Compliance, or Legal functions.

Best practice

Consult with the respective data owner(s) or data controller to identify relevant parties who will know and can sign off on how to handle data in accordance with applicable regulations, use agreements, and other requirements.

**Complete Required Privacy Reviews or Privacy Impact Assessment**

- Depending on industry, region/jurisdiction, application, or organization-specific requirements, additional documentation may be required before processing can begin. This could come in the form of an organization-specific or regulatory requirement to prepare documentation such as Privacy Threshold Assessments, Privacy/Data Protection Impact Assessments, Conformity Assessments, Privacy Statements, Record of Processing Activities, System of Records Notices (SORNs), etc.
- Check with relevant teams internally on standard operating procedures for starting new projects, especially if those projects involve handling sensitive data.

**Work with your Legal and Compliance team**

Best practice

Work with internal legal and compliance teams to determine the documentation necessary for a system to start handling sensitive data.

In case of ad-hoc questions or a need for broader alignment, ensure that you as a data administrator or user are in touch with the relevant legal and compliance team to stay informed about needs.

Best practice

Engage legal and compliance teams early to inform them on the use and scope of the Foundry platform.

**Connect with Palantir’s Privacy and Civil Liberties Team**

Palantir also has a Privacy and Civil Liberties (PCL) team that can be used as a resource for general best practices of handling sensitive data in Palantir Foundry. Contact your Palantir representative to be connected with Palantir's PCL team.

## Best practices and tips

### Ensure data minimization

Where sensitive data is not needed for certain user groups or projects, drop the columns or rows containing that data to limit access to sensitive data downstream. Make sure that any access to sensitive data is limited to explicit approved purposes as defined by the data owner(s) and/or relevant data protection and data governance teams.

For an additional layer of security, we recommend using the [Cipher](/docs/foundry/cipher/overview/) service to obfuscate data using cryptographic operations (encryption, decryption, or hashing). Cipher provides users the tools to configure privacy and governance protections in operational workflows on top of Foundry's sophisticated encryption at the storage and network levels.

### Ensure accountable data usage

Checkpoints is a Foundry application that facilitates accountability and purpose limits by enabling data governance teams to request justifications before certain sensitive data actions can be performed. For more details, see the [Checkpoints](/docs/foundry/checkpoints/overview/) documentation and workflow for [requesting justifications for sensitive actions](/docs/foundry/security/requesting-justification-for-sensitive-actions/).

Best practice

Deploy Checkpoints if users should provide a justification and/or acknowledgment prior to being able to perform actions considered sensitive in your particular processing environment.

### Scan for sensitive datasets

Sensitive Data Scanner is a Foundry application that enables administrators to create organization-specific definitions of sensitive data (such as PII) and a policy around what should happen when data matching this definition is identified. Sensitive Data Scanner can be triggered manually or configured to run in the background and watch for new data entering a dataset, project, or the platform. When Sensitive Data Scanner detects that a dataset contains information that corresponds to a pre-specified definition of sensitive data, the application will trigger a configured response, such as alerting administrators by creating a [Foundry-generated Issue](/docs/foundry/getting-help/issues/) or proactively locking down the dataset by applying a [Security Marking](/docs/foundry/security/markings/). For more details, see the [Sensitive Data Scanner documentation](/docs/foundry/sensitive-data-scanner/overview/).

### Configure data retention and deletion

**Data retention** describes the process that governs how long data is stored in Foundry and how data is removed from Foundry. Consistent with FIPPs, sensitive data like PII typically need to be deleted as soon as the processing purpose has been fulfilled, in order to comply with applicable data protection regulations.

Depending on your contractual agreements or compliance needs, some data may also need to be retained. You should therefore be mindful that deletion from Foundry at some point becomes irreversible and proactively implement relevant controls, such as eligibility reviews, throughout the retention process.

We highly recommended you determine retention requirements as early as possible, ideally before any data is ingested at all.

For more information, refer to the documentation on [how retention works in Foundry](/docs/foundry/retention/overview/), or contact your Palantir representative.

[←

PREVIOUSThird-party applications / Foundry third-party application & API guidance](/docs/foundry/platform-security-third-party/3pa-api-guidance/)

[NEXTDownload controls

→](/docs/foundry/security/download-controls/)

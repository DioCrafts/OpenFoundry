---
title: "Palantir"
source: "https://www.palantir.com/docs/foundry/object-link-types/object-types-overview/"
scraped_at: "2026-04-26T16:20:10Z"
---

# Palantir

## Captura de pantalla

![Screenshot](Object Types Overview.screenshot.png)

---

[Ontology building](/docs/foundry/ontology/overview/)Object and link types[Object types](/docs/foundry/object-link-types/object-types-overview/)[Overview](/docs/foundry/object-link-types/object-types-overview/)

# Object types

An **object type** is the schema definition of a real-world entity or event.
An **object or object instance** refers to a single instance of an object type; an object corresponds to a single real-world entity or event.
An **object set** refers to a collection of multiple object instances; that is, an object set represents a group of real-world entities or events.

For example, in the Ontology Manager, you may create an `Employee` object type that defines the characteristics for “All employees” or all objects of that type. An object refers to a single instance of the `Employee` object type, like the notional employees “Melissa Chang”, “Akriti Patel”, or “Diego Rodriguez.” A group of objects like “All tenured employees” represents an object set.

Similarly, in the Ontology Manager, you may create a `Flight` object type that defines characteristics for “All flights” or all objects of that type. An object refers to a single instance of the `Flight` object type, like “JFK → SFO 2021-02-24” or “TLV → LHR 2020-04-16.” A group of objects like “All arrived flights” represents an object set.

The concepts underpinning the Ontology have analogous concepts in the structure of a dataset. The definition of an object type in the Ontology is analogous to that of a dataset, while the definition of an object is analogous to that of a row in the dataset. The definition of an object set is analogous to a filtered set of rows in a dataset. For example, an `Employee` dataset may define the schema for “All employee rows.” In this case, a single row refers to a single employee, like “Melissa Chang,” “Akriti Patel,” or “Diego Rodriguez." If you filter the dataset based on tenure, you will have a set of rows that represent “All tenured employees.”

Rather than being an abstract data model, the Foundry Ontology maps each ontological concept to an organization's actual data, enabling this data asset to power real-world applications. Objects are created and displayed in user applications by adding backing datasources to an object type in the Ontology Manager. To create objects of type `Employee`, an organization will add backing datasources to the `Employee` object type and connect their employee directory and other enterprise data into the Ontology.

Get started by learning how to [create an object type](/docs/foundry/object-link-types/create-object-type/).

[←

PREVIOUSTypes reference](/docs/foundry/object-link-types/type-reference/)

[NEXTCreate an object type

→](/docs/foundry/object-link-types/create-object-type/)

use std::collections::{BTreeMap, HashSet};

use serde::Serialize;
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::{
    domain::type_system::validate_property_value,
    models::{
        interface::InterfaceProperty, property::Property, shared_property::SharedPropertyType,
    },
};

#[derive(Debug, Clone, Serialize)]
pub struct EffectivePropertyDefinition {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub property_type: String,
    pub required: bool,
    pub unique_constraint: bool,
    pub time_dependent: bool,
    pub default_value: Option<Value>,
    pub validation_rules: Option<Value>,
    pub source: String,
}

const SHARED_PROPERTY_PRECEDENCE: u8 = 0;
const INTERFACE_PROPERTY_PRECEDENCE: u8 = 1;
const DIRECT_PROPERTY_PRECEDENCE: u8 = 2;

fn merge_effective_definitions(
    definitions: impl IntoIterator<Item = (u8, EffectivePropertyDefinition)>,
) -> Vec<EffectivePropertyDefinition> {
    let mut merged = BTreeMap::<String, (u8, EffectivePropertyDefinition)>::new();

    for (precedence, definition) in definitions {
        match merged.get(&definition.name) {
            Some((existing_precedence, _)) if *existing_precedence >= precedence => continue,
            _ => {
                merged.insert(definition.name.clone(), (precedence, definition));
            }
        }
    }

    merged
        .into_values()
        .map(|(_, definition)| definition)
        .collect()
}

pub async fn load_effective_properties(
    db: &sqlx::PgPool,
    object_type_id: Uuid,
) -> Result<Vec<EffectivePropertyDefinition>, sqlx::Error> {
    let shared = sqlx::query_as::<_, SharedPropertyType>(
        r#"SELECT spt.id, spt.name, spt.display_name, spt.description, spt.property_type,
                  spt.required, spt.unique_constraint, spt.time_dependent, spt.default_value,
                  spt.validation_rules, spt.owner_id, spt.created_at, spt.updated_at
           FROM shared_property_types spt
           INNER JOIN object_type_shared_property_types otsp
                ON otsp.shared_property_type_id = spt.id
           WHERE otsp.object_type_id = $1
           ORDER BY otsp.created_at ASC, spt.created_at ASC"#,
    )
    .bind(object_type_id)
    .fetch_all(db)
    .await?;

    let direct = sqlx::query_as::<_, Property>(
        r#"SELECT id, object_type_id, name, display_name, description, property_type, required,
                  unique_constraint, time_dependent, default_value, validation_rules,
                  inline_edit_config, created_at, updated_at
           FROM properties
           WHERE object_type_id = $1
           ORDER BY created_at ASC"#,
    )
    .bind(object_type_id)
    .fetch_all(db)
    .await?;

    let interfaces = sqlx::query_as::<_, InterfaceProperty>(
        r#"SELECT ip.id, ip.interface_id, ip.name, ip.display_name, ip.description, ip.property_type,
                  ip.required, ip.unique_constraint, ip.time_dependent, ip.default_value,
                  ip.validation_rules, ip.created_at, ip.updated_at
           FROM interface_properties ip
           INNER JOIN object_type_interfaces oti ON oti.interface_id = ip.interface_id
           WHERE oti.object_type_id = $1
           ORDER BY ip.created_at ASC"#,
    )
    .bind(object_type_id)
    .fetch_all(db)
    .await?;

    let definitions = shared
        .into_iter()
        .map(|property| {
            (
                SHARED_PROPERTY_PRECEDENCE,
                EffectivePropertyDefinition {
                    name: property.name,
                    display_name: property.display_name,
                    description: property.description,
                    property_type: property.property_type,
                    required: property.required,
                    unique_constraint: property.unique_constraint,
                    time_dependent: property.time_dependent,
                    default_value: property.default_value,
                    validation_rules: property.validation_rules,
                    source: "shared_property_type".to_string(),
                },
            )
        })
        .chain(interfaces.into_iter().map(|property| {
            (
                INTERFACE_PROPERTY_PRECEDENCE,
                EffectivePropertyDefinition {
                    name: property.name,
                    display_name: property.display_name,
                    description: property.description,
                    property_type: property.property_type,
                    required: property.required,
                    unique_constraint: property.unique_constraint,
                    time_dependent: property.time_dependent,
                    default_value: property.default_value,
                    validation_rules: property.validation_rules,
                    source: "interface".to_string(),
                },
            )
        }))
        .chain(direct.into_iter().map(|property| {
            (
                DIRECT_PROPERTY_PRECEDENCE,
                EffectivePropertyDefinition {
                    name: property.name,
                    display_name: property.display_name,
                    description: property.description,
                    property_type: property.property_type,
                    required: property.required,
                    unique_constraint: property.unique_constraint,
                    time_dependent: property.time_dependent,
                    default_value: property.default_value,
                    validation_rules: property.validation_rules,
                    source: "object_type".to_string(),
                },
            )
        }));

    Ok(merge_effective_definitions(definitions))
}

pub fn validate_object_properties(
    definitions: &[EffectivePropertyDefinition],
    properties: &Value,
) -> Result<Value, String> {
    let Some(properties) = properties.as_object() else {
        return Err("object properties must be a JSON object".to_string());
    };

    let known = definitions
        .iter()
        .map(|property| property.name.as_str())
        .collect::<HashSet<_>>();
    for key in properties.keys() {
        if !known.contains(key.as_str()) {
            return Err(format!("unknown property '{key}'"));
        }
    }

    let mut normalized = Map::new();
    for definition in definitions {
        let value = properties
            .get(&definition.name)
            .cloned()
            .or_else(|| definition.default_value.clone());

        match value {
            Some(value) => {
                validate_property_value(&definition.property_type, &value)
                    .map_err(|error| format!("{}: {}", definition.name, error))?;
                normalized.insert(definition.name.clone(), value);
            }
            None if definition.required => {
                return Err(format!("{} is required", definition.name));
            }
            None => {}
        }
    }

    Ok(Value::Object(normalized))
}

#[cfg(test)]
mod tests {
    use super::{
        DIRECT_PROPERTY_PRECEDENCE, EffectivePropertyDefinition, INTERFACE_PROPERTY_PRECEDENCE,
        SHARED_PROPERTY_PRECEDENCE, merge_effective_definitions,
    };

    fn definition(name: &str, source: &str, required: bool) -> EffectivePropertyDefinition {
        EffectivePropertyDefinition {
            name: name.to_string(),
            display_name: name.to_string(),
            description: String::new(),
            property_type: "string".to_string(),
            required,
            unique_constraint: false,
            time_dependent: false,
            default_value: None,
            validation_rules: None,
            source: source.to_string(),
        }
    }

    #[test]
    fn prefers_more_specific_property_source() {
        let merged = merge_effective_definitions([
            (
                SHARED_PROPERTY_PRECEDENCE,
                definition("status", "shared_property_type", false),
            ),
            (
                INTERFACE_PROPERTY_PRECEDENCE,
                definition("status", "interface", false),
            ),
            (
                DIRECT_PROPERTY_PRECEDENCE,
                definition("status", "object_type", true),
            ),
        ]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].source, "object_type");
        assert!(merged[0].required);
    }

    #[test]
    fn keeps_first_definition_for_same_precedence() {
        let merged = merge_effective_definitions([
            (
                SHARED_PROPERTY_PRECEDENCE,
                definition("priority", "shared_property_type", false),
            ),
            (
                SHARED_PROPERTY_PRECEDENCE,
                definition("priority", "shared_property_type", true),
            ),
        ]);

        assert_eq!(merged.len(), 1);
        assert!(!merged[0].required);
    }
}

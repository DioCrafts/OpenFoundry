use std::collections::{BTreeMap, HashSet};

use serde::Serialize;
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::{
    domain::type_system::validate_property_value,
    models::{interface::InterfaceProperty, property::Property},
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

pub async fn load_effective_properties(
    db: &sqlx::PgPool,
    object_type_id: Uuid,
) -> Result<Vec<EffectivePropertyDefinition>, sqlx::Error> {
    let direct = sqlx::query_as::<_, Property>(
        r#"SELECT id, object_type_id, name, display_name, description, property_type, required,
                  unique_constraint, time_dependent, default_value, validation_rules, created_at, updated_at
           FROM properties
           WHERE object_type_id = $1
           ORDER BY created_at ASC"#,
    )
    .bind(object_type_id)
    .fetch_all(db)
    .await?;

    let shared = sqlx::query_as::<_, InterfaceProperty>(
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

    let mut merged = BTreeMap::new();
    for property in shared {
        merged.entry(property.name.clone()).or_insert(EffectivePropertyDefinition {
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
        });
    }

    for property in direct {
        merged.insert(
            property.name.clone(),
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
        );
    }

    Ok(merged.into_values().collect())
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

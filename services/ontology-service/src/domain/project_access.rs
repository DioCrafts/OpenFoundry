use std::collections::{HashMap, HashSet};

use auth_middleware::Claims;
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::project::{OntologyProject, OntologyProjectRole};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OntologyResourceKind {
    ObjectType,
    LinkType,
    Interface,
    SharedPropertyType,
    ActionType,
    FunctionPackage,
    Rule,
    ObjectSet,
}

impl OntologyResourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ObjectType => "object_type",
            Self::LinkType => "link_type",
            Self::Interface => "interface",
            Self::SharedPropertyType => "shared_property_type",
            Self::ActionType => "action_type",
            Self::FunctionPackage => "function_package",
            Self::Rule => "rule",
            Self::ObjectSet => "object_set",
        }
    }

    fn table_name(self) -> &'static str {
        match self {
            Self::ObjectType => "object_types",
            Self::LinkType => "link_types",
            Self::Interface => "ontology_interfaces",
            Self::SharedPropertyType => "shared_property_types",
            Self::ActionType => "action_types",
            Self::FunctionPackage => "ontology_function_packages",
            Self::Rule => "ontology_rules",
            Self::ObjectSet => "ontology_object_sets",
        }
    }
}

impl TryFrom<&str> for OntologyResourceKind {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim() {
            "object_type" => Ok(Self::ObjectType),
            "link_type" => Ok(Self::LinkType),
            "interface" => Ok(Self::Interface),
            "shared_property_type" => Ok(Self::SharedPropertyType),
            "action_type" => Ok(Self::ActionType),
            "function_package" => Ok(Self::FunctionPackage),
            "rule" => Ok(Self::Rule),
            "object_set" => Ok(Self::ObjectSet),
            other => Err(format!(
                "resource_kind '{other}' is not supported; expected one of: object_type, link_type, interface, shared_property_type, action_type, function_package, rule, object_set"
            )),
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct ProjectAccessRow {
    id: Uuid,
    owner_id: Uuid,
    workspace_slug: Option<String>,
    membership_role: Option<OntologyProjectRole>,
}

#[derive(Debug, Clone, FromRow)]
struct ResourceProjectRow {
    resource_id: Uuid,
    project_id: Uuid,
}

pub fn claims_workspace_slug(claims: &Claims) -> Option<String> {
    claims
        .session_scope
        .as_ref()
        .and_then(|scope| scope.workspace.as_deref())
        .or_else(|| claims.attribute("workspace").and_then(Value::as_str))
        .or_else(|| claims.attribute("default_workspace").and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub async fn list_accessible_projects(
    db: &sqlx::PgPool,
    claims: &Claims,
) -> Result<HashMap<Uuid, OntologyProjectRole>, sqlx::Error> {
    if claims.has_role("admin") {
        let projects = sqlx::query_as::<_, OntologyProject>(
            r#"SELECT id, slug, display_name, description, workspace_slug, owner_id, created_at, updated_at
               FROM ontology_projects"#,
        )
        .fetch_all(db)
        .await?;
        return Ok(projects
            .into_iter()
            .map(|project| (project.id, OntologyProjectRole::Owner))
            .collect());
    }

    let workspace = claims_workspace_slug(claims);
    let rows = sqlx::query_as::<_, ProjectAccessRow>(
        r#"SELECT p.id,
                  p.owner_id,
                  p.workspace_slug,
                  m.role AS membership_role
           FROM ontology_projects p
           LEFT JOIN ontology_project_memberships m
                ON m.project_id = p.id AND m.user_id = $1"#,
    )
    .bind(claims.sub)
    .fetch_all(db)
    .await?;

    let mut accessible = HashMap::new();
    for row in rows {
        let role = if row.owner_id == claims.sub {
            Some(OntologyProjectRole::Owner)
        } else if let Some(role) = row.membership_role {
            Some(role)
        } else if row.workspace_slug.as_ref() == workspace.as_ref() {
            Some(OntologyProjectRole::Viewer)
        } else {
            None
        };

        if let Some(role) = role {
            accessible.insert(row.id, role);
        }
    }

    Ok(accessible)
}

pub async fn ensure_project_view_access(
    db: &sqlx::PgPool,
    claims: &Claims,
    project_id: Uuid,
) -> Result<OntologyProjectRole, String> {
    let accessible = list_accessible_projects(db, claims)
        .await
        .map_err(|error| format!("failed to evaluate project access: {error}"))?;
    accessible.get(&project_id).copied().ok_or_else(|| {
        "forbidden: current user cannot view resources in this ontology project".to_string()
    })
}

pub async fn ensure_project_edit_access(
    db: &sqlx::PgPool,
    claims: &Claims,
    project_id: Uuid,
) -> Result<OntologyProjectRole, String> {
    let role = ensure_project_view_access(db, claims, project_id).await?;
    if role.rank() >= OntologyProjectRole::Editor.rank() {
        Ok(role)
    } else {
        Err("forbidden: current user cannot edit resources in this ontology project".to_string())
    }
}

pub async fn load_resource_project_id(
    db: &sqlx::PgPool,
    resource_kind: OntologyResourceKind,
    resource_id: Uuid,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>(
        r#"SELECT project_id
           FROM ontology_project_resources
           WHERE resource_kind = $1 AND resource_id = $2"#,
    )
    .bind(resource_kind.as_str())
    .bind(resource_id)
    .fetch_optional(db)
    .await
}

pub async fn load_resource_project_map(
    db: &sqlx::PgPool,
    resource_kind: OntologyResourceKind,
    resource_ids: &[Uuid],
) -> Result<HashMap<Uuid, Uuid>, sqlx::Error> {
    if resource_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let wanted = resource_ids.iter().copied().collect::<HashSet<_>>();
    let rows = sqlx::query_as::<_, ResourceProjectRow>(
        r#"SELECT resource_id, project_id
           FROM ontology_project_resources
           WHERE resource_kind = $1"#,
    )
    .bind(resource_kind.as_str())
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .filter(|row| wanted.contains(&row.resource_id))
        .map(|row| (row.resource_id, row.project_id))
        .collect())
}

pub fn resource_is_visible(
    claims: &Claims,
    project_id: Option<Uuid>,
    accessible_projects: &HashMap<Uuid, OntologyProjectRole>,
) -> bool {
    if claims.has_role("admin") {
        return true;
    }

    match project_id {
        Some(project_id) => accessible_projects.contains_key(&project_id),
        None => true,
    }
}

pub async fn ensure_resource_view_access(
    db: &sqlx::PgPool,
    claims: &Claims,
    project_id: Option<Uuid>,
) -> Result<(), String> {
    if let Some(project_id) = project_id {
        ensure_project_view_access(db, claims, project_id).await?;
    }
    Ok(())
}

pub async fn ensure_resource_manage_access(
    db: &sqlx::PgPool,
    claims: &Claims,
    owner_id: Uuid,
    project_id: Option<Uuid>,
) -> Result<(), String> {
    if claims.has_role("admin") {
        return Ok(());
    }

    if let Some(project_id) = project_id {
        ensure_project_edit_access(db, claims, project_id).await?;
        return Ok(());
    }

    if owner_id == claims.sub {
        Ok(())
    } else {
        Err("forbidden: only the owner can modify an unscoped ontology resource".to_string())
    }
}

pub async fn load_resource_owner_id(
    db: &sqlx::PgPool,
    resource_kind: OntologyResourceKind,
    resource_id: Uuid,
) -> Result<Option<Uuid>, String> {
    let query = format!(
        "SELECT owner_id FROM {} WHERE id = $1",
        resource_kind.table_name()
    );
    sqlx::query_scalar::<_, Uuid>(&query)
        .bind(resource_id)
        .fetch_optional(db)
        .await
        .map_err(|error| format!("failed to load resource owner: {error}"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    use auth_middleware::claims::{Claims, SessionScope};

    use super::{OntologyResourceKind, claims_workspace_slug};

    fn workspace_claims() -> Claims {
        Claims {
            sub: Uuid::nil(),
            iat: 0,
            exp: i64::MAX,
            iss: None,
            aud: None,
            jti: Uuid::nil(),
            email: "user@example.com".to_string(),
            name: "User".to_string(),
            roles: vec!["viewer".to_string()],
            permissions: Vec::new(),
            org_id: None,
            attributes: json!({ "workspace": "finance-lab" }),
            auth_methods: Vec::new(),
            token_use: None,
            api_key_id: None,
            session_kind: None,
            session_scope: Some(SessionScope {
                workspace: Some("project-alpha".to_string()),
                ..Default::default()
            }),
        }
    }

    #[test]
    fn resource_kind_parser_accepts_supported_values() {
        assert_eq!(
            OntologyResourceKind::try_from("action_type").expect("kind"),
            OntologyResourceKind::ActionType
        );
        assert!(OntologyResourceKind::try_from("unknown").is_err());
    }

    #[test]
    fn workspace_slug_prefers_session_scope_over_attributes() {
        let claims = workspace_claims();
        assert_eq!(claims_workspace_slug(&claims).as_deref(), Some("project-alpha"));
    }
}

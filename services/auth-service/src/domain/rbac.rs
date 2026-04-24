use sqlx::PgPool;
use uuid::Uuid;

use crate::models::role::Role;

#[derive(Debug, Clone, Default)]
pub struct AccessBundle {
    pub roles: Vec<String>,
    pub groups: Vec<String>,
    pub permissions: Vec<String>,
}

pub async fn assign_role(pool: &PgPool, user_id: Uuid, role_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(user_id)
        .bind(role_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn remove_role(pool: &PgPool, user_id: Uuid, role_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM user_roles WHERE user_id = $1 AND role_id = $2")
        .bind(user_id)
        .bind(role_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_role_by_name(pool: &PgPool, name: &str) -> Result<Option<Role>, sqlx::Error> {
    sqlx::query_as::<_, Role>("SELECT id, name, description, created_at FROM roles WHERE name = $1")
        .bind(name)
        .fetch_optional(pool)
        .await
}

pub async fn get_user_access_bundle(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<AccessBundle, sqlx::Error> {
    let roles = sqlx::query_scalar::<_, String>(
        r#"SELECT DISTINCT name FROM (
               SELECT r.name AS name
               FROM roles r
               INNER JOIN user_roles ur ON ur.role_id = r.id
               WHERE ur.user_id = $1
               UNION
               SELECT r.name AS name
               FROM roles r
               INNER JOIN group_roles gr ON gr.role_id = r.id
               INNER JOIN group_members gm ON gm.group_id = gr.group_id
               WHERE gm.user_id = $1
           ) effective_roles
           ORDER BY name"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let groups = sqlx::query_scalar::<_, String>(
        r#"SELECT g.name
           FROM groups g
           INNER JOIN group_members gm ON gm.group_id = g.id
           WHERE gm.user_id = $1
           ORDER BY g.name"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let permissions = sqlx::query_scalar::<_, String>(
        r#"SELECT DISTINCT p.resource || ':' || p.action AS permission_key
           FROM permissions p
           INNER JOIN role_permissions rp ON rp.permission_id = p.id
           WHERE rp.role_id IN (
               SELECT ur.role_id
               FROM user_roles ur
               WHERE ur.user_id = $1
               UNION
               SELECT gr.role_id
               FROM group_roles gr
               INNER JOIN group_members gm ON gm.group_id = gr.group_id
               WHERE gm.user_id = $1
           )
           ORDER BY permission_key"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(AccessBundle {
        roles,
        groups,
        permissions,
    })
}

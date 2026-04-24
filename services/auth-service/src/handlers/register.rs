use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub id: Uuid,
    pub email: String,
    pub name: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> impl IntoResponse {
    // Check if email already exists
    let existing =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)")
            .bind(&body.email)
            .fetch_one(&state.db)
            .await;

    if matches!(existing, Ok(true)) {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": "email already registered" })),
        )
            .into_response();
    }

    let password_hash = match hash_password(&body.password) {
        Ok(h) => h,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "failed to hash password" })),
            )
                .into_response();
        }
    };

    let user_id = Uuid::now_v7();
    let result = sqlx::query(
        r#"INSERT INTO users (id, email, name, password_hash, is_active, auth_source)
              VALUES ($1, $2, $3, $4, true, 'local')"#,
    )
    .bind(user_id)
    .bind(&body.email)
    .bind(&body.name)
    .bind(&password_hash)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => {
            // Assign default 'viewer' role
            let _ = sqlx::query(
                r#"INSERT INTO user_roles (user_id, role_id)
                   SELECT $1, id FROM roles WHERE name = 'viewer'
                   ON CONFLICT DO NOTHING"#,
            )
            .bind(user_id)
            .execute(&state.db)
            .await;

            tracing::info!(user_id = %user_id, email = %body.email, "user registered");

            (
                StatusCode::CREATED,
                Json(serde_json::json!(RegisterResponse {
                    id: user_id,
                    email: body.email,
                    name: body.name,
                })),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("registration failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "registration failed" })),
            )
                .into_response()
        }
    }
}

fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    use argon2::password_hash::SaltString;
    use argon2::password_hash::rand_core::OsRng;
    use argon2::{Argon2, PasswordHasher};

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

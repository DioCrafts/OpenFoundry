// Gateway auth middleware — delegates to auth-middleware crate.
// Re-exported so gateway routes can use it via `crate::middleware::auth`.
pub use auth_middleware::layer::auth_layer;
pub use auth_middleware::layer::AuthUser;

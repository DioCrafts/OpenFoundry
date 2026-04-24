// Gateway auth middleware — delegates to auth-middleware crate.
// Re-exported so gateway routes can use it via `crate::middleware::auth`.
#[allow(unused_imports)]
pub use auth_middleware::layer::AuthUser;
#[allow(unused_imports)]
pub use auth_middleware::layer::auth_layer;

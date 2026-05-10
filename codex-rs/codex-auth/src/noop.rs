use std::future::Future;
use std::pin::Pin;
use std::sync::OnceLock;

use codex_wire_protocol::{AuthError, AuthToken, AuthType};

/// No-op authentication provider that always returns `AuthToken::None`.
pub struct NoOpAuthProvider {
    provider_id: String,
    auth_type_cache: OnceLock<AuthType>,
}

impl NoOpAuthProvider {
    pub fn new(provider_id: String) -> Self {
        Self {
            provider_id,
            auth_type_cache: OnceLock::new(),
        }
    }
}

impl codex_wire_protocol::AuthProvider for NoOpAuthProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn auth_type(&self) -> &AuthType {
        self.auth_type_cache.get_or_init(|| AuthType::None)
    }

    fn authenticate(&self) -> Pin<Box<dyn Future<Output = Result<AuthToken, AuthError>> + Send>> {
        Box::pin(async move { Ok(AuthToken::None) })
    }

    fn refresh(
        &self,
        _token: &AuthToken,
    ) -> Pin<Box<dyn Future<Output = Result<AuthToken, AuthError>> + Send>> {
        self.authenticate()
    }

    fn revoke(
        &self,
        _token: &AuthToken,
    ) -> Pin<Box<dyn Future<Output = Result<(), AuthError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }
}

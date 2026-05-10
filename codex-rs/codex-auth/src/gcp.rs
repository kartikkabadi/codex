use std::future::Future;
use std::pin::Pin;
use std::sync::OnceLock;

use codex_wire_protocol::{AuthError, AuthToken, AuthType};

/// Stub GCP authentication provider.
pub struct GcpAuthProvider {
    provider_id: String,
    auth_type_cache: OnceLock<AuthType>,
}

impl GcpAuthProvider {
    pub fn new(provider_id: String) -> Self {
        Self {
            provider_id,
            auth_type_cache: OnceLock::new(),
        }
    }
}

impl codex_wire_protocol::AuthProvider for GcpAuthProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn auth_type(&self) -> &AuthType {
        self.auth_type_cache.get_or_init(|| AuthType::GcpAuth)
    }

    fn authenticate(&self) -> Pin<Box<dyn Future<Output = Result<AuthToken, AuthError>> + Send>> {
        Box::pin(async move { Err(AuthError::Unsupported("GCP".to_string())) })
    }

    fn refresh(
        &self,
        _token: &AuthToken,
    ) -> Pin<Box<dyn Future<Output = Result<AuthToken, AuthError>> + Send>> {
        Box::pin(async move { Err(AuthError::Unsupported("GCP refresh".to_string())) })
    }

    fn revoke(
        &self,
        _token: &AuthToken,
    ) -> Pin<Box<dyn Future<Output = Result<(), AuthError>> + Send>> {
        Box::pin(async move { Err(AuthError::Unsupported("GCP revoke".to_string())) })
    }
}

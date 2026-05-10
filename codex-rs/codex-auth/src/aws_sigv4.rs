use std::future::Future;
use std::pin::Pin;

use codex_wire_protocol::{AuthError, AuthToken, AuthType};

/// Stub AWS SigV4 authentication provider.
pub struct AwsSigV4AuthProvider {
    provider_id: String,
}

impl AwsSigV4AuthProvider {
    pub fn new(provider_id: String) -> Self {
        Self { provider_id }
    }
}

impl codex_wire_protocol::AuthProvider for AwsSigV4AuthProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn auth_type(&self) -> &AuthType {
        leak_auth_type(AuthType::AwsSigV4)
    }

    fn authenticate(&self) -> Pin<Box<dyn Future<Output = Result<AuthToken, AuthError>> + Send>> {
        Box::pin(async move { Err(AuthError::Unsupported("AWS SigV4".to_string())) })
    }

    fn refresh(
        &self,
        _token: &AuthToken,
    ) -> Pin<Box<dyn Future<Output = Result<AuthToken, AuthError>> + Send>> {
        Box::pin(async move { Err(AuthError::Unsupported("AWS SigV4 refresh".to_string())) })
    }

    fn revoke(
        &self,
        _token: &AuthToken,
    ) -> Pin<Box<dyn Future<Output = Result<(), AuthError>> + Send>> {
        Box::pin(async move { Err(AuthError::Unsupported("AWS SigV4 revoke".to_string())) })
    }
}

fn leak_auth_type(auth_type: AuthType) -> &'static AuthType {
    Box::leak(Box::new(auth_type))
}

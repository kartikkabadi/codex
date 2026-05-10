use std::future::Future;
use std::pin::Pin;

use codex_wire_protocol::{AuthError, AuthToken, AuthType};

/// Stub OAuth2 authentication provider.
pub struct OAuth2AuthProvider {
    provider_id: String,
}

impl OAuth2AuthProvider {
    pub fn new(provider_id: String) -> Self {
        Self { provider_id }
    }
}

impl codex_wire_protocol::AuthProvider for OAuth2AuthProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn auth_type(&self) -> &AuthType {
        leak_auth_type(AuthType::OAuth2 {
            authorize_url: String::new(),
            token_url: String::new(),
            scopes: Vec::new(),
            device_code_url: None,
        })
    }

    fn authenticate(&self) -> Pin<Box<dyn Future<Output = Result<AuthToken, AuthError>> + Send>> {
        Box::pin(async move { Err(AuthError::Unsupported("OAuth2".to_string())) })
    }

    fn refresh(
        &self,
        _token: &AuthToken,
    ) -> Pin<Box<dyn Future<Output = Result<AuthToken, AuthError>> + Send>> {
        Box::pin(async move { Err(AuthError::Unsupported("OAuth2 refresh".to_string())) })
    }

    fn revoke(
        &self,
        _token: &AuthToken,
    ) -> Pin<Box<dyn Future<Output = Result<(), AuthError>> + Send>> {
        Box::pin(async move { Err(AuthError::Unsupported("OAuth2 revoke".to_string())) })
    }
}

fn leak_auth_type(auth_type: AuthType) -> &'static AuthType {
    Box::leak(Box::new(auth_type))
}

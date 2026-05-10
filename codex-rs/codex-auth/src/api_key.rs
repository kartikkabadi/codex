use std::env;
use std::future::Future;
use std::pin::Pin;
use std::sync::OnceLock;

use codex_wire_protocol::{AuthError, AuthToken, AuthType};
use tracing::warn;

/// Provider that reads an API key from an environment variable.
pub struct ApiKeyAuthProvider {
    provider_id: String,
    env_var: String,
    header_name: String,
    auth_type_cache: OnceLock<AuthType>,
}

impl ApiKeyAuthProvider {
    pub fn new(provider_id: String, env_var: String, header_name: String) -> Self {
        Self {
            provider_id,
            env_var,
            header_name,
            auth_type_cache: OnceLock::new(),
        }
    }
}

impl codex_wire_protocol::AuthProvider for ApiKeyAuthProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn auth_type(&self) -> &AuthType {
        self.auth_type_cache.get_or_init(|| AuthType::ApiKey {
            env_var: self.env_var.clone(),
            header_name: self.header_name.clone(),
        })
    }

    fn authenticate(&self) -> Pin<Box<dyn Future<Output = Result<AuthToken, AuthError>> + Send>> {
        let env_var = self.env_var.clone();
        let header_name = self.header_name.clone();
        Box::pin(async move {
            match env::var(&env_var) {
                Ok(key) => Ok(AuthToken::ApiKey { key, header_name }),
                Err(_) => Err(AuthError::MissingEnvVar { env_var }),
            }
        })
    }

    fn refresh(
        &self,
        _token: &AuthToken,
    ) -> Pin<Box<dyn Future<Output = Result<AuthToken, AuthError>> + Send>> {
        self.authenticate()
    }

    fn revoke(
        &self,
        token: &AuthToken,
    ) -> Pin<Box<dyn Future<Output = Result<(), AuthError>> + Send>> {
        let _ = token;
        warn!(provider_id = %self.provider_id, "revoke is a no-op for ApiKeyAuthProvider");
        Box::pin(async move { Ok(()) })
    }
}

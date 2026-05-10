use std::env;
use std::future::Future;
use std::pin::Pin;

use codex_wire_protocol::{AuthError, AuthToken, AuthType};
use tracing::warn;

/// Provider that reads an API key from an environment variable.
pub struct ApiKeyAuthProvider {
    provider_id: String,
    env_var: String,
    header_name: String,
}

impl ApiKeyAuthProvider {
    pub fn new(provider_id: String, env_var: String, header_name: String) -> Self {
        Self {
            provider_id,
            env_var,
            header_name,
        }
    }
}

impl codex_wire_protocol::AuthProvider for ApiKeyAuthProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn auth_type(&self) -> &AuthType {
        // Return a static reference constructed from our fields.
        // Since AuthType is owned and we need a reference, we use a thread-local
        // or we can return a new one each call via a Box leak pattern.
        // For simplicity and correctness in a dyn-compatible trait, we leak a Box.
        leak_auth_type(AuthType::ApiKey {
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

fn leak_auth_type(auth_type: AuthType) -> &'static AuthType {
    Box::leak(Box::new(auth_type))
}

// codex-wire-protocol/src/auth.rs

use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone)]
pub enum AuthToken {
    Bearer(String),
    ApiKey {
        key: String,
        header_name: String,
    },
    AwsSigV4 {
        access_key: String,
        secret_key: String,
        session_token: Option<String>,
        region: String,
        service: String,
    },
    GcpAuth {
        token: String,
    },
    None,
}

/// Describes what authentication a provider needs.
#[derive(Debug, Clone)]
pub enum AuthType {
    ApiKey {
        env_var: String,
        header_name: String,
    },
    OAuth2 {
        authorize_url: String,
        token_url: String,
        scopes: Vec<String>,
        device_code_url: Option<String>,
    },
    AwsSigV4,
    GcpAuth,
    None,
}

/// Errors that can occur during authentication operations.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("auth failed: {0}")]
    Failed(String),
    #[error("unsupported auth operation: {0}")]
    Unsupported(String),
    #[error("missing environment variable: {env_var}")]
    MissingEnvVar { env_var: String },
    #[error("oauth error: {0}")]
    OAuth(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Trait for authentication providers.
///
/// Designed to be dyn-compatible: all methods return `Pin<Box<dyn Future>>`.
pub trait AuthProvider: Send + Sync {
    /// Unique identifier for this provider.
    fn provider_id(&self) -> &str;

    /// Describes the kind of authentication this provider uses.
    fn auth_type(&self) -> &AuthType;

    /// Perform initial authentication.
    fn authenticate(&self) -> Pin<Box<dyn Future<Output = Result<AuthToken, AuthError>> + Send>>;

    /// Refresh an existing token.
    fn refresh(
        &self,
        token: &AuthToken,
    ) -> Pin<Box<dyn Future<Output = Result<AuthToken, AuthError>> + Send>>;

    /// Revoke an existing token.
    fn revoke(
        &self,
        token: &AuthToken,
    ) -> Pin<Box<dyn Future<Output = Result<(), AuthError>> + Send>>;
}

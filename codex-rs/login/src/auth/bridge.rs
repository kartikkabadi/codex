use std::future::Future;
use std::pin::Pin;

use codex_wire_protocol::auth::{AuthError, AuthProvider, AuthToken, AuthType};

use crate::auth::CodexAuth;

/// Bridge that wraps a [`CodexAuth`] and exposes it as a [`AuthProvider`].
///
/// This lets existing code that works with `CodexAuth` (used in 66+ locations)
/// interoperate with APIs that expect `dyn AuthProvider` without a full
/// refactor of every call site.
pub struct CodexAuthBridge {
    auth: CodexAuth,
}

impl CodexAuthBridge {
    pub fn new(auth: CodexAuth) -> Self {
        Self { auth }
    }
}

impl AuthProvider for CodexAuthBridge {
    fn provider_id(&self) -> &str {
        match &self.auth {
            CodexAuth::ApiKey(_) => "openai-api-key",
            CodexAuth::Chatgpt(_) => "openai-chatgpt",
            CodexAuth::ChatgptAuthTokens(_) => "openai-chatgpt-tokens",
            CodexAuth::AgentIdentity(_) => "openai-agent-identity",
        }
    }

    fn auth_type(&self) -> &AuthType {
        static OAUTH2: std::sync::OnceLock<AuthType> = std::sync::OnceLock::new();
        OAUTH2.get_or_init(|| AuthType::OAuth2 {
            authorize_url: "https://auth.openai.com/authorize".to_string(),
            token_url: "https://auth.openai.com/oauth/token".to_string(),
            scopes: vec!["codex".to_string()],
            device_code_url: None,
        })
    }

    fn authenticate(&self) -> Pin<Box<dyn Future<Output = Result<AuthToken, AuthError>> + Send>> {
        let auth = self.auth.clone();
        Box::pin(async move { codex_auth_to_auth_token(&auth) })
    }

    fn refresh(
        &self,
        _token: &AuthToken,
    ) -> Pin<Box<dyn Future<Output = Result<AuthToken, AuthError>> + Send>> {
        let auth = self.auth.clone();
        Box::pin(async move { codex_auth_to_auth_token(&auth) })
    }

    fn revoke(
        &self,
        _token: &AuthToken,
    ) -> Pin<Box<dyn Future<Output = Result<(), AuthError>> + Send>> {
        Box::pin(async move { Ok(()) })
    }
}

/// Convert a [`CodexAuth`] into an [`AuthToken`] using the same mapping rules
/// the rest of the codebase expects.
///
/// Mapping:
/// - `CodexAuth::ApiKey` -> `AuthToken::ApiKey`
/// - `CodexAuth::Chatgpt` -> `AuthToken::Bearer`
/// - `CodexAuth::ChatgptAuthTokens` -> `AuthToken::Bearer`
/// - `CodexAuth::AgentIdentity` -> `AuthToken::Bearer`
fn codex_auth_to_auth_token(auth: &CodexAuth) -> Result<AuthToken, AuthError> {
    match auth {
        CodexAuth::ApiKey(api_key_auth) => {
            let key = api_key_auth.api_key().to_string();
            Ok(AuthToken::ApiKey {
                key,
                header_name: "Authorization".to_string(),
            })
        }
        CodexAuth::Chatgpt(_) | CodexAuth::ChatgptAuthTokens(_) => {
            let token = auth.get_token().map_err(AuthError::Io)?;
            Ok(AuthToken::Bearer(token))
        }
        CodexAuth::AgentIdentity(agent_identity_auth) => {
            // AgentIdentity uses a signed agent assertion (JWT) as its bearer token.
            // We build it from the stored record using the agent-identity crate helper.
            let record = agent_identity_auth.record();
            let key = codex_agent_identity::AgentIdentityKey {
                agent_runtime_id: &record.agent_runtime_id,
                private_key_pkcs8_base64: &record.agent_private_key,
            };
            let target = codex_agent_identity::AgentTaskAuthorizationTarget {
                agent_runtime_id: &record.agent_runtime_id,
                task_id: agent_identity_auth.process_task_id(),
            };
            let token = codex_agent_identity::authorization_header_for_agent_task(key, target)
                .map_err(|e| AuthError::Failed(e.to_string()))?;
            Ok(AuthToken::Bearer(token))
        }
    }
}

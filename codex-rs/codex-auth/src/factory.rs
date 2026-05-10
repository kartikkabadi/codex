use codex_wire_protocol::AuthType;

use crate::api_key::ApiKeyAuthProvider;
use crate::aws_sigv4::AwsSigV4AuthProvider;
use crate::gcp::GcpAuthProvider;
use crate::noop::NoOpAuthProvider;
use crate::oauth2::OAuth2AuthProvider;

/// Create a boxed [`codex_wire_protocol::AuthProvider`] for the given
/// `provider_id` and `auth_type`.
pub fn create_auth_provider(
    provider_id: &str,
    auth_type: AuthType,
) -> Box<dyn codex_wire_protocol::AuthProvider> {
    match auth_type {
        AuthType::ApiKey {
            env_var,
            header_name,
        } => Box::new(ApiKeyAuthProvider::new(
            provider_id.to_string(),
            env_var,
            header_name,
        )),
        AuthType::OAuth2 { .. } => Box::new(OAuth2AuthProvider::new(provider_id.to_string())),
        AuthType::AwsSigV4 => Box::new(AwsSigV4AuthProvider::new(provider_id.to_string())),
        AuthType::GcpAuth => Box::new(GcpAuthProvider::new(provider_id.to_string())),
        AuthType::None => Box::new(NoOpAuthProvider::new(provider_id.to_string())),
    }
}

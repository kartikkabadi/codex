//! End-to-end integration tests for multi-protocol provider dispatch.
//!
//! These tests verify that `ModelClient` can be constructed with non-OpenAI
//! providers (Anthropic, Gemini) and that `codex_api::create_protocol` dispatches
//! to the correct wire-protocol adapter without performing real network I/O.

use codex_api::ApiError;
use codex_api::create_protocol;
use codex_model_provider_info::ModelProviderInfo;
use codex_model_provider_info::WireApi;
use codex_protocol::SessionId;
use codex_protocol::ThreadId;
use codex_protocol::protocol::SessionSource;

use crate::ModelClient;

fn anthropic_provider_info() -> ModelProviderInfo {
    ModelProviderInfo {
        name: "Anthropic".into(),
        base_url: Some("https://api.anthropic.com/v1".into()),
        env_key: Some("ANTHROPIC_API_KEY".into()),
        env_key_instructions: None,
        experimental_bearer_token: None,
        auth: None,
        aws: None,
        wire_api: WireApi::Responses,
        query_params: None,
        http_headers: None,
        env_http_headers: None,
        request_max_retries: None,
        stream_max_retries: None,
        stream_idle_timeout_ms: None,
        websocket_connect_timeout_ms: None,
        requires_openai_auth: false,
        supports_websockets: false,
        protocol_id: "anthropic-messages".into(),
    }
}

fn gemini_provider_info() -> ModelProviderInfo {
    ModelProviderInfo {
        name: "Gemini".into(),
        base_url: Some("https://generativelanguage.googleapis.com/v1beta".into()),
        env_key: Some("GOOGLE_API_KEY".into()),
        env_key_instructions: None,
        experimental_bearer_token: None,
        auth: None,
        aws: None,
        wire_api: WireApi::Responses,
        query_params: None,
        http_headers: None,
        env_http_headers: None,
        request_max_retries: None,
        stream_max_retries: None,
        stream_idle_timeout_ms: None,
        websocket_connect_timeout_ms: None,
        requires_openai_auth: false,
        supports_websockets: false,
        protocol_id: "gemini".into(),
    }
}

fn make_model_client(provider_info: ModelProviderInfo) -> ModelClient {
    let thread_id = ThreadId::new();
    ModelClient::new(
        /*auth_manager*/ None,
        SessionId::from(thread_id),
        thread_id,
        /*installation_id*/ "11111111-1111-4111-8111-111111111111".to_string(),
        provider_info,
        SessionSource::Cli,
        /*model_verbosity*/ None,
        /*enable_request_compression*/ false,
        /*include_timing_metrics*/ false,
        /*beta_features_header*/ None,
        /*attestation_provider*/ None,
    )
}

#[test]
fn test_model_client_with_anthropic_provider() {
    let provider = anthropic_provider_info();
    let client = make_model_client(provider.clone());

    // Verify the provider metadata is preserved inside ModelClient.
    // We access it indirectly through the provider's info() method.
    let info = client.provider_info();
    assert_eq!(info.protocol_id, "anthropic-messages");
    assert_eq!(info.name, "Anthropic");
    assert!(!info.requires_openai_auth);
}

#[test]
fn test_model_client_with_gemini_provider() {
    let provider = gemini_provider_info();
    let client = make_model_client(provider.clone());

    let info = client.provider_info();
    assert_eq!(info.protocol_id, "gemini");
    assert_eq!(info.name, "Gemini");
    assert!(!info.requires_openai_auth);
}

#[test]
fn test_create_protocol_dispatches_to_anthropic() {
    let protocol = create_protocol(
        "anthropic-messages",
        "https://api.anthropic.com/v1".to_string(),
    );
    assert!(protocol.is_ok());
    let protocol = protocol.unwrap();
    // The protocol should report capabilities consistent with Anthropic.
    let caps = protocol.capabilities();
    // Anthropic does not support native compaction in this adapter.
    assert!(!caps.supports_compaction);
}

#[test]
fn test_create_protocol_dispatches_to_gemini() {
    let protocol = create_protocol(
        "gemini",
        "https://generativelanguage.googleapis.com/v1beta".to_string(),
    );
    assert!(protocol.is_ok());
    let protocol = protocol.unwrap();
    let caps = protocol.capabilities();
    // Gemini adapter does not support native compaction.
    assert!(!caps.supports_compaction);
}

#[test]
fn test_unknown_protocol_returns_error() {
    let result = create_protocol("unknown", "https://example.com".to_string());
    assert!(result.is_err());
    let err = result.err().unwrap();
    match err {
        ApiError::UnsupportedProtocol(id) => assert_eq!(id, "unknown"),
        other => panic!("expected UnsupportedProtocol, got: {other:?}"),
    }
}

// codex-wire-protocol-anthropic/src/models.rs

use codex_wire_protocol::response::ModelInfo;

/// Return a hardcoded list of known Anthropic Claude models.
pub fn list_anthropic_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "claude-sonnet-4-20250514".to_string(),
            display_name: "Claude Sonnet 4".to_string(),
            context_window: 200_000,
            supports_tools: true,
            supports_vision: true,
            supports_reasoning: false,
            supports_realtime: false,
            max_output_tokens: Some(8192),
        },
        ModelInfo {
            id: "claude-opus-4-20250514".to_string(),
            display_name: "Claude Opus 4".to_string(),
            context_window: 200_000,
            supports_tools: true,
            supports_vision: true,
            supports_reasoning: true,
            supports_realtime: false,
            max_output_tokens: Some(8192),
        },
        ModelInfo {
            id: "claude-sonnet-4-20250514-mini".to_string(),
            display_name: "Claude Sonnet 4 Mini".to_string(),
            context_window: 200_000,
            supports_tools: true,
            supports_vision: true,
            supports_reasoning: false,
            supports_realtime: false,
            max_output_tokens: Some(8192),
        },
        ModelInfo {
            id: "claude-3-5-sonnet-20241022".to_string(),
            display_name: "Claude 3.5 Sonnet".to_string(),
            context_window: 200_000,
            supports_tools: true,
            supports_vision: true,
            supports_reasoning: false,
            supports_realtime: false,
            max_output_tokens: Some(8192),
        },
        ModelInfo {
            id: "claude-3-5-haiku-20241022".to_string(),
            display_name: "Claude 3.5 Haiku".to_string(),
            context_window: 200_000,
            supports_tools: true,
            supports_vision: false,
            supports_reasoning: false,
            supports_realtime: false,
            max_output_tokens: Some(4096),
        },
        ModelInfo {
            id: "claude-3-haiku-20240307".to_string(),
            display_name: "Claude 3 Haiku".to_string(),
            context_window: 200_000,
            supports_tools: true,
            supports_vision: false,
            supports_reasoning: false,
            supports_realtime: false,
            max_output_tokens: Some(4096),
        },
    ]
}

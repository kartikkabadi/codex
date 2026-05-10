// codex-wire-protocol-gemini/src/models.rs

use codex_wire_protocol::response::ModelInfo;

/// Return a hardcoded list of known Gemini models.
pub fn list_gemini_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "gemini-2.5-pro-preview-03-25".to_string(),
            display_name: "Gemini 2.5 Pro Preview".to_string(),
            context_window: 1_048_576,
            supports_tools: true,
            supports_vision: true,
            supports_reasoning: true,
            supports_realtime: false,
            max_output_tokens: Some(65_536),
        },
        ModelInfo {
            id: "gemini-2.5-flash-preview-04-17".to_string(),
            display_name: "Gemini 2.5 Flash Preview".to_string(),
            context_window: 1_048_576,
            supports_tools: true,
            supports_vision: true,
            supports_reasoning: true,
            supports_realtime: false,
            max_output_tokens: Some(65_536),
        },
        ModelInfo {
            id: "gemini-2.0-flash".to_string(),
            display_name: "Gemini 2.0 Flash".to_string(),
            context_window: 1_048_576,
            supports_tools: true,
            supports_vision: true,
            supports_reasoning: false,
            supports_realtime: false,
            max_output_tokens: Some(8_192),
        },
        ModelInfo {
            id: "gemini-2.0-flash-lite".to_string(),
            display_name: "Gemini 2.0 Flash Lite".to_string(),
            context_window: 1_048_576,
            supports_tools: true,
            supports_vision: true,
            supports_reasoning: false,
            supports_realtime: false,
            max_output_tokens: Some(8_192),
        },
        ModelInfo {
            id: "gemini-1.5-pro".to_string(),
            display_name: "Gemini 1.5 Pro".to_string(),
            context_window: 2_097_152,
            supports_tools: true,
            supports_vision: true,
            supports_reasoning: false,
            supports_realtime: false,
            max_output_tokens: Some(8_192),
        },
        ModelInfo {
            id: "gemini-1.5-flash".to_string(),
            display_name: "Gemini 1.5 Flash".to_string(),
            context_window: 1_048_576,
            supports_tools: true,
            supports_vision: true,
            supports_reasoning: false,
            supports_realtime: false,
            max_output_tokens: Some(8_192),
        },
    ]
}

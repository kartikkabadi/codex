use std::fmt;

use codex_protocol::openai_models::ModelInfo;

use crate::catalog::ModelCatalog;
use crate::model_info;

pub struct AnthropicModelCatalog;

impl fmt::Debug for AnthropicModelCatalog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnthropicModelCatalog").finish()
    }
}

impl Default for AnthropicModelCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl AnthropicModelCatalog {
    pub fn new() -> Self {
        Self
    }
}

impl ModelCatalog for AnthropicModelCatalog {
    fn provider_id(&self) -> &str {
        "anthropic"
    }

    fn bundled_models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                slug: "claude-sonnet-4-20250514".to_string(),
                display_name: "Claude Sonnet 4".to_string(),
                description: Some("Balanced performance and cost".to_string()),
                priority: 1,
                ..model_info::model_info_from_slug("claude-sonnet-4-20250514")
            },
            ModelInfo {
                slug: "claude-opus-4-20250514".to_string(),
                display_name: "Claude Opus 4".to_string(),
                description: Some("Highest capability model".to_string()),
                priority: 2,
                ..model_info::model_info_from_slug("claude-opus-4-20250514")
            },
            ModelInfo {
                slug: "claude-sonnet-4-20250514-mini".to_string(),
                display_name: "Claude Sonnet 4 Mini".to_string(),
                description: Some("Fast, lightweight model".to_string()),
                priority: 3,
                ..model_info::model_info_from_slug("claude-sonnet-4-20250514-mini")
            },
            ModelInfo {
                slug: "claude-3-7-sonnet".to_string(),
                display_name: "Claude 3.7 Sonnet".to_string(),
                description: Some("Prior generation Sonnet".to_string()),
                priority: 4,
                ..model_info::model_info_from_slug("claude-3-7-sonnet")
            },
            ModelInfo {
                slug: "claude-3-5-haiku".to_string(),
                display_name: "Claude 3.5 Haiku".to_string(),
                description: Some("Fast, compact model".to_string()),
                priority: 5,
                ..model_info::model_info_from_slug("claude-3-5-haiku")
            },
        ]
    }

    fn base_instructions(&self) -> Option<&str> {
        Some(
            "You are Claude, an AI assistant by Anthropic. Collaborate with the user in their workspace to achieve their goals.",
        )
    }
}

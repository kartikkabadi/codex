use std::fmt;

use codex_protocol::openai_models::ModelInfo;

use crate::catalog::ModelCatalog;
use crate::model_info;

pub struct OpenAiModelCatalog;

impl fmt::Debug for OpenAiModelCatalog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAiModelCatalog").finish()
    }
}

impl Default for OpenAiModelCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenAiModelCatalog {
    pub fn new() -> Self {
        Self
    }
}

impl ModelCatalog for OpenAiModelCatalog {
    fn provider_id(&self) -> &str {
        "openai"
    }

    fn bundled_models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                slug: "gpt-5.2-codex".to_string(),
                display_name: "GPT-5.2 Codex".to_string(),
                description: Some("OpenAI's flagship coding model".to_string()),
                priority: 1,
                ..model_info::model_info_from_slug("gpt-5.2-codex")
            },
            ModelInfo {
                slug: "gpt-5".to_string(),
                display_name: "GPT-5".to_string(),
                description: Some("General-purpose reasoning model".to_string()),
                priority: 2,
                ..model_info::model_info_from_slug("gpt-5")
            },
            ModelInfo {
                slug: "gpt-5-mini".to_string(),
                display_name: "GPT-5 Mini".to_string(),
                description: Some("Fast, cost-effective model".to_string()),
                priority: 3,
                ..model_info::model_info_from_slug("gpt-5-mini")
            },
            ModelInfo {
                slug: "o4-mini".to_string(),
                display_name: "o4 Mini".to_string(),
                description: Some("Reasoning-optimized model".to_string()),
                priority: 4,
                ..model_info::model_info_from_slug("o4-mini")
            },
            ModelInfo {
                slug: "o3".to_string(),
                display_name: "o3".to_string(),
                description: Some("High-capability reasoning model".to_string()),
                priority: 5,
                ..model_info::model_info_from_slug("o3")
            },
        ]
    }

    fn base_instructions(&self) -> Option<&str> {
        Some(
            "You are Codex, a coding agent by OpenAI. Work collaboratively with the user in their workspace.",
        )
    }
}

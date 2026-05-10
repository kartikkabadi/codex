use std::fmt;

use codex_protocol::openai_models::ModelInfo;

use crate::catalog::ModelCatalog;
use crate::model_info;

pub struct GeminiModelCatalog;

impl fmt::Debug for GeminiModelCatalog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GeminiModelCatalog").finish()
    }
}

impl Default for GeminiModelCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl GeminiModelCatalog {
    pub fn new() -> Self {
        Self
    }
}

impl ModelCatalog for GeminiModelCatalog {
    fn provider_id(&self) -> &str {
        "gemini"
    }

    fn bundled_models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                slug: "gemini-2.5-pro".to_string(),
                display_name: "Gemini 2.5 Pro".to_string(),
                description: Some("Google's advanced reasoning model".to_string()),
                priority: 1,
                ..model_info::model_info_from_slug("gemini-2.5-pro")
            },
            ModelInfo {
                slug: "gemini-2.5-flash".to_string(),
                display_name: "Gemini 2.5 Flash".to_string(),
                description: Some("Fast multimodal model".to_string()),
                priority: 2,
                ..model_info::model_info_from_slug("gemini-2.5-flash")
            },
            ModelInfo {
                slug: "gemini-2.0-flash".to_string(),
                display_name: "Gemini 2.0 Flash".to_string(),
                description: Some("Efficient general-purpose model".to_string()),
                priority: 3,
                ..model_info::model_info_from_slug("gemini-2.0-flash")
            },
            ModelInfo {
                slug: "gemini-2.0-pro".to_string(),
                display_name: "Gemini 2.0 Pro".to_string(),
                description: Some("Prior generation Pro model".to_string()),
                priority: 4,
                ..model_info::model_info_from_slug("gemini-2.0-pro")
            },
            ModelInfo {
                slug: "gemini-1.5-flash".to_string(),
                display_name: "Gemini 1.5 Flash".to_string(),
                description: Some("Lightweight long-context model".to_string()),
                priority: 5,
                ..model_info::model_info_from_slug("gemini-1.5-flash")
            },
        ]
    }

    fn base_instructions(&self) -> Option<&str> {
        Some(
            "You are Gemini, an AI assistant by Google. Work with the user in their workspace to help them build and ship software.",
        )
    }
}

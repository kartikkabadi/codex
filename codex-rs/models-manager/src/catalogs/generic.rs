use std::fmt;

use codex_protocol::openai_models::ModelInfo;

use crate::catalog::ModelCatalog;

pub struct GenericModelCatalog {
    provider: String,
}

impl fmt::Debug for GenericModelCatalog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GenericModelCatalog")
            .field("provider", &self.provider)
            .finish()
    }
}

impl GenericModelCatalog {
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
        }
    }
}

impl ModelCatalog for GenericModelCatalog {
    fn provider_id(&self) -> &str {
        &self.provider
    }

    fn bundled_models(&self) -> Vec<ModelInfo> {
        Vec::new()
    }

    fn base_instructions(&self) -> Option<&str> {
        None
    }
}

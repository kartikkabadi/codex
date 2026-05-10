use std::fmt;

use codex_protocol::openai_models::ModelInfo;

pub trait ModelCatalog: fmt::Debug + Send + Sync {
    fn provider_id(&self) -> &str;
    fn bundled_models(&self) -> Vec<ModelInfo>;
    fn base_instructions(&self) -> Option<&str>;
}

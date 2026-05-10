// codex-wire-protocol/src/response.rs

use serde::{Deserialize, Serialize};

/// A stream of turn events from the provider.
pub struct TurnStream {
    pub events: tokio::sync::mpsc::Receiver<TurnEvent>,
    pub turn_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TurnEvent {
    Started {
        turn_id: String,
    },
    ContentDelta {
        text: String,
    },
    ReasoningDelta {
        text: String,
    },
    ToolCallStarted {
        call_id: String,
        name: String,
    },
    ToolCallArgumentsDelta {
        call_id: String,
        arguments: String,
    },
    ToolCallCompleted {
        call_id: String,
        name: String,
        arguments: String,
    },
    ToolResult {
        call_id: String,
        output: String,
    },
    Completed,
    Failed {
        error: String,
        code: Option<String>,
    },
    Usage {
        input_tokens: u32,
        output_tokens: u32,
        reasoning_tokens: Option<u32>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub context_window: u64,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub supports_reasoning: bool,
    pub supports_realtime: bool,
    pub max_output_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactResponse {
    pub summary: String,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

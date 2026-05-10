// codex-wire-protocol/src/capabilities.rs

#[derive(Debug, Clone, Copy, Default)]
pub struct ProtocolCapabilities {
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub supports_reasoning: bool,
    pub supports_realtime: bool,
    pub supports_compaction: bool,
    pub supports_websockets: bool,
    pub supports_parallel_tool_calls: bool,
}

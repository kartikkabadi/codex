// codex-wire-protocol-openai/src/request.rs

use codex_wire_protocol::request::{
    ReasoningConfig, ReasoningEffort, ToolChoice, ToolSpec, TurnItem, TurnRequest,
};

use crate::types::{Reasoning, ResponsesApiRequest};

/// Convert a generic `TurnRequest` into an OpenAI `ResponsesApiRequest`.
pub fn turn_request_to_responses_api_request(req: TurnRequest) -> ResponsesApiRequest {
    ResponsesApiRequest {
        model: req.model,
        instructions: req.instructions,
        input: req
            .input
            .into_iter()
            .map(map_turn_item)
            .collect(),
        tools: req
            .tools
            .into_iter()
            .map(map_tool_spec)
            .collect(),
        tool_choice: map_tool_choice(req.tool_choice),
        parallel_tool_calls: false,
        reasoning: req.reasoning.map(map_reasoning_config),
        store: true,
        stream: req.stream,
        include: vec!["reasoning".to_string()],
        service_tier: None,
        prompt_cache_key: None,
        text: None,
        client_metadata: req.metadata,
    }
}

fn map_turn_item(item: TurnItem) -> codex_protocol::models::ResponseItem {
    match item {
        TurnItem::Message { role, content } => codex_protocol::models::ResponseItem::Message {
            id: None,
            role: map_role(role),
            content: vec![codex_protocol::models::ContentItem::OutputText {
                text: content,
            }],
            phase: None,
        },
        TurnItem::Image { mime_type, data } => codex_protocol::models::ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![codex_protocol::models::ContentItem::InputImage {
                image_url: format!("data:{mime_type};base64,{data}"),
                detail: None,
            }],
            phase: None,
        },
        TurnItem::File { name, content } => codex_protocol::models::ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![codex_protocol::models::ContentItem::InputText {
                text: format!("File '{name}':\n{content}"),
            }],
            phase: None,
        },
    }
}

fn map_role(role: codex_wire_protocol::request::Role) -> String {
    match role {
        codex_wire_protocol::request::Role::System => "system".to_string(),
        codex_wire_protocol::request::Role::User => "user".to_string(),
        codex_wire_protocol::request::Role::Assistant => "assistant".to_string(),
    }
}

fn map_tool_spec(spec: ToolSpec) -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": spec.name,
            "description": spec.description,
            "parameters": spec.parameters,
        }
    })
}

fn map_tool_choice(choice: ToolChoice) -> String {
    match choice {
        ToolChoice::Auto => "auto".to_string(),
        ToolChoice::Required => "required".to_string(),
        ToolChoice::None => "none".to_string(),
        ToolChoice::Named(name) => name,
    }
}

fn map_reasoning_config(cfg: ReasoningConfig) -> Reasoning {
    Reasoning {
        effort: Some(match cfg.effort {
            ReasoningEffort::Low => codex_protocol::openai_models::ReasoningEffort::Low,
            ReasoningEffort::Medium => codex_protocol::openai_models::ReasoningEffort::Medium,
            ReasoningEffort::High => codex_protocol::openai_models::ReasoningEffort::High,
        }),
        summary: None,
    }
}

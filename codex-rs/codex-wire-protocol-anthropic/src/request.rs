// codex-wire-protocol-anthropic/src/request.rs

use codex_wire_protocol::request::Role;
use codex_wire_protocol::request::ToolChoice;
use codex_wire_protocol::request::ToolSpec;
use codex_wire_protocol::request::TurnItem;
use codex_wire_protocol::request::TurnRequest;

use crate::types::Content;
use crate::types::Message;
use crate::types::MessagesRequest;
use crate::types::Tool;

/// Convert a generic `TurnRequest` into an Anthropic `MessagesRequest`.
pub fn turn_request_to_messages_request(req: TurnRequest) -> MessagesRequest {
    let mut system: Option<String> = None;
    let mut messages = Vec::new();

    for item in req.input {
        match item {
            TurnItem::Message { role, content } => {
                if matches!(role, Role::System) {
                    system = Some(content);
                } else {
                    messages.push(Message {
                        role: map_role(role),
                        content: vec![Content::Text { text: content }],
                    });
                }
            }
            TurnItem::Image { mime_type, data } => {
                messages.push(Message {
                    role: "user".to_string(),
                    content: vec![Content::Image {
                        source: crate::types::ImageSource {
                            source_type: "base64".to_string(),
                            media_type: mime_type,
                            data,
                        },
                    }],
                });
            }
            TurnItem::File { name, content } => {
                messages.push(Message {
                    role: "user".to_string(),
                    content: vec![Content::Text {
                        text: format!("File '{name}':\n{content}"),
                    }],
                });
            }
        }
    }

    let tools = req.tools.into_iter().map(map_tool_spec).collect();
    let tool_choice = map_tool_choice(req.tool_choice);

    MessagesRequest {
        model: req.model,
        max_tokens: req.max_tokens.unwrap_or(4096),
        system,
        messages,
        tools,
        tool_choice,
        temperature: req.temperature,
        top_p: req.top_p,
        stop_sequences: req.stop_sequences,
        stream: req.stream,
        metadata: req.metadata.map(|m| {
            m.into_iter()
                .map(|(k, v)| (k, serde_json::Value::String(v)))
                .collect()
        }),
    }
}

fn map_role(role: Role) -> String {
    match role {
        Role::System => "system".to_string(),
        Role::User => "user".to_string(),
        Role::Assistant => "assistant".to_string(),
    }
}

fn map_tool_spec(spec: ToolSpec) -> Tool {
    Tool {
        name: spec.name,
        description: Some(spec.description),
        input_schema: spec.parameters,
    }
}

fn map_tool_choice(choice: ToolChoice) -> Option<crate::types::ToolChoice> {
    match choice {
        ToolChoice::Auto => Some(crate::types::ToolChoice::Auto),
        ToolChoice::Required => Some(crate::types::ToolChoice::Any),
        ToolChoice::None => Some(crate::types::ToolChoice::None),
        ToolChoice::Named(name) => Some(crate::types::ToolChoice::Tool { name }),
    }
}

// codex-wire-protocol-gemini/src/request.rs

use codex_wire_protocol::request::Role;
use codex_wire_protocol::request::ToolChoice;
use codex_wire_protocol::request::ToolSpec;
use codex_wire_protocol::request::TurnItem;
use codex_wire_protocol::request::TurnRequest;

use crate::types::Blob;
use crate::types::Content;
use crate::types::FunctionCallingConfig;
use crate::types::FunctionCallingMode;
use crate::types::FunctionDeclaration;
use crate::types::GenerateContentRequest;
use crate::types::GenerationConfig;
use crate::types::Part;
use crate::types::Tool;
use crate::types::ToolConfig;

/// Convert a generic `TurnRequest` into a Gemini `GenerateContentRequest`.
pub fn turn_request_to_gemini_request(req: TurnRequest) -> GenerateContentRequest {
    let mut system_instruction: Option<Content> = None;
    let mut contents = Vec::new();

    for item in req.input {
        match item {
            TurnItem::Message { role, content } => {
                if matches!(role, Role::System) {
                    system_instruction = Some(Content {
                        role: None,
                        parts: vec![Part::Text { text: content }],
                    });
                } else {
                    contents.push(Content {
                        role: Some(map_role(role)),
                        parts: vec![Part::Text { text: content }],
                    });
                }
            }
            TurnItem::Image { mime_type, data } => {
                contents.push(Content {
                    role: Some("user".to_string()),
                    parts: vec![Part::InlineData {
                        inline_data: Blob { mime_type, data },
                    }],
                });
            }
            TurnItem::File { name, content } => {
                contents.push(Content {
                    role: Some("user".to_string()),
                    parts: vec![Part::Text {
                        text: format!("File '{name}':\n{content}"),
                    }],
                });
            }
        }
    }

    let tools = req.tools.into_iter().map(map_tool_spec).collect();
    let tool_config = map_tool_choice(req.tool_choice);

    let generation_config = Some(GenerationConfig {
        temperature: req.temperature,
        top_p: req.top_p,
        max_output_tokens: req.max_tokens,
        stop_sequences: req.stop_sequences,
    });

    GenerateContentRequest {
        model: req.model,
        system_instruction,
        contents,
        tools,
        tool_config,
        generation_config,
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
        function_declarations: vec![FunctionDeclaration {
            name: spec.name,
            description: Some(spec.description),
            parameters: spec.parameters,
        }],
    }
}

fn map_tool_choice(choice: ToolChoice) -> Option<ToolConfig> {
    let mode = match choice {
        ToolChoice::Auto => FunctionCallingMode::Auto,
        ToolChoice::Required => FunctionCallingMode::Any,
        ToolChoice::None => FunctionCallingMode::None,
        ToolChoice::Named(name) => {
            return Some(ToolConfig {
                function_calling_config: FunctionCallingConfig {
                    mode: FunctionCallingMode::Any,
                    allowed_function_names: vec![name],
                },
            });
        }
    };
    Some(ToolConfig {
        function_calling_config: FunctionCallingConfig {
            mode,
            allowed_function_names: Vec::new(),
        },
    })
}

use std::collections::HashMap;

use codex_login::CodexAuth;
use codex_model_provider_info::ModelProviderInfo;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ReasoningEffort as ReasoningEffortConfig;
use codex_tools::ToolSpec as CodexToolSpec;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::client::X_CODEX_INSTALLATION_ID_HEADER;
use crate::client_common::Prompt;
use crate::client_common::ResponseEvent;
use crate::client_common::ResponseStream;

const RESPONSE_STREAM_CHANNEL_CAPACITY: usize = 1600;

pub(crate) fn supported_protocol_id(protocol_id: &str) -> bool {
    matches!(protocol_id, "anthropic-messages" | "gemini")
}

pub(crate) fn build_turn_request(
    prompt: &Prompt,
    model_info: &ModelInfo,
    effort: Option<ReasoningEffortConfig>,
    installation_id: &str,
) -> Result<codex_wire_protocol::TurnRequest> {
    let mut input = Vec::new();
    if !prompt.base_instructions.text.is_empty() {
        input.push(codex_wire_protocol::TurnItem::Message {
            role: codex_wire_protocol::Role::System,
            content: prompt.base_instructions.text.clone(),
        });
    }
    input.extend(
        prompt
            .get_formatted_input()
            .into_iter()
            .filter_map(response_item_to_turn_item),
    );

    let tools = prompt
        .tools
        .iter()
        .filter_map(tool_spec_to_wire_tool_spec)
        .collect();
    let reasoning = effort.map(|effort| codex_wire_protocol::ReasoningConfig {
        effort: match effort {
            ReasoningEffortConfig::Low => codex_wire_protocol::ReasoningEffort::Low,
            ReasoningEffortConfig::Medium => codex_wire_protocol::ReasoningEffort::Medium,
            ReasoningEffortConfig::High => codex_wire_protocol::ReasoningEffort::High,
            ReasoningEffortConfig::None | ReasoningEffortConfig::Minimal => {
                codex_wire_protocol::ReasoningEffort::Low
            }
            ReasoningEffortConfig::XHigh => codex_wire_protocol::ReasoningEffort::High,
        },
    });

    Ok(codex_wire_protocol::TurnRequest {
        model: model_info.slug.clone(),
        instructions: prompt.base_instructions.text.clone(),
        input,
        tools,
        tool_choice: codex_wire_protocol::ToolChoice::Auto,
        reasoning,
        stream: true,
        temperature: None,
        max_tokens: None,
        top_p: None,
        stop_sequences: None,
        metadata: Some(HashMap::from([(
            X_CODEX_INSTALLATION_ID_HEADER.to_string(),
            installation_id.to_string(),
        )])),
    })
}

fn response_item_to_turn_item(item: ResponseItem) -> Option<codex_wire_protocol::TurnItem> {
    match item {
        ResponseItem::Message { role, content, .. } => {
            let text = content_items_to_plain_text(&content);
            (!text.is_empty()).then_some(codex_wire_protocol::TurnItem::Message {
                role: match role.as_str() {
                    "assistant" => codex_wire_protocol::Role::Assistant,
                    "system" => codex_wire_protocol::Role::System,
                    _ => codex_wire_protocol::Role::User,
                },
                content: text,
            })
        }
        ResponseItem::FunctionCallOutput {
            call_id, output, ..
        }
        | ResponseItem::CustomToolCallOutput {
            call_id, output, ..
        } => Some(codex_wire_protocol::TurnItem::Message {
            role: codex_wire_protocol::Role::User,
            content: format!("Tool result for {call_id}:\n{output}"),
        }),
        _ => None,
    }
}

fn content_items_to_plain_text(items: &[ContentItem]) -> String {
    items
        .iter()
        .filter_map(|item| match item {
            ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                Some(text.as_str())
            }
            ContentItem::InputImage { .. } => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn tool_spec_to_wire_tool_spec(tool: &CodexToolSpec) -> Option<codex_wire_protocol::ToolSpec> {
    match tool {
        CodexToolSpec::Function(tool) => Some(codex_wire_protocol::ToolSpec {
            name: tool.name.clone(),
            description: tool.description.clone(),
            parameters: serde_json::to_value(&tool.parameters).ok()?,
        }),
        CodexToolSpec::Namespace(namespace) => {
            let parameters = serde_json::json!({
                "type": "object",
                "properties": {
                    "tool": {
                        "type": "string",
                        "description": "Name of the namespace tool to call."
                    },
                    "arguments": {
                        "type": "object",
                        "description": "Arguments for the selected namespace tool."
                    }
                },
                "required": ["tool", "arguments"],
                "additionalProperties": false
            });
            Some(codex_wire_protocol::ToolSpec {
                name: namespace.name.clone(),
                description: namespace.description.clone(),
                parameters,
            })
        }
        _ => None,
    }
}

pub(crate) fn auth_token_for_provider(
    auth: Option<&CodexAuth>,
    provider: &ModelProviderInfo,
) -> Result<codex_wire_protocol::AuthToken> {
    if let Some(api_key) = provider.api_key()? {
        return Ok(codex_wire_protocol::AuthToken::ApiKey {
            key: api_key,
            header_name: "Authorization".to_string(),
        });
    }

    if let Some(token) = provider.experimental_bearer_token.clone() {
        return Ok(codex_wire_protocol::AuthToken::Bearer(token));
    }

    if let Some(auth) = auth {
        return auth
            .get_token()
            .map(codex_wire_protocol::AuthToken::Bearer)
            .map_err(CodexErr::Io);
    }

    Ok(codex_wire_protocol::AuthToken::None)
}

pub(crate) fn map_wire_error(error: codex_wire_protocol::WireError) -> CodexErr {
    CodexErr::Stream(error.to_string(), None)
}

pub(crate) fn map_turn_stream(turn_stream: codex_wire_protocol::TurnStream) -> ResponseStream {
    let (tx_event, rx_event) =
        mpsc::channel::<Result<ResponseEvent>>(RESPONSE_STREAM_CHANNEL_CAPACITY);
    let consumer_dropped = CancellationToken::new();
    let consumer_dropped_for_stream = consumer_dropped.clone();
    tokio::spawn(async move {
        let mut events = turn_stream.events;
        let mut completed = false;
        loop {
            let event = tokio::select! {
                _ = consumer_dropped.cancelled() => return,
                event = events.recv() => event,
            };
            let Some(event) = event else {
                break;
            };
            let Some(mapped) = turn_event_to_response_event(event) else {
                continue;
            };
            if matches!(mapped, Ok(ResponseEvent::Completed { .. })) {
                completed = true;
            }
            if tx_event.send(mapped).await.is_err() {
                return;
            }
        }
        if !completed {
            let _ = tx_event
                .send(Err(CodexErr::Stream(
                    "native protocol stream closed before completion".to_string(),
                    None,
                )))
                .await;
        }
    });

    ResponseStream {
        rx_event,
        consumer_dropped: consumer_dropped_for_stream,
    }
}

pub(crate) fn turn_event_to_response_event(
    event: codex_wire_protocol::TurnEvent,
) -> Option<Result<ResponseEvent>> {
    match event {
        codex_wire_protocol::TurnEvent::Started { .. } => Some(Ok(ResponseEvent::Created)),
        codex_wire_protocol::TurnEvent::ContentDelta { text } => {
            Some(Ok(ResponseEvent::OutputTextDelta(text)))
        }
        codex_wire_protocol::TurnEvent::ReasoningDelta { text } => {
            Some(Ok(ResponseEvent::ReasoningContentDelta {
                delta: text,
                content_index: 0,
            }))
        }
        codex_wire_protocol::TurnEvent::ToolCallStarted { call_id, name } => Some(Ok(
            ResponseEvent::OutputItemAdded(ResponseItem::FunctionCall {
                id: None,
                name,
                namespace: None,
                arguments: String::new(),
                call_id,
            }),
        )),
        codex_wire_protocol::TurnEvent::ToolCallArgumentsDelta { call_id, arguments } => {
            Some(Ok(ResponseEvent::ToolCallInputDelta {
                item_id: call_id.clone(),
                call_id: Some(call_id),
                delta: arguments,
            }))
        }
        codex_wire_protocol::TurnEvent::ToolCallCompleted {
            call_id,
            name,
            arguments,
        } => Some(Ok(ResponseEvent::OutputItemDone(
            ResponseItem::FunctionCall {
                id: None,
                name,
                namespace: None,
                arguments,
                call_id,
            },
        ))),
        codex_wire_protocol::TurnEvent::Completed { end_turn } => {
            Some(Ok(ResponseEvent::Completed {
                response_id: format!("native-{}", uuid::Uuid::new_v4()),
                token_usage: None,
                end_turn: Some(end_turn),
            }))
        }
        codex_wire_protocol::TurnEvent::Failed { error, .. } => {
            Some(Err(CodexErr::Stream(error, None)))
        }
        codex_wire_protocol::TurnEvent::Usage { .. } => None,
        codex_wire_protocol::TurnEvent::ToolResult { .. } => None,
    }
}

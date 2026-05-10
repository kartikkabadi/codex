// codex-wire-protocol-openai/src/response.rs

use codex_wire_protocol::response::TurnEvent;
use codex_wire_protocol::response::TurnStream;

use crate::types::ResponseEvent;

/// Convert a `ResponseStream` (OpenAI-specific events) into a generic
/// `TurnStream`.
pub fn response_stream_to_turn_stream(
    mut openai_stream: crate::types::ResponseStream,
    turn_id: String,
) -> TurnStream {
    let (tx, rx) = tokio::sync::mpsc::channel(256);

    tokio::spawn(async move {
        while let Some(result) = openai_stream.rx_event.recv().await {
            match result {
                Ok(event) => {
                    if let Some(turn_event) = map_response_event(event) {
                        let _ = tx.send(turn_event).await;
                    }
                }
                Err(err) => {
                    let wire_err: codex_wire_protocol::error::WireError = err.into();
                    let _ = tx
                        .send(TurnEvent::Failed {
                            error: wire_err.to_string(),
                            code: None,
                        })
                        .await;
                    break;
                }
            }
        }
    });

    TurnStream {
        events: rx,
        turn_id,
    }
}

fn map_response_event(event: ResponseEvent) -> Option<TurnEvent> {
    match event {
        ResponseEvent::Created => Some(TurnEvent::Started {
            turn_id: String::new(),
        }),
        ResponseEvent::OutputTextDelta(text) => Some(TurnEvent::ContentDelta { text }),
        ResponseEvent::ReasoningContentDelta { delta, .. } => {
            Some(TurnEvent::ReasoningDelta { text: delta })
        }
        ResponseEvent::ReasoningSummaryDelta { delta, .. } => {
            Some(TurnEvent::ReasoningDelta { text: delta })
        }
        ResponseEvent::ToolCallInputDelta { item_id, delta, .. } => {
            Some(TurnEvent::ToolCallArgumentsDelta {
                call_id: item_id,
                arguments: delta,
            })
        }
        ResponseEvent::OutputItemDone(item) => map_output_item_done(item),
        ResponseEvent::OutputItemAdded(item) => map_output_item_added(item),
        ResponseEvent::Completed {
            response_id: _,
            token_usage,
            end_turn,
        } => {
            if let Some(usage) = token_usage {
                Some(TurnEvent::Usage {
                    input_tokens: usage.input_tokens as u32,
                    output_tokens: usage.output_tokens as u32,
                    reasoning_tokens: Some(usage.reasoning_output_tokens as u32),
                })
            } else {
                Some(TurnEvent::Completed {
                    end_turn: end_turn.unwrap_or(false),
                })
            }
        }
        ResponseEvent::ServerModel(model) => {
            // Could emit a metadata event; for now skip.
            let _ = model;
            None
        }
        ResponseEvent::ModelVerifications(_) => None,
        ResponseEvent::ServerReasoningIncluded(_) => None,
        ResponseEvent::RateLimits(_) => None,
        ResponseEvent::ModelsEtag(_) => None,
        ResponseEvent::ReasoningSummaryPartAdded { .. } => None,
    }
}

fn map_output_item_done(item: codex_protocol::models::ResponseItem) -> Option<TurnEvent> {
    match item {
        codex_protocol::models::ResponseItem::Message { role, content, .. } => {
            let text = content
                .iter()
                .filter_map(|c| match c {
                    codex_protocol::models::ContentItem::OutputText { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("");
            if role == "assistant" && !text.is_empty() {
                Some(TurnEvent::ContentDelta { text })
            } else {
                None
            }
        }
        codex_protocol::models::ResponseItem::FunctionCall {
            call_id,
            name,
            arguments,
            ..
        } => Some(TurnEvent::ToolCallCompleted {
            call_id,
            name,
            arguments,
        }),
        codex_protocol::models::ResponseItem::FunctionCallOutput {
            call_id, output, ..
        } => Some(TurnEvent::ToolResult {
            call_id,
            output: output.body.to_text().unwrap_or_default(),
        }),
        _ => None,
    }
}

fn map_output_item_added(item: codex_protocol::models::ResponseItem) -> Option<TurnEvent> {
    match item {
        codex_protocol::models::ResponseItem::FunctionCall { call_id, name, .. } => {
            Some(TurnEvent::ToolCallStarted { call_id, name })
        }
        _ => None,
    }
}

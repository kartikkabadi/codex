// codex-wire-protocol-anthropic/src/response.rs

use codex_wire_protocol::response::TurnEvent;
use codex_wire_protocol::response::TurnStream;

use crate::sse::AnthropicSseStream;
use crate::sse::SseEvent;
use crate::sse::spawn_sse_stream;

/// Convert an `AnthropicSseStream` into a generic `TurnStream`.
pub fn response_stream_to_turn_stream(
    sse_stream: AnthropicSseStream,
    turn_id: String,
) -> TurnStream {
    let (tx, rx) = tokio::sync::mpsc::channel(256);
    let mut rx_sse = spawn_sse_stream(sse_stream);

    tokio::spawn(async move {
        while let Some(result) = rx_sse.recv().await {
            match result {
                Ok(event) => {
                    let turn_events = map_sse_event(event);
                    for turn_event in turn_events {
                        if tx.send(turn_event).await.is_err() {
                            break;
                        }
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

fn map_sse_event(event: SseEvent) -> Vec<TurnEvent> {
    match event {
        SseEvent::MessageStart { .. } => {
            vec![TurnEvent::Started {
                turn_id: String::new(),
            }]
        }
        SseEvent::ContentBlockDelta { text, .. } => {
            vec![TurnEvent::ContentDelta { text }]
        }
        SseEvent::ToolUseStart { id, name, .. } => {
            vec![TurnEvent::ToolCallStarted { call_id: id, name }]
        }
        SseEvent::ToolUseDelta { input_json, .. } => {
            vec![TurnEvent::ToolCallArgumentsDelta {
                call_id: String::new(), // Anthropic tool_use deltas don't include call_id; we rely on ordering
                arguments: input_json,
            }]
        }
        SseEvent::MessageDelta { stop_reason, usage } => {
            let mut events = Vec::new();
            if stop_reason.as_deref() == Some("end_turn")
                || stop_reason.as_deref() == Some("tool_use")
            {
                events.push(TurnEvent::Completed { end_turn: true });
            }
            if let Some(u) = usage {
                events.push(TurnEvent::Usage {
                    input_tokens: u.input_tokens,
                    output_tokens: u.output_tokens,
                    reasoning_tokens: None,
                });
            }
            events
        }
        SseEvent::MessageStop => vec![TurnEvent::Completed { end_turn: true }],
        SseEvent::Ping => Vec::new(),
        SseEvent::ContentBlockStop { .. } => Vec::new(),
    }
}

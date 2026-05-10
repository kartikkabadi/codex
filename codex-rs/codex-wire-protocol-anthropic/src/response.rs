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
                    if let Some(turn_event) = map_sse_event(event) {
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

fn map_sse_event(event: SseEvent) -> Option<TurnEvent> {
    match event {
        SseEvent::MessageStart { .. } => Some(TurnEvent::Started {
            turn_id: String::new(),
        }),
        SseEvent::ContentBlockDelta { text, .. } => Some(TurnEvent::ContentDelta { text }),
        SseEvent::MessageDelta { stop_reason, usage } => {
            if stop_reason.as_deref() == Some("end_turn") {
                Some(TurnEvent::Completed { end_turn: true })
            } else {
                usage.map(|u| TurnEvent::Usage {
                    input_tokens: u.input_tokens,
                    output_tokens: u.output_tokens,
                    reasoning_tokens: None,
                })
            }
        }
        SseEvent::MessageStop => Some(TurnEvent::Completed { end_turn: true }),
        SseEvent::Ping => None,
        SseEvent::ContentBlockStop { .. } => None,
    }
}

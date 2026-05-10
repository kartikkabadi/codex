// codex-wire-protocol-gemini/src/response.rs

use codex_wire_protocol::response::TurnEvent;
use codex_wire_protocol::response::TurnStream;

use crate::sse::GeminiSseStream;
use crate::sse::SseEvent;
use crate::sse::spawn_sse_stream;

/// Convert a `GeminiSseStream` into a generic `TurnStream`.
pub fn response_stream_to_turn_stream(sse_stream: GeminiSseStream, turn_id: String) -> TurnStream {
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
        SseEvent::Candidate {
            content,
            finish_reason,
        } => {
            let mut events = Vec::new();
            for part in content.parts {
                match part {
                    crate::types::Part::Text { text } => {
                        events.push(TurnEvent::ContentDelta { text });
                    }
                    crate::types::Part::FunctionCall { function_call } => {
                        events.push(TurnEvent::ToolCallCompleted {
                            call_id: function_call.name.clone(),
                            name: function_call.name,
                            arguments: function_call.args.to_string(),
                        });
                    }
                    _ => {}
                }
            }
            if finish_reason.as_deref() == Some("STOP") {
                events.push(TurnEvent::Completed { end_turn: true });
            }
            // Return the first event; caller should ideally batch, but
            // TurnStream only yields one at a time. For simplicity emit
            // content delta if present, otherwise completed.
            events.into_iter().next()
        }
        SseEvent::Usage {
            prompt_tokens,
            completion_tokens,
        } => Some(TurnEvent::Usage {
            input_tokens: prompt_tokens,
            output_tokens: completion_tokens,
            reasoning_tokens: None,
        }),
    }
}

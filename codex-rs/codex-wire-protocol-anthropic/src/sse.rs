// codex-wire-protocol-anthropic/src/sse.rs

use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use bytes::Bytes;
use futures::Stream;
use futures::StreamExt;
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::trace;

use crate::error::AnthropicError;
use crate::types::Usage;

/// SSE event types emitted by Anthropic's streaming Messages API.
#[derive(Debug, Clone)]
pub enum SseEvent {
    MessageStart {
        id: String,
    },
    ContentBlockDelta {
        index: usize,
        text: String,
    },
    ContentBlockStop {
        index: usize,
    },
    MessageDelta {
        stop_reason: Option<String>,
        usage: Option<Usage>,
    },
    MessageStop,
    Ping,
}

/// Raw SSE event parsed from the stream.
#[derive(Debug, Deserialize)]
struct SseRawEvent {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    index: Option<usize>,
    #[serde(default)]
    delta: Option<SseDelta>,
    #[serde(default)]
    message: Option<SseMessage>,
    #[serde(default)]
    content_block: Option<SseContentBlock>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct SseDelta {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SseMessage {
    id: String,
}

#[derive(Debug, Deserialize)]
struct SseContentBlock {
    #[serde(default)]
    index: Option<usize>,
}

/// A stream of raw bytes from the HTTP response body.
pub type ByteStream = Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>;

/// Anthropic-specific SSE stream that parses raw bytes into structured events.
pub struct AnthropicSseStream {
    inner: ByteStream,
    buffer: String,
}

impl AnthropicSseStream {
    pub fn new(inner: ByteStream) -> Self {
        Self {
            inner,
            buffer: String::new(),
        }
    }
}

impl Stream for AnthropicSseStream {
    type Item = Result<SseEvent, AnthropicError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.inner.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(chunk))) => {
                    self.buffer.push_str(&String::from_utf8_lossy(&chunk));
                    if let Some(event) = try_parse_event(&mut self.buffer) {
                        return Poll::Ready(Some(event));
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(AnthropicError::Sse(e.to_string()))));
                }
                Poll::Ready(None) => {
                    if !self.buffer.trim().is_empty() {
                        let remainder = self.buffer.clone();
                        self.buffer.clear();
                        return Poll::Ready(Some(Err(AnthropicError::Sse(format!(
                            "Incomplete SSE stream: {remainder}"
                        )))));
                    }
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

fn try_parse_event(buffer: &mut String) -> Option<Result<SseEvent, AnthropicError>> {
    if let Some((event_line, data_line, rest)) = extract_sse_event(buffer) {
        *buffer = rest;
        let event_type = event_line.trim();
        let data = data_line.trim();

        trace!("SSE event: {event_type} data: {data}");

        match event_type {
            "message_start" => {
                if let Ok(raw) = serde_json::from_str::<SseRawEvent>(data) {
                    if let Some(msg) = raw.message {
                        return Some(Ok(SseEvent::MessageStart { id: msg.id }));
                    }
                }
            }
            "content_block_delta" => {
                if let Ok(raw) = serde_json::from_str::<SseRawEvent>(data) {
                    if let (Some(idx), Some(delta)) = (raw.index, raw.delta) {
                        if let Some(text) = delta.text {
                            return Some(Ok(SseEvent::ContentBlockDelta { index: idx, text }));
                        }
                    }
                }
            }
            "content_block_stop" => {
                if let Ok(raw) = serde_json::from_str::<SseRawEvent>(data) {
                    if let Some(idx) = raw.index {
                        return Some(Ok(SseEvent::ContentBlockStop { index: idx }));
                    }
                }
            }
            "message_delta" => {
                if let Ok(raw) = serde_json::from_str::<SseRawEvent>(data) {
                    let stop_reason = raw.delta.and_then(|d| d.stop_reason);
                    return Some(Ok(SseEvent::MessageDelta {
                        stop_reason,
                        usage: raw.usage,
                    }));
                }
            }
            "message_stop" => return Some(Ok(SseEvent::MessageStop)),
            "ping" => return Some(Ok(SseEvent::Ping)),
            _ => {
                debug!("Unhandled SSE event type: {event_type}");
            }
        }
    }
    None
}

/// Extract a single SSE event (event: ...\ndata: ...\n\n) from the buffer.
fn extract_sse_event(buffer: &str) -> Option<(String, String, String)> {
    let mut event_line: Option<String> = None;
    let mut data_line: Option<String> = None;
    let mut consumed = 0usize;

    for line in buffer.lines() {
        consumed += line.len() + 1; // +1 for newline
        let trimmed = line.trim();
        if trimmed.is_empty() {
            // End of event
            if let (Some(event), Some(data)) = (&event_line, &data_line) {
                let rest = buffer.split_at(consumed).1.to_string();
                return Some((event.clone(), data.clone(), rest));
            }
            continue;
        }
        if let Some(stripped) = trimmed.strip_prefix("event:") {
            event_line = Some(stripped.trim().to_string());
        } else if let Some(stripped) = trimmed.strip_prefix("data:") {
            data_line = Some(stripped.trim().to_string());
        }
    }

    None
}

/// Spawn a background task that consumes the SSE stream and sends parsed events
/// over an mpsc channel.
pub fn spawn_sse_stream(
    sse_stream: AnthropicSseStream,
) -> mpsc::Receiver<Result<SseEvent, AnthropicError>> {
    let (tx, rx) = mpsc::channel::<Result<SseEvent, AnthropicError>>(256);
    tokio::spawn(async move {
        let mut stream = sse_stream;
        while let Some(result) = stream.next().await {
            if tx.send(result).await.is_err() {
                break;
            }
        }
    });
    rx
}

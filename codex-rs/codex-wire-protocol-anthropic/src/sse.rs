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
    ToolUseStart {
        index: usize,
        id: String,
        name: String,
    },
    ToolUseDelta {
        index: usize,
        input_json: String,
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
    #[serde(default)]
    input_json: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SseMessage {
    id: String,
}

#[derive(Debug, Deserialize)]
struct SseContentBlock {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
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
                    // Keep trying to parse events from the buffer. If an unhandled
                    // event is consumed (buffer shrinks but no event returned), keep
                    // trying until no more complete events can be extracted.
                    loop {
                        let before = self.buffer.len();
                        if let Some(event) = try_parse_event(&mut self.buffer) {
                            return Poll::Ready(Some(event));
                        }
                        if self.buffer.len() == before {
                            break;
                        }
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(AnthropicError::Sse(e.to_string()))));
                }
                Poll::Ready(None) => {
                    // Drain all remaining complete events from the buffer.
                    // Keep trying until no more complete events can be extracted.
                    loop {
                        let before = self.buffer.len();
                        if let Some(event) = try_parse_event(&mut self.buffer) {
                            return Poll::Ready(Some(event));
                        }
                        if self.buffer.len() == before {
                            break;
                        }
                    }
                    // After draining complete events, check if anything remains.
                    let trimmed = self.buffer.trim();
                    if !trimmed.is_empty() {
                        // If the trimmed buffer looks like a single data line (no event: prefix),
                        // try to parse it as a raw JSON event.
                        if let Some(data) = trimmed.strip_prefix("data:") {
                            let data = data.trim();
                            if let Ok(raw) = serde_json::from_str::<SseRawEvent>(data) {
                                self.buffer.clear();
                                if let Some(event) = map_raw_event(raw) {
                                    return Poll::Ready(Some(Ok(event)));
                                }
                            }
                        }
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

        if let Some(result) = parse_sse_event_data(event_type, data) {
            return Some(result);
        }
    }
    None
}

fn parse_sse_event_data(event_type: &str, data: &str) -> Option<Result<SseEvent, AnthropicError>> {
    match event_type {
        "message_start" => {
            if let Ok(raw) = serde_json::from_str::<SseRawEvent>(data) {
                if let Some(msg) = raw.message {
                    return Some(Ok(SseEvent::MessageStart { id: msg.id }));
                }
            }
        }
        "content_block_start" => {
            if let Ok(raw) = serde_json::from_str::<SseRawEvent>(data) {
                if let (Some(idx), Some(block)) = (raw.index, raw.content_block) {
                    if let (Some(id), Some(name)) = (block.id, block.name) {
                        return Some(Ok(SseEvent::ToolUseStart {
                            index: idx,
                            id,
                            name,
                        }));
                    }
                }
            }
        }
        "content_block_delta" => {
            if let Ok(raw) = serde_json::from_str::<SseRawEvent>(data) {
                if let (Some(idx), Some(delta)) = (raw.index, raw.delta) {
                    if let Some(text) = delta.text {
                        return Some(Ok(SseEvent::ContentBlockDelta { index: idx, text }));
                    }
                    if let Some(input_json) = delta.input_json {
                        return Some(Ok(SseEvent::ToolUseDelta {
                            index: idx,
                            input_json,
                        }));
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
    None
}

fn map_raw_event(raw: SseRawEvent) -> Option<SseEvent> {
    match raw.kind.as_str() {
        "message_stop" => Some(SseEvent::MessageStop),
        "ping" => Some(SseEvent::Ping),
        _ => None,
    }
}

/// Extract a single SSE event (event: ...\ndata: ...\n\n) from the buffer.
fn extract_sse_event(buffer: &str) -> Option<(String, String, String)> {
    // Find the first blank line (\n\n) that separates SSE events.
    let mut event_line: Option<String> = None;
    let mut data_line: Option<String> = None;
    let mut consumed = 0usize;

    for line in buffer.lines() {
        consumed += line.len() + 1; // +1 for newline
        let trimmed = line.trim();
        if trimmed.is_empty() {
            // End of event — only return if we have both event and data
            if let (Some(event), Some(data)) = (event_line.take(), data_line.take()) {
                let rest = buffer.split_at(consumed).1.to_string();
                return Some((event, data, rest));
            }
            // Otherwise this is just an empty line between events; keep scanning
            continue;
        }
        if let Some(stripped) = trimmed.strip_prefix("event:") {
            // If we already had an event+data pair without a blank line separator,
            // treat the new "event:" as the start of the next event and return what we have.
            if let (Some(event), Some(data)) = (event_line.take(), data_line.take()) {
                // consumed already includes this line; back it out
                consumed -= line.len() + 1;
                let rest = buffer.split_at(consumed).1.to_string();
                return Some((event, data, rest));
            }
            event_line = Some(stripped.trim().to_string());
        } else if let Some(stripped) = trimmed.strip_prefix("data:") {
            data_line = Some(stripped.trim().to_string());
        }
    }

    // End of buffer reached. If we have a complete event (event + data) but no trailing blank line,
    // return it anyway — the stream may have ended without the final \n\n.
    if let (Some(event), Some(data)) = (event_line, data_line) {
        let rest = buffer.split_at(consumed).1.to_string();
        return Some((event, data, rest));
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

// codex-wire-protocol-gemini/src/sse.rs

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

use crate::error::GeminiError;
use crate::types::Content;
use crate::types::UsageMetadata;

/// SSE event types emitted by Gemini's streaming generateContent API.
#[derive(Debug, Clone)]
pub enum SseEvent {
    Candidate {
        content: Content,
        finish_reason: Option<String>,
    },
    Usage {
        prompt_tokens: u32,
        completion_tokens: u32,
    },
}

/// Raw SSE event parsed from the stream.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SseRawEvent {
    #[serde(default)]
    candidates: Option<Vec<SseRawCandidate>>,
    #[serde(default)]
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SseRawCandidate {
    #[serde(default)]
    content: Option<Content>,
    #[serde(default)]
    finish_reason: Option<String>,
}

/// A stream of raw bytes from the HTTP response body.
pub type ByteStream = Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>;

/// Gemini-specific SSE stream that parses raw bytes into structured events.
pub struct GeminiSseStream {
    inner: ByteStream,
    buffer: String,
}

impl GeminiSseStream {
    pub fn new(inner: ByteStream) -> Self {
        Self {
            inner,
            buffer: String::new(),
        }
    }
}

impl Stream for GeminiSseStream {
    type Item = Result<SseEvent, GeminiError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.inner.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(chunk))) => {
                    self.buffer.push_str(&String::from_utf8_lossy(&chunk));
                    loop {
                        let buf_len = self.buffer.len();
                        if let Some(event) = try_parse_event(&mut self.buffer) {
                            return Poll::Ready(Some(event));
                        }
                        if self.buffer.len() == buf_len {
                            break;
                        }
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(GeminiError::Sse(e.to_string()))));
                }
                Poll::Ready(None) => {
                    // Drain all remaining complete events from the buffer.
                    loop {
                        let buf_len = self.buffer.len();
                        if let Some(event) = try_parse_event(&mut self.buffer) {
                            return Poll::Ready(Some(event));
                        }
                        if self.buffer.len() == buf_len {
                            break;
                        }
                    }
                    if !self.buffer.trim().is_empty() {
                        let remainder = self.buffer.clone();
                        self.buffer.clear();
                        return Poll::Ready(Some(Err(GeminiError::Sse(format!(
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

fn try_parse_event(buffer: &mut String) -> Option<Result<SseEvent, GeminiError>> {
    if let Some((data_line, rest)) = extract_sse_data(buffer) {
        *buffer = rest;
        let data = data_line.trim();

        trace!("SSE data: {data}");

        if data == "[DONE]" {
            return None;
        }

        let raw: SseRawEvent = match serde_json::from_str(data) {
            Ok(ev) => ev,
            Err(e) => {
                debug!("Failed to parse SSE data: {e}, data: {data}");
                return None;
            }
        };

        if let Some(candidates) = raw.candidates {
            for candidate in candidates {
                if let Some(content) = candidate.content {
                    return Some(Ok(SseEvent::Candidate {
                        content,
                        finish_reason: candidate.finish_reason.clone(),
                    }));
                }
            }
        }

        if let Some(usage) = raw.usage_metadata {
            return Some(Ok(SseEvent::Usage {
                prompt_tokens: usage.prompt_token_count,
                completion_tokens: usage.candidates_token_count,
            }));
        }
    }
    None
}

/// Extract a single SSE data block (data: {...}\n\n) from the buffer.
fn extract_sse_data(buffer: &str) -> Option<(String, String)> {
    let mut data_lines: Vec<String> = Vec::new();
    let mut consumed = 0usize;

    for line in buffer.lines() {
        consumed += line.len() + 1; // +1 for newline
        let trimmed = line.trim();
        if trimmed.is_empty() {
            // End of event
            if !data_lines.is_empty() {
                let rest = buffer.split_at(consumed).1.to_string();
                return Some((data_lines.join("\n"), rest));
            }
            continue;
        }
        if let Some(stripped) = trimmed.strip_prefix("data:") {
            data_lines.push(stripped.trim().to_string());
        }
    }

    // End of buffer reached. If we have data but no trailing blank line,
    // return it anyway — the stream may have ended without the final \n\n.
    if !data_lines.is_empty() {
        let rest = buffer.split_at(consumed).1.to_string();
        return Some((data_lines.join("\n"), rest));
    }

    None
}

/// Spawn a background task that consumes the SSE stream and sends parsed events
/// over an mpsc channel.
pub fn spawn_sse_stream(
    sse_stream: GeminiSseStream,
) -> mpsc::Receiver<Result<SseEvent, GeminiError>> {
    let (tx, rx) = mpsc::channel::<Result<SseEvent, GeminiError>>(256);
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

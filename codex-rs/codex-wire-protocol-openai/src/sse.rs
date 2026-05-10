// codex-wire-protocol-openai/src/sse.rs

use std::io::BufRead;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

use bytes::Bytes;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::ModelVerification;
use codex_protocol::protocol::TokenUsage;
use eventsource_stream::EventStreamError;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use futures::TryStreamExt;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio::time::Instant;
use tokio::time::timeout;
use tokio_util::io::ReaderStream;
use tracing::debug;
use tracing::trace;

use crate::types::ResponseEvent;
use crate::types::ResponseStream;

const X_REASONING_INCLUDED_HEADER: &str = "x-reasoning-included";
const OPENAI_MODEL_HEADER: &str = "openai-model";
const REQUEST_ID_HEADER: &str = "x-request-id";
const TRUSTED_ACCESS_FOR_CYBER_VERIFICATION: &str = "trusted_access_for_cyber";

pub type ByteStream =
    Pin<Box<dyn futures::Stream<Item = Result<Bytes, std::io::Error>> + Send>>;

/// Telemetry trait for SSE processing.
pub trait SseTelemetry: Send + Sync {
    fn on_sse_poll(
        &self,
        response: &Result<Option<Result<eventsource_stream::Event, EventStreamError<std::io::Error>>>, tokio::time::error::Elapsed>,
        elapsed: Duration,
    );
}

/// Streams SSE events from an on-disk fixture for tests.
pub fn stream_from_fixture(
    path: impl AsRef<std::path::Path>,
    idle_timeout: Duration,
) -> Result<ResponseStream, crate::error::OpenAiError> {
    let file = std::fs::File::open(path.as_ref())
        .map_err(|err| crate::error::OpenAiError::Stream(err.to_string()))?;
    let mut content = String::new();
    for line in std::io::BufReader::new(file).lines() {
        let line: std::io::Result<String> = line;
        let line = line.map_err(|err| crate::error::OpenAiError::Stream(err.to_string()))?;
        content.push_str(&line);
        content.push_str("\n\n");
    }

    let reader = std::io::Cursor::new(content);
    let stream = ReaderStream::new(reader)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err.to_string()));
    let (tx_event, rx_event) =
        mpsc::channel::<Result<ResponseEvent, crate::error::OpenAiError>>(1600);
    tokio::spawn(process_sse(
        Box::pin(stream),
        tx_event,
        idle_timeout,
        /*telemetry*/ None,
    ));
    Ok(ResponseStream {
        rx_event,
        upstream_request_id: None,
    })
}

/// Spawn a background task that processes an SSE byte stream into
/// `ResponseEvent`s.
pub fn spawn_response_stream(
    bytes: ByteStream,
    headers: &http::HeaderMap,
    idle_timeout: Duration,
    telemetry: Option<Arc<dyn SseTelemetry>>,
    turn_state: Option<Arc<OnceLock<String>>>,
) -> ResponseStream {
    let upstream_request_id = headers
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let server_model = headers
        .get(OPENAI_MODEL_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string);
    let reasoning_included = headers.get(X_REASONING_INCLUDED_HEADER).is_some();

    if let Some(turn_state) = turn_state.as_ref() {
        if let Some(header_value) = headers
            .get("x-codex-turn-state")
            .and_then(|v| v.to_str().ok())
        {
            let _ = turn_state.set(header_value.to_string());
        }
    }

    let (tx_event, rx_event) =
        mpsc::channel::<Result<ResponseEvent, crate::error::OpenAiError>>(1600);
    tokio::spawn(async move {
        if let Some(model) = server_model {
            let _ = tx_event.send(Ok(ResponseEvent::ServerModel(model))).await;
        }
        if reasoning_included {
            let _ = tx_event
                .send(Ok(ResponseEvent::ServerReasoningIncluded(true)))
                .await;
        }
        process_sse(bytes, tx_event, idle_timeout, telemetry).await;
    });

    ResponseStream {
        rx_event,
        upstream_request_id,
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Error {
    r#type: Option<String>,
    code: Option<String>,
    message: Option<String>,
    plan_type: Option<String>,
    resets_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ResponseCompleted {
    id: String,
    #[serde(default)]
    usage: Option<ResponseCompletedUsage>,
    #[serde(default)]
    end_turn: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ResponseCompletedUsage {
    input_tokens: i64,
    input_tokens_details: Option<ResponseCompletedInputTokensDetails>,
    output_tokens: i64,
    output_tokens_details: Option<ResponseCompletedOutputTokensDetails>,
    total_tokens: i64,
}

impl From<ResponseCompletedUsage> for TokenUsage {
    fn from(val: ResponseCompletedUsage) -> Self {
        TokenUsage {
            input_tokens: val.input_tokens,
            cached_input_tokens: val
                .input_tokens_details
                .map(|d| d.cached_tokens)
                .unwrap_or(0),
            output_tokens: val.output_tokens,
            reasoning_output_tokens: val
                .output_tokens_details
                .map(|d| d.reasoning_tokens)
                .unwrap_or(0),
            total_tokens: val.total_tokens,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ResponseCompletedInputTokensDetails {
    cached_tokens: i64,
}

#[derive(Debug, Deserialize)]
struct ResponseCompletedOutputTokensDetails {
    reasoning_tokens: i64,
}

#[derive(Deserialize, Debug)]
pub struct ResponsesStreamEvent {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    headers: Option<Value>,
    metadata: Option<Value>,
    response: Option<Value>,
    item: Option<Value>,
    item_id: Option<String>,
    call_id: Option<String>,
    delta: Option<String>,
    summary_index: Option<i64>,
    content_index: Option<i64>,
}

impl ResponsesStreamEvent {
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Returns the effective model reported by the server, if present.
    ///
    /// Precedence:
    /// 1. `response.headers` for standard Responses stream events.
    /// 2. top-level `headers` for websocket metadata events.
    pub fn response_model(&self) -> Option<String> {
        let response_headers_model = self
            .response
            .as_ref()
            .and_then(|response| response.get("headers"))
            .and_then(header_openai_model_value_from_json);

        match response_headers_model {
            Some(model) => Some(model),
            None => self
                .headers
                .as_ref()
                .and_then(header_openai_model_value_from_json),
        }
    }

    pub(crate) fn model_verifications(&self) -> Option<Vec<ModelVerification>> {
        if self.kind() != "response.metadata" {
            return None;
        }

        self.metadata
            .as_ref()
            .and_then(|metadata| metadata.get("openai_verification_recommendation"))
            .and_then(model_verifications_from_json_value)
    }
}

fn header_openai_model_value_from_json(value: &Value) -> Option<String> {
    let headers = value.as_object()?;
    headers.iter().find_map(|(name, value)| {
        if name.eq_ignore_ascii_case("openai-model") || name.eq_ignore_ascii_case("x-openai-model")
        {
            json_value_as_string(value)
        } else {
            None
        }
    })
}

fn model_verifications_from_json_value(value: &Value) -> Option<Vec<ModelVerification>> {
    let verifications = value
        .as_array()
        .map(|items| {
            let mut verifications = Vec::new();
            for verification in items
                .iter()
                .filter_map(Value::as_str)
                .filter_map(parse_model_verification)
            {
                if !verifications.contains(&verification) {
                    verifications.push(verification);
                }
            }
            verifications
        })
        .unwrap_or_default();

    if verifications.is_empty() {
        None
    } else {
        Some(verifications)
    }
}

fn parse_model_verification(value: &str) -> Option<ModelVerification> {
    match value {
        TRUSTED_ACCESS_FOR_CYBER_VERIFICATION => Some(ModelVerification::TrustedAccessForCyber),
        _ => None,
    }
}

fn json_value_as_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Array(items) => items.first().and_then(json_value_as_string),
        _ => None,
    }
}

#[derive(Debug)]
pub enum ResponsesEventError {
    Api(crate::error::OpenAiError),
}

impl ResponsesEventError {
    pub fn into_api_error(self) -> crate::error::OpenAiError {
        match self {
            Self::Api(error) => error,
        }
    }
}

pub fn process_responses_event(
    event: ResponsesStreamEvent,
) -> std::result::Result<Option<ResponseEvent>, ResponsesEventError> {
    match event.kind.as_str() {
        "response.output_item.done" => {
            if let Some(item_val) = event.item {
                if let Ok(item) = serde_json::from_value::<ResponseItem>(item_val) {
                    return Ok(Some(ResponseEvent::OutputItemDone(item)));
                }
                debug!("failed to parse ResponseItem from output_item.done");
            }
        }
        "response.output_text.delta" => {
            if let Some(delta) = event.delta {
                return Ok(Some(ResponseEvent::OutputTextDelta(delta)));
            }
        }
        "response.custom_tool_call_input.delta" => {
            if let (Some(delta), Some(item_id)) =
                (event.delta, event.item_id.clone().or(event.call_id.clone()))
            {
                return Ok(Some(ResponseEvent::ToolCallInputDelta {
                    item_id,
                    call_id: event.call_id,
                    delta,
                }));
            }
        }
        "response.reasoning_summary_text.delta" => {
            if let (Some(delta), Some(summary_index)) = (event.delta, event.summary_index) {
                return Ok(Some(ResponseEvent::ReasoningSummaryDelta {
                    delta,
                    summary_index,
                }));
            }
        }
        "response.reasoning_text.delta" => {
            if let (Some(delta), Some(content_index)) = (event.delta, event.content_index) {
                return Ok(Some(ResponseEvent::ReasoningContentDelta {
                    delta,
                    content_index,
                }));
            }
        }
        "response.created" => {
            if event.response.is_some() {
                return Ok(Some(ResponseEvent::Created {}));
            }
        }
        "response.failed" => {
            if let Some(resp_val) = event.response {
                let mut response_error =
                    crate::error::OpenAiError::Stream("response.failed event received".into());
                if let Some(error) = resp_val.get("error")
                    && let Ok(error) = serde_json::from_value::<Error>(error.clone())
                {
                    if is_context_window_error(&error) {
                        response_error = crate::error::OpenAiError::ContextWindowExceeded;
                    } else if is_quota_exceeded_error(&error) {
                        response_error = crate::error::OpenAiError::QuotaExceeded;
                    } else if is_usage_not_included(&error) {
                        response_error = crate::error::OpenAiError::UsageNotIncluded;
                    } else if is_cyber_policy_error(&error) {
                        let message = cyber_policy_message(error.message);
                        response_error = crate::error::OpenAiError::CyberPolicy { message };
                    } else if is_invalid_prompt_error(&error) {
                        let message = error
                            .message
                            .unwrap_or_else(|| "Invalid request.".to_string());
                        response_error = crate::error::OpenAiError::InvalidRequest { message };
                    } else if is_server_overloaded_error(&error) {
                        response_error = crate::error::OpenAiError::ServerOverloaded;
                    } else {
                        let delay = try_parse_retry_after(&error);
                        let message = error.message.unwrap_or_default();
                        response_error = crate::error::OpenAiError::Retryable { message, delay };
                    }
                }
                return Err(ResponsesEventError::Api(response_error));
            }

            return Err(ResponsesEventError::Api(crate::error::OpenAiError::Stream(
                "response.failed event received".into(),
            )));
        }
        "response.incomplete" => {
            let reason = event.response.as_ref().and_then(|response| {
                response
                    .get("incomplete_details")
                    .and_then(|details| details.get("reason"))
                    .and_then(Value::as_str)
            });
            let reason = reason.unwrap_or("unknown");
            let message = format!("Incomplete response returned, reason: {reason}");
            return Err(ResponsesEventError::Api(crate::error::OpenAiError::Stream(
                message,
            )));
        }
        "response.completed" => {
            if let Some(resp_val) = event.response {
                match serde_json::from_value::<ResponseCompleted>(resp_val) {
                    Ok(resp) => {
                        return Ok(Some(ResponseEvent::Completed {
                            response_id: resp.id,
                            token_usage: resp.usage.map(Into::into),
                            end_turn: resp.end_turn,
                        }));
                    }
                    Err(err) => {
                        let error = format!("failed to parse ResponseCompleted: {err}");
                        debug!("{error}");
                        return Err(ResponsesEventError::Api(crate::error::OpenAiError::Stream(
                            error,
                        )));
                    }
                }
            }
        }
        "response.output_item.added" => {
            if let Some(item_val) = event.item {
                if let Ok(item) = serde_json::from_value::<ResponseItem>(item_val) {
                    return Ok(Some(ResponseEvent::OutputItemAdded(item)));
                }
                debug!("failed to parse ResponseItem from output_item.added");
            }
        }
        "response.reasoning_summary_part.added" => {
            if let Some(summary_index) = event.summary_index {
                return Ok(Some(ResponseEvent::ReasoningSummaryPartAdded {
                    summary_index,
                }));
            }
        }
        _ => {
            trace!("unhandled responses event: {}", event.kind);
        }
    }

    Ok(None)
}

pub async fn process_sse(
    stream: ByteStream,
    tx_event: mpsc::Sender<Result<ResponseEvent, crate::error::OpenAiError>>,
    idle_timeout: Duration,
    telemetry: Option<Arc<dyn SseTelemetry>>,
) {
    let mut stream = stream.eventsource();
    let mut response_error: Option<crate::error::OpenAiError> = None;
    let mut last_server_model: Option<String> = None;

    loop {
        let start = Instant::now();
        let response = timeout(idle_timeout, stream.next()).await;
        if let Some(t) = telemetry.as_ref() {
            t.on_sse_poll(&response, start.elapsed());
        }
        let sse = match response {
            Ok(Some(Ok(sse))) => sse,
            Ok(Some(Err(e))) => {
                debug!("SSE Error: {e:#}");
                let _ = tx_event
                    .send(Err(crate::error::OpenAiError::Stream(e.to_string())))
                    .await;
                return;
            }
            Ok(None) => {
                let error = response_error.unwrap_or(crate::error::OpenAiError::Stream(
                    "stream closed before response.completed".into(),
                ));
                let _ = tx_event.send(Err(error)).await;
                return;
            }
            Err(_) => {
                let _ = tx_event
                    .send(Err(crate::error::OpenAiError::Stream(
                        "idle timeout waiting for SSE".into(),
                    )))
                    .await;
                return;
            }
        };

        trace!("SSE event: {}", &sse.data);

        let event: ResponsesStreamEvent = match serde_json::from_str(&sse.data) {
            Ok(event) => event,
            Err(e) => {
                debug!("Failed to parse SSE event: {e}, data: {}", &sse.data);
                continue;
            }
        };
        let model_verifications = event.model_verifications();

        if let Some(model) = event.response_model()
            && last_server_model.as_deref() != Some(model.as_str())
        {
            if tx_event
                .send(Ok(ResponseEvent::ServerModel(model.clone())))
                .await
                .is_err()
            {
                return;
            }
            last_server_model = Some(model);
        }
        if let Some(verifications) = model_verifications
            && tx_event
                .send(Ok(ResponseEvent::ModelVerifications(verifications)))
                .await
                .is_err()
        {
            return;
        }

        match process_responses_event(event) {
            Ok(Some(event)) => {
                let is_completed = matches!(event, ResponseEvent::Completed { .. });
                if tx_event.send(Ok(event)).await.is_err() {
                    return;
                }
                if is_completed {
                    return;
                }
            }
            Ok(None) => {}
            Err(error) => {
                response_error = Some(error.into_api_error());
            }
        };
    }
}

fn try_parse_retry_after(err: &Error) -> Option<Duration> {
    if err.code.as_deref() != Some("rate_limit_exceeded") {
        return None;
    }

    let re = rate_limit_regex();
    if let Some(message) = &err.message
        && let Some(captures) = re.captures(message)
    {
        let seconds = captures.get(1);
        let unit = captures.get(2);

        if let (Some(value), Some(unit)) = (seconds, unit) {
            let value = value.as_str().parse::<f64>().ok()?;
            let unit = unit.as_str().to_ascii_lowercase();

            if unit == "s" || unit.starts_with("second") {
                return Some(Duration::from_secs_f64(value));
            } else if unit == "ms" {
                return Some(Duration::from_millis(value as u64));
            }
        }
    }
    None
}

fn is_context_window_error(error: &Error) -> bool {
    error.code.as_deref() == Some("context_length_exceeded")
}

fn is_quota_exceeded_error(error: &Error) -> bool {
    error.code.as_deref() == Some("insufficient_quota")
}

fn is_usage_not_included(error: &Error) -> bool {
    error.code.as_deref() == Some("usage_not_included")
}

fn is_invalid_prompt_error(error: &Error) -> bool {
    error.code.as_deref() == Some("invalid_prompt")
}

fn is_cyber_policy_error(error: &Error) -> bool {
    error.code.as_deref() == Some("cyber_policy")
}

fn is_server_overloaded_error(error: &Error) -> bool {
    error.code.as_deref() == Some("server_is_overloaded")
        || error.code.as_deref() == Some("slow_down")
}

fn cyber_policy_fallback_message() -> String {
    "This request has been flagged for possible cybersecurity risk.".to_string()
}

fn cyber_policy_message(message: Option<String>) -> String {
    message
        .filter(|message| !message.trim().is_empty())
        .unwrap_or_else(cyber_policy_fallback_message)
}

fn rate_limit_regex() -> &'static regex_lite::Regex {
    static RE: std::sync::OnceLock<regex_lite::Regex> = std::sync::OnceLock::new();
    #[expect(clippy::unwrap_used)]
    RE.get_or_init(|| {
        regex_lite::Regex::new(r"(?i)try again in\s*(\d+(?:\.\d+)?)\s*(s|ms|seconds?)").unwrap()
    })
}

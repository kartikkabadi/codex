// codex-wire-protocol-openai/src/websocket.rs

use std::sync::Arc;
use std::time::Duration;

use futures::SinkExt;
use futures::StreamExt;
use http::HeaderMap;
use http::HeaderValue;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::Instant;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::connect_async_tls_with_config;
use tokio_tungstenite::tungstenite::Error as WsError;
use tokio_tungstenite::tungstenite::Message;
use tracing::debug;
use tracing::instrument;
use tracing::trace;
use url::Url;

use crate::error::OpenAiError;
use crate::sse::ResponsesStreamEvent;
use crate::sse::process_responses_event;
use crate::types::ResponseEvent;
use crate::types::ResponseStream;

#[allow(dead_code)]
struct WsStream {
    tx_command: mpsc::Sender<WsCommand>,
    rx_message: mpsc::UnboundedReceiver<Result<Message, WsError>>,
    pump_task: tokio::task::JoinHandle<()>,
}

#[allow(dead_code)]
enum WsCommand {
    Send {
        message: Message,
        tx_result: oneshot::Sender<Result<(), WsError>>,
    },
}

#[allow(dead_code)]
impl WsStream {
    fn new(inner: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Self {
        let (tx_command, mut rx_command) = mpsc::channel::<WsCommand>(32);
        let (tx_message, rx_message) = mpsc::unbounded_channel::<Result<Message, WsError>>();

        let pump_task = tokio::spawn(async move {
            let mut inner = inner;
            loop {
                tokio::select! {
                    command = rx_command.recv() => {
                        let Some(command) = command else {
                            break;
                        };
                        match command {
                            WsCommand::Send { message, tx_result } => {
                                let result = inner.send(message).await;
                                let should_break = result.is_err();
                                let _ = tx_result.send(result);
                                if should_break {
                                    break;
                                }
                            }
                        }
                    }
                    message = inner.next() => {
                        let Some(message) = message else {
                            break;
                        };
                        match message {
                            Ok(Message::Ping(payload)) => {
                                if let Err(err) = inner.send(Message::Pong(payload)).await {
                                    let _ = tx_message.send(Err(err));
                                    break;
                                }
                            }
                            Ok(Message::Pong(_)) => {}
                            Ok(message @ (Message::Text(_)
                            | Message::Binary(_)
                            | Message::Close(_)
                            | Message::Frame(_))) => {
                                let is_close = matches!(message, Message::Close(_));
                                if tx_message.send(Ok(message)).is_err() {
                                    break;
                                }
                                if is_close {
                                    break;
                                }
                            }
                            Err(err) => {
                                let _ = tx_message.send(Err(err));
                                break;
                            }
                        }
                    }
                }
            }
        });

        Self {
            tx_command,
            rx_message,
            pump_task,
        }
    }

    async fn request(
        &self,
        make_command: impl FnOnce(oneshot::Sender<Result<(), WsError>>) -> WsCommand,
    ) -> Result<(), WsError> {
        let (tx_result, rx_result) = oneshot::channel();
        if self.tx_command.send(make_command(tx_result)).await.is_err() {
            return Err(WsError::ConnectionClosed);
        }
        rx_result.await.unwrap_or(Err(WsError::ConnectionClosed))
    }

    async fn send(&self, message: Message) -> Result<(), WsError> {
        self.request(|tx_result| WsCommand::Send { message, tx_result })
            .await
    }

    async fn next(&mut self) -> Option<Result<Message, WsError>> {
        self.rx_message.recv().await
    }
}

impl Drop for WsStream {
    fn drop(&mut self) {
        self.pump_task.abort();
    }
}

#[allow(dead_code)]
const X_CODEX_TURN_STATE_HEADER: &str = "x-codex-turn-state";
const X_MODELS_ETAG_HEADER: &str = "x-models-etag";
const X_REASONING_INCLUDED_HEADER: &str = "x-reasoning-included";
const OPENAI_MODEL_HEADER: &str = "openai-model";
#[allow(dead_code)]
const WEBSOCKET_CONNECTION_LIMIT_REACHED_CODE: &str = "websocket_connection_limit_reached";
#[allow(dead_code)]
const WEBSOCKET_CONNECTION_LIMIT_REACHED_MESSAGE: &str = "Responses websocket connection limit reached (60 minutes). Create a new websocket connection to continue.";

pub struct ResponsesWebsocketConnection {
    stream: Arc<Mutex<Option<WsStream>>>,
    idle_timeout: Duration,
    server_reasoning_included: bool,
    models_etag: Option<String>,
    server_model: Option<String>,
}

impl std::fmt::Debug for ResponsesWebsocketConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResponsesWebsocketConnection")
            .field("stream", &"<ws-stream>")
            .field("idle_timeout", &self.idle_timeout)
            .field("server_reasoning_included", &self.server_reasoning_included)
            .field("models_etag", &self.models_etag)
            .field("server_model", &self.server_model)
            .finish()
    }
}

impl ResponsesWebsocketConnection {
    fn new(
        stream: WsStream,
        idle_timeout: Duration,
        server_reasoning_included: bool,
        models_etag: Option<String>,
        server_model: Option<String>,
    ) -> Self {
        Self {
            stream: Arc::new(Mutex::new(Some(stream))),
            idle_timeout,
            server_reasoning_included,
            models_etag,
            server_model,
        }
    }

    /// Spawn a task that reads WebSocket messages and forwards parsed
    /// `ResponseEvent`s into the returned `ResponseStream`.
    pub fn into_response_stream(self) -> ResponseStream {
        let (tx_event, rx_event) = mpsc::channel::<Result<ResponseEvent, OpenAiError>>(1600);
        let idle_timeout = self.idle_timeout;
        let stream = self.stream.clone();

        tokio::spawn(async move {
            if let Some(model) = self.server_model {
                let _ = tx_event.send(Ok(ResponseEvent::ServerModel(model))).await;
            }
            if self.server_reasoning_included {
                let _ = tx_event
                    .send(Ok(ResponseEvent::ServerReasoningIncluded(true)))
                    .await;
            }
            if let Some(etag) = self.models_etag {
                let _ = tx_event.send(Ok(ResponseEvent::ModelsEtag(etag))).await;
            }

            let mut response_error: Option<OpenAiError> = None;
            let mut last_server_model: Option<String> = None;

            loop {
                let msg = {
                    let mut guard = stream.lock().await;
                    let Some(ws) = guard.as_mut() else {
                        break;
                    };
                    let _start = Instant::now();
                    match tokio::time::timeout(idle_timeout, ws.next()).await {
                        Ok(Some(result)) => result,
                        Ok(None) => {
                            let error = response_error.unwrap_or(OpenAiError::Stream(
                                "stream closed before response.completed".into(),
                            ));
                            let _ = tx_event.send(Err(error)).await;
                            break;
                        }
                        Err(_) => {
                            let _ = tx_event
                                .send(Err(OpenAiError::Stream(
                                    "idle timeout waiting for WebSocket message".into(),
                                )))
                                .await;
                            break;
                        }
                    }
                };

                match msg {
                    Ok(Message::Text(text)) => {
                        trace!("WS text: {}", text);
                        let event: ResponsesStreamEvent = match serde_json::from_str(&text) {
                            Ok(event) => event,
                            Err(e) => {
                                debug!("Failed to parse WS event: {e}, data: {}", text);
                                continue;
                            }
                        };

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
                        }
                    }
                    Ok(Message::Close(_)) => {
                        let error = response_error.unwrap_or(OpenAiError::Stream(
                            "WebSocket closed before response.completed".into(),
                        ));
                        let _ = tx_event.send(Err(error)).await;
                        break;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        debug!("WS Error: {e:#}");
                        let _ = tx_event
                            .send(Err(OpenAiError::WebSocket(e.to_string())))
                            .await;
                        break;
                    }
                }
            }
        });

        ResponseStream {
            rx_event,
            upstream_request_id: None,
        }
    }
}

/// Open a WebSocket connection to an OpenAI Responses endpoint.
#[instrument(level = "info", skip_all)]
pub async fn connect_responses_websocket(
    base_url: &str,
    api_key: &str,
    extra_headers: HeaderMap,
    idle_timeout: Duration,
) -> Result<ResponsesWebsocketConnection, OpenAiError> {
    let url = Url::parse(base_url).map_err(|e| OpenAiError::Stream(e.to_string()))?;
    let mut req = http::Request::builder()
        .uri(url.as_str())
        .body(())
        .map_err(|e| OpenAiError::Stream(e.to_string()))?;
    req.headers_mut().insert(
        http::header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {api_key}"))
            .map_err(|e| OpenAiError::Stream(e.to_string()))?,
    );

    for (key, value) in extra_headers.iter() {
        req.headers_mut().insert(key.clone(), value.clone());
    }

    let (ws_stream, resp) = connect_async_tls_with_config(req, None, false, None)
        .await
        .map_err(|e| OpenAiError::WebSocket(e.to_string()))?;

    let server_reasoning_included = resp.headers().get(X_REASONING_INCLUDED_HEADER).is_some();
    let models_etag = resp
        .headers()
        .get(X_MODELS_ETAG_HEADER)
        .and_then(|v: &HeaderValue| v.to_str().ok())
        .map(ToString::to_string);
    let server_model = resp
        .headers()
        .get(OPENAI_MODEL_HEADER)
        .and_then(|v: &HeaderValue| v.to_str().ok())
        .map(ToString::to_string);

    let ws = WsStream::new(ws_stream);
    Ok(ResponsesWebsocketConnection::new(
        ws,
        idle_timeout,
        server_reasoning_included,
        models_etag,
        server_model,
    ))
}

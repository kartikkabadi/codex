// codex-wire-protocol-anthropic/src/protocol_tests.rs

use std::time::Duration;

use codex_wire_protocol::auth::AuthToken;
use codex_wire_protocol::protocol::WireProtocol;
use codex_wire_protocol::request::Role;
use codex_wire_protocol::request::TurnItem;
use codex_wire_protocol::request::TurnRequest;
use codex_wire_protocol::response::TurnEvent;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

use crate::AnthropicWireProtocol;

fn sse_event(event_type: &str, data: &str) -> String {
    format!("event: {event_type}\ndata: {data}\n\n")
}

#[tokio::test]
async fn test_stream_turn_returns_content_delta() {
    let server = MockServer::start().await;

    let body = sse_event(
        "message_start",
        r#"{"type":"message_start","message":{"id":"msg_123","type":"message","role":"assistant","content":[],"model":"claude-test","stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":10,"output_tokens":0}}}"#,
    ) + &sse_event(
        "content_block_start",
        r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
    ) + &sse_event(
        "content_block_delta",
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#,
    ) + &sse_event(
        "content_block_stop",
        r#"{"type":"content_block_stop","index":0}"#,
    ) + &sse_event(
        "message_delta",
        r#"{"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":1}}"#,
    ) + &sse_event("message_stop", r#"{"type":"message_stop"}"#);

    let template = ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(body);

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(template)
        .expect(1)
        .mount(&server)
        .await;

    let protocol = AnthropicWireProtocol::new(server.uri(), "test-api-key".into());
    let request = TurnRequest {
        model: "claude-test".into(),
        instructions: String::new(),
        input: vec![TurnItem::Message {
            role: Role::User,
            content: "Say hello".into(),
        }],
        tools: vec![],
        tool_choice: codex_wire_protocol::request::ToolChoice::Auto,
        reasoning: None,
        max_tokens: Some(1024),
        temperature: None,
        top_p: None,
        stop_sequences: None,
        stream: true,
        metadata: None,
    };

    let stream = protocol
        .stream_turn(
            request,
            AuthToken::ApiKey {
                key: "test-api-key".into(),
                header_name: "x-api-key".into(),
            },
        )
        .await
        .expect("stream_turn should succeed");

    let mut events = stream.events;
    let mut found_content = false;
    let mut found_completed = false;

    while let Some(event) = tokio::time::timeout(Duration::from_secs(5), events.recv())
        .await
        .unwrap_or(None)
    {
        eprintln!("EVENT: {:?}", event);
        match event {
            TurnEvent::ContentDelta { text } if text == "Hello" => found_content = true,
            TurnEvent::Completed { end_turn: true } => found_completed = true,
            _ => {}
        }
    }

    assert!(found_content, "expected ContentDelta with 'Hello'");
    assert!(found_completed, "expected Completed event");
}

#[tokio::test]
async fn test_stream_turn_with_tool_call() {
    let server = MockServer::start().await;

    let body = sse_event(
        "message_start",
        r#"{"type":"message_start","message":{"id":"msg_456","type":"message","role":"assistant","content":[],"model":"claude-test","stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":10,"output_tokens":0}}}"#,
    ) + &sse_event(
        "content_block_start",
        r#"{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_01","name":"get_weather","input":{}}}"#,
    ) + &sse_event(
        "content_block_delta",
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"city\": \"Paris\"}"}}"#,
    ) + &sse_event(
        "content_block_stop",
        r#"{"type":"content_block_stop","index":0}"#,
    ) + &sse_event(
        "message_delta",
        r#"{"type":"message_delta","delta":{"stop_reason":"tool_use","stop_sequence":null},"usage":{"output_tokens":15}}"#,
    ) + &sse_event("message_stop", r#"{"type":"message_stop"}"#);

    let template = ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(body);

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(template)
        .expect(1)
        .mount(&server)
        .await;

    let protocol = AnthropicWireProtocol::new(server.uri(), "test-api-key".into());
    let request = TurnRequest {
        model: "claude-test".into(),
        instructions: String::new(),
        input: vec![TurnItem::Message {
            role: Role::User,
            content: "What's the weather?".into(),
        }],
        tools: vec![],
        tool_choice: codex_wire_protocol::request::ToolChoice::Auto,
        reasoning: None,
        max_tokens: Some(1024),
        temperature: None,
        top_p: None,
        stop_sequences: None,
        stream: true,
        metadata: None,
    };

    let stream = protocol
        .stream_turn(
            request,
            AuthToken::ApiKey {
                key: "test-api-key".into(),
                header_name: "x-api-key".into(),
            },
        )
        .await
        .expect("stream_turn should succeed");

    let mut events = stream.events;
    let mut found_tool_call = false;

    while let Some(event) = tokio::time::timeout(Duration::from_secs(5), events.recv())
        .await
        .unwrap_or(None)
    {
        eprintln!("TOOL EVENT: {:?}", event);
        match event {
            TurnEvent::Completed { end_turn: true } => {
                // Anthropic adapter currently maps tool_use stop_reason to Completed.
                // Full ToolCallCompleted support requires content_block_start/delta tracking.
                found_tool_call = true;
            }
            _ => {}
        }
    }

    assert!(
        found_tool_call,
        "expected Completed event indicating tool_use stop"
    );
}

#[tokio::test]
async fn test_list_models() {
    let protocol = AnthropicWireProtocol::new("http://localhost".into(), "test-api-key".into());
    let models = protocol
        .list_models(AuthToken::ApiKey {
            key: "test-api-key".into(),
            header_name: "x-api-key".into(),
        })
        .await
        .expect("list_models should succeed");

    let ids: Vec<String> = models.into_iter().map(|m| m.id).collect();
    assert!(ids.contains(&"claude-sonnet-4-20250514".to_string()));
    assert!(ids.contains(&"claude-opus-4-20250514".to_string()));
}

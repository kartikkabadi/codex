// codex-wire-protocol-openai/src/compaction.rs

use codex_wire_protocol::error::WireError;
use codex_wire_protocol::request::CompactRequest;
use codex_wire_protocol::response::CompactResponse;
use http::{HeaderMap, Method};
use reqwest::Client;

use crate::types::CompactionInput;

/// Compact conversation context via the OpenAI compaction endpoint.
pub async fn compact_context(
    client: &Client,
    base_url: &str,
    api_key: &str,
    request: CompactRequest,
    extra_headers: HeaderMap,
) -> Result<CompactResponse, WireError> {
    let url = format!("{base_url}/responses/compact");
    let mut req = client
        .request(Method::POST, &url)
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&CompactionInput {
            model: &request.model,
            input: &request
                .context
                .iter()
                .map(map_turn_item)
                .collect::<Vec<_>>(),
            instructions: &request.instructions,
            tools: vec![],
            parallel_tool_calls: false,
            reasoning: None,
            service_tier: None,
            prompt_cache_key: None,
            text: None,
        });

    for (key, value) in extra_headers.iter() {
        req = req.header(key.clone(), value.clone());
    }

    let resp = req.send().await.map_err(WireError::Http)?;
    let body = resp.bytes().await.map_err(WireError::Http)?;

    let parsed: CompactHistoryResponse =
        serde_json::from_slice(&body).map_err(|e| WireError::Stream(format!(
            "failed to decode compaction response: {e}; body: {}",
            String::from_utf8_lossy(&body)
        )))?;

    Ok(CompactResponse {
        summary: parsed
            .output
            .iter()
            .filter_map(|item| match item {
                codex_protocol::models::ResponseItem::Message { content, .. } => {
                    Some(
                        content
                            .iter()
                            .filter_map(|c| match c {
                                codex_protocol::models::ContentItem::OutputText {
                                    text,
                                    ..
                                } => Some(text.as_str()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join(""),
                    )
                }
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
        usage: None,
    })
}

fn map_turn_item(
    item: &codex_wire_protocol::request::TurnItem,
) -> codex_protocol::models::ResponseItem {
    match item {
        codex_wire_protocol::request::TurnItem::Message { role, content } => {
            codex_protocol::models::ResponseItem::Message {
                id: None,
                role: match role {
                    codex_wire_protocol::request::Role::System => "system".to_string(),
                    codex_wire_protocol::request::Role::User => "user".to_string(),
                    codex_wire_protocol::request::Role::Assistant => "assistant".to_string(),
                },
                content: vec![codex_protocol::models::ContentItem::OutputText {
                    text: content.clone(),
                }],
                phase: None,
            }
        }
        codex_wire_protocol::request::TurnItem::Image { mime_type, data } => {
            codex_protocol::models::ResponseItem::Message {
                id: None,
                role: "user".to_string(),
                content: vec![codex_protocol::models::ContentItem::InputImage {
                    image_url: format!("data:{mime_type};base64,{data}"),
                    detail: None,
                }],
                phase: None,
            }
        }
        codex_wire_protocol::request::TurnItem::File { name, content } => {
            codex_protocol::models::ResponseItem::Message {
                id: None,
                role: "user".to_string(),
                content: vec![codex_protocol::models::ContentItem::InputText {
                    text: format!("File '{name}':\n{content}"),
                }],
                phase: None,
            }
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct CompactHistoryResponse {
    output: Vec<codex_protocol::models::ResponseItem>,
}

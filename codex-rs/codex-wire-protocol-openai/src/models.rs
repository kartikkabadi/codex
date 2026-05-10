// codex-wire-protocol-openai/src/models.rs

use codex_protocol::openai_models::{ModelInfo, ModelsResponse};
use codex_wire_protocol::error::WireError;
use codex_wire_protocol::response::ModelInfo as WireModelInfo;
use http::{HeaderMap, Method, header::ETAG};
use reqwest::Client;

/// List available models from an OpenAI-compatible endpoint.
pub async fn list_models(
    client: &Client,
    base_url: &str,
    api_key: &str,
    client_version: &str,
    extra_headers: HeaderMap,
) -> Result<(Vec<WireModelInfo>, Option<String>), WireError> {
    let url = format!("{base_url}/models?client_version={client_version}");
    let mut req = client
        .request(Method::GET, &url)
        .header("Authorization", format!("Bearer {api_key}"));

    for (key, value) in extra_headers.iter() {
        req = req.header(key.clone(), value.clone());
    }

    let resp = req.send().await.map_err(WireError::Http)?;
    let header_etag = resp
        .headers()
        .get(ETAG)
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string);

    let body = resp.bytes().await.map_err(WireError::Http)?;
    let ModelsResponse { models } =
        serde_json::from_slice::<ModelsResponse>(&body).map_err(|e| {
            WireError::Stream(format!(
                "failed to decode models response: {e}; body: {}",
                String::from_utf8_lossy(&body)
            ))
        })?;

    let mapped = models.into_iter().map(map_model_info).collect();
    Ok((mapped, header_etag))
}

fn map_model_info(m: ModelInfo) -> WireModelInfo {
    let context_window = m.resolved_context_window().unwrap_or(0) as u64;
    WireModelInfo {
        id: m.slug,
        display_name: m.display_name,
        context_window,
        supports_tools: m.supports_parallel_tool_calls,
        supports_vision: m.supports_image_detail_original,
        supports_reasoning: m.supports_reasoning_summaries,
        supports_realtime: false,
        max_output_tokens: None,
    }
}

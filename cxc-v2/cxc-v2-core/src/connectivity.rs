use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::protocol::{self, Protocol};
use crate::supplier::Supplier;

pub use crate::protocol::fetch_models;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ModelTestResult {
    pub ok: bool,
    pub latency_ms: i64,
    pub status: Option<u16>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ModelTestOutcome {
    pub model: String,
    pub protocol: Protocol,
    pub result: ModelTestResult,
}

pub async fn test_model(
    supplier: &Supplier,
    api_key: &str,
    model: &str,
    protocol: Protocol,
    client: &reqwest::Client,
) -> ModelTestResult {
    let started = Instant::now();
    let url = protocol.endpoint(&supplier.base_url);
    let request = client
        .post(&url)
        .timeout(Duration::from_secs(20))
        .json(&protocol.request_body(model, false));
    let request = protocol.apply_headers(request, api_key);

    match request.send().await {
        Ok(response) => {
            let status = response.status();
            let code = status.as_u16();
            let body = response.text().await.unwrap_or_default();
            let latency_ms = started.elapsed().as_millis() as i64;

            if status.is_success() {
                if extract_text(protocol, &body).is_some() {
                    ModelTestResult {
                        ok: true,
                        latency_ms,
                        status: Some(code),
                        error: None,
                    }
                } else {
                    ModelTestResult {
                        ok: false,
                        latency_ms,
                        status: Some(code),
                        error: Some("响应中没有文本或思考内容".to_string()),
                    }
                }
            } else {
                ModelTestResult {
                    ok: false,
                    latency_ms,
                    status: Some(code),
                    error: Some(categorize_api_error(code, &body)),
                }
            }
        }
        Err(error) => ModelTestResult {
            ok: false,
            latency_ms: started.elapsed().as_millis() as i64,
            status: None,
            error: Some(protocol::categorize_network_error(&error)),
        },
    }
}

/// 批量测试供应商模型。并发上限 4。
pub async fn test_supplier_models(
    supplier: &Supplier,
    api_key: &str,
    client: &reqwest::Client,
) -> Vec<ModelTestOutcome> {
    let mut tasks: Vec<(String, Protocol)> = Vec::new();
    for entry in &supplier.models {
        let protocols = if entry.protocols.is_empty() {
            if supplier.protocols.is_empty() {
                Protocol::ALL.to_vec()
            } else {
                supplier.protocols.clone()
            }
        } else {
            entry.protocols.clone()
        };
        for protocol in protocols {
            tasks.push((entry.model.clone(), protocol));
        }
    }

    if tasks.is_empty() {
        return Vec::new();
    }

    let semaphore = Arc::new(Semaphore::new(4));
    let mut set = JoinSet::new();
    for (model, protocol) in tasks {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let supplier = supplier.clone();
        let api_key = api_key.to_string();
        let client = client.clone();
        set.spawn(async move {
            let _permit = permit;
            let result = test_model(&supplier, &api_key, &model, protocol, &client).await;
            ModelTestOutcome {
                model,
                protocol,
                result,
            }
        });
    }

    let mut outcomes = Vec::new();
    while let Some(result) = set.join_next().await {
        if let Ok(outcome) = result {
            outcomes.push(outcome);
        }
    }
    outcomes.sort_by(|left, right| {
        left.model
            .cmp(&right.model)
            .then(left.protocol.cmp(&right.protocol))
    });
    outcomes
}

pub fn extract_text(protocol: Protocol, body: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    let text = match protocol {
        Protocol::ChatCompletions => chat_text(&value),
        Protocol::Responses => responses_text(&value),
        Protocol::Messages => messages_text(&value),
    };
    text.filter(|text| !text.trim().is_empty())
}

fn chat_text(value: &Value) -> Option<String> {
    let choice = value.get("choices")?.as_array()?.first()?;
    if let Some(message) = choice.get("message") {
        if let Some(text) = message.get("content").and_then(content_text) {
            return Some(text);
        }
        for key in ["reasoning_content", "reasoning"] {
            if let Some(text) = message.get(key).and_then(|value| value.as_str())
                && !text.trim().is_empty()
            {
                return Some(text.to_string());
            }
        }
    }
    choice
        .get("text")
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn responses_text(value: &Value) -> Option<String> {
    if let Some(text) = value.get("output_text").and_then(|value| value.as_str())
        && !text.trim().is_empty()
    {
        return Some(text.to_string());
    }
    let output = value.get("output")?.as_array()?;
    let mut reasoning = None;
    for item in output {
        if let Some(content) = item.get("content").and_then(|value| value.as_array()) {
            for part in content {
                if let Some(text) = part.get("text").and_then(|value| value.as_str())
                    && !text.trim().is_empty()
                {
                    return Some(text.to_string());
                }
            }
        }
        if let Some(summary) = item.get("summary").and_then(|value| value.as_array()) {
            for part in summary {
                if let Some(text) = part.get("text").and_then(|value| value.as_str())
                    && !text.trim().is_empty()
                {
                    reasoning = Some(text.to_string());
                }
            }
        }
    }
    reasoning
}

fn messages_text(value: &Value) -> Option<String> {
    if let Some(text) = value.get("completion").and_then(|value| value.as_str())
        && !text.trim().is_empty()
    {
        return Some(text.to_string());
    }
    let content = value.get("content")?.as_array()?;
    let mut thinking = None;
    for block in content {
        let kind = block
            .get("type")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        if kind == "text"
            && let Some(text) = block.get("text").and_then(|value| value.as_str())
            && !text.trim().is_empty()
        {
            return Some(text.to_string());
        }
        if kind == "thinking"
            && let Some(text) = block.get("thinking").and_then(|value| value.as_str())
            && !text.trim().is_empty()
        {
            thinking = Some(text.to_string());
        }
    }
    thinking
}

fn content_text(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    let parts = value.as_array()?;
    let mut result = String::new();
    for part in parts {
        if let Some(text) = part.get("text").and_then(|value| value.as_str()) {
            result.push_str(text);
        }
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn categorize_api_error(status: u16, body: &str) -> String {
    let detail = truncate(body, 200);
    match status {
        401 | 403 => "鉴权失败，请检查 API Key".to_string(),
        404 => "模型或端点不存在".to_string(),
        429 => "请求过于频繁，稍后重试".to_string(),
        _ => {
            if detail.is_empty() {
                format!("HTTP {status}")
            } else {
                format!("HTTP {status}: {detail}")
            }
        }
    }
}

fn truncate(value: &str, limit: usize) -> String {
    let value = value.trim();
    if value.chars().count() <= limit {
        value.to_string()
    } else {
        let truncated: String = value.chars().take(limit).collect();
        format!("{truncated}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supplier::Supplier;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_supplier(base_url: String) -> Supplier {
        Supplier {
            id: "supplier-1".to_string(),
            name: "relay".to_string(),
            base_url,
            keys: vec![crate::supplier::SupplierKey {
                id: "k1".to_string(),
                name: "默认".to_string(),
                api_key: "sk-test".to_string(),
                remark: None,
            }],
            default_key_id: Some("k1".to_string()),
            protocols: vec![Protocol::ChatCompletions],
            remark: None,
            last_probe: None,
            models: Vec::new(),
        }
    }

    #[tokio::test]
    async fn test_model_accepts_chat_text() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{ "message": { "content": "hello" } }]
            })))
            .mount(&server)
            .await;

        let result = test_model(
            &test_supplier(server.uri()),
            "sk-test",
            "test-model",
            Protocol::ChatCompletions,
            &reqwest::Client::new(),
        )
        .await;
        assert!(result.ok);
        assert_eq!(result.status, Some(200));
        assert!(result.latency_ms >= 0);
    }

    #[tokio::test]
    async fn test_model_reports_auth_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(401).set_body_json(serde_json::json!({"error": "no"})),
            )
            .mount(&server)
            .await;

        let result = test_model(
            &test_supplier(server.uri()),
            "sk-test",
            "test-model",
            Protocol::ChatCompletions,
            &reqwest::Client::new(),
        )
        .await;
        assert!(!result.ok);
        assert!(result.error.unwrap().contains("鉴权"));
    }

    #[test]
    fn extracts_chat_text_and_reasoning() {
        let body = r#"{"choices":[{"message":{"content":"hello"}}]}"#;
        assert_eq!(
            extract_text(Protocol::ChatCompletions, body).as_deref(),
            Some("hello")
        );
        let body = r#"{"choices":[{"message":{"content":null,"reasoning_content":"think"}}]}"#;
        assert_eq!(
            extract_text(Protocol::ChatCompletions, body).as_deref(),
            Some("think")
        );
    }

    #[test]
    fn extracts_messages_text_and_thinking() {
        let body = r#"{"content":[{"type":"text","text":"hello"}]}"#;
        assert_eq!(
            extract_text(Protocol::Messages, body).as_deref(),
            Some("hello")
        );
        let body = r#"{"content":[{"type":"thinking","thinking":"think"}]}"#;
        assert_eq!(
            extract_text(Protocol::Messages, body).as_deref(),
            Some("think")
        );
    }

    #[test]
    fn extracts_responses_output() {
        let body = r#"{"output":[{"content":[{"type":"output_text","text":"hello"}]}]}"#;
        assert_eq!(
            extract_text(Protocol::Responses, body).as_deref(),
            Some("hello")
        );
    }
}

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;
use thiserror::Error;

use crate::supplier::Supplier;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    ChatCompletions,
    Responses,
    Messages,
}

impl Protocol {
    pub const ALL: [Protocol; 3] = [
        Protocol::ChatCompletions,
        Protocol::Responses,
        Protocol::Messages,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Protocol::ChatCompletions => "chat_completions",
            Protocol::Responses => "responses",
            Protocol::Messages => "messages",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Protocol::ChatCompletions => "OpenAI Chat",
            Protocol::Responses => "OpenAI Responses",
            Protocol::Messages => "Anthropic Messages",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "chat_completions" | "chat" => Some(Protocol::ChatCompletions),
            "responses" => Some(Protocol::Responses),
            "messages" | "anthropic" => Some(Protocol::Messages),
            _ => None,
        }
    }

    pub fn normalize_all(values: impl IntoIterator<Item = Protocol>) -> Vec<Protocol> {
        let mut result: Vec<Protocol> = Vec::new();
        for protocol in values {
            if !result.contains(&protocol) {
                result.push(protocol);
            }
        }
        result.sort();
        result
    }

    pub fn endpoint(self, base_url: &str) -> String {
        let base = base_url.trim().trim_end_matches('/');
        match self {
            Protocol::ChatCompletions => format!("{base}/chat/completions"),
            Protocol::Responses => format!("{base}/responses"),
            Protocol::Messages => {
                if base.ends_with("/v1/messages") {
                    base.to_string()
                } else if base.ends_with("/v1") {
                    format!("{base}/messages")
                } else {
                    format!("{base}/v1/messages")
                }
            }
        }
    }

    pub fn request_body(self, model: &str, probe: bool) -> Value {
        let max_tokens = if probe { 1 } else { 16 };
        match self {
            Protocol::ChatCompletions => json!({
                "model": model,
                "messages": [{ "role": "user", "content": "hi" }],
                "max_tokens": max_tokens,
                "stream": false
            }),
            Protocol::Responses => json!({
                "model": model,
                "input": "hi",
                "max_output_tokens": if probe { 16 } else { 32 },
                "stream": false
            }),
            Protocol::Messages => json!({
                "model": model,
                "max_tokens": max_tokens,
                "messages": [{ "role": "user", "content": "hi" }]
            }),
        }
    }

    pub fn apply_headers(
        self,
        request: reqwest::RequestBuilder,
        api_key: &str,
    ) -> reqwest::RequestBuilder {
        match self {
            Protocol::Messages => {
                let request = request.header("anthropic-version", "2023-06-01");
                if api_key.trim().starts_with("sk-ant-") {
                    request.header("x-api-key", api_key.trim())
                } else {
                    request.bearer_auth(api_key.trim())
                }
            }
            _ => request.bearer_auth(api_key.trim()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ProtocolProbeResult {
    pub protocol: Protocol,
    pub confirmed: bool,
    pub status: Option<u16>,
    pub reason: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ProbeReport {
    pub added: Vec<Protocol>,
    pub protocols: Vec<Protocol>,
    pub results: Vec<ProtocolProbeResult>,
}

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("请求失败: {0}")]
    Http(String),
    #[error("响应解析失败: {0}")]
    Parse(String),
}

pub async fn probe_supplier(supplier: &Supplier, client: &reqwest::Client) -> ProbeReport {
    let model = supplier
        .models
        .first()
        .map(|entry| entry.model.clone())
        .filter(|model| !model.trim().is_empty())
        .unwrap_or_else(|| "cxc-probe".to_string());

    let (chat, responses, messages) = tokio::join!(
        probe_one(client, supplier, Protocol::ChatCompletions, &model),
        probe_one(client, supplier, Protocol::Responses, &model),
        probe_one(client, supplier, Protocol::Messages, &model),
    );

    let results = vec![chat, responses, messages];
    let confirmed: Vec<Protocol> = results
        .iter()
        .filter(|result| result.confirmed)
        .map(|result| result.protocol)
        .collect();
    let added: Vec<Protocol> = confirmed
        .iter()
        .copied()
        .filter(|protocol| !supplier.protocols.contains(protocol))
        .collect();
    let protocols = Protocol::normalize_all(supplier.protocols.iter().copied().chain(confirmed));

    ProbeReport {
        added,
        protocols,
        results,
    }
}

async fn probe_one(
    client: &reqwest::Client,
    supplier: &Supplier,
    protocol: Protocol,
    model: &str,
) -> ProtocolProbeResult {
    let url = protocol.endpoint(&supplier.base_url);
    let request = client
        .post(&url)
        .timeout(Duration::from_secs(10))
        .json(&protocol.request_body(model, true));
    let request = protocol.apply_headers(request, &supplier.api_key);

    match request.send().await {
        Ok(response) => {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let json_object = serde_json::from_str::<Value>(&body)
                .map(|value| value.is_object())
                .unwrap_or(false);
            let code = status.as_u16();

            if status.is_success() && json_object {
                ProtocolProbeResult {
                    protocol,
                    confirmed: true,
                    status: Some(code),
                    reason: "端点返回了 JSON 响应".to_string(),
                }
            } else if matches!(code, 400 | 404 | 415 | 422) && json_object {
                ProtocolProbeResult {
                    protocol,
                    confirmed: true,
                    status: Some(code),
                    reason: format!("端点存在，返回 {code} 与 JSON 错误体"),
                }
            } else if matches!(code, 401 | 403) {
                ProtocolProbeResult {
                    protocol,
                    confirmed: false,
                    status: Some(code),
                    reason: "鉴权失败，请检查 API Key".to_string(),
                }
            } else if code == 429 {
                ProtocolProbeResult {
                    protocol,
                    confirmed: false,
                    status: Some(code),
                    reason: "请求被限流，稍后重试".to_string(),
                }
            } else {
                ProtocolProbeResult {
                    protocol,
                    confirmed: false,
                    status: Some(code),
                    reason: if json_object {
                        format!("端点返回 {code}")
                    } else {
                        format!("端点返回 {code}，正文不是 JSON")
                    },
                }
            }
        }
        Err(error) => ProtocolProbeResult {
            protocol,
            confirmed: false,
            status: None,
            reason: categorize_network_error(&error),
        },
    }
}

pub async fn fetch_models(
    base_url: &str,
    api_key: &str,
    client: &reqwest::Client,
) -> Result<Vec<String>, ProtocolError> {
    let base = base_url.trim().trim_end_matches('/');
    let url = format!("{base}/models");
    let response = client
        .get(&url)
        .timeout(Duration::from_secs(15))
        .bearer_auth(api_key.trim())
        .send()
        .await
        .map_err(|error| ProtocolError::Http(categorize_network_error(&error)))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| ProtocolError::Http(categorize_network_error(&error)))?;

    if !status.is_success() {
        return Err(ProtocolError::Http(format!(
            "模型列表请求失败：HTTP {}",
            status.as_u16()
        )));
    }

    let value: Value = serde_json::from_str(&body)
        .map_err(|error| ProtocolError::Parse(format!("模型列表不是 JSON：{error}")))?;
    let mut models = Vec::new();

    let arrays = [
        value.get("data").and_then(Value::as_array),
        value.get("models").and_then(Value::as_array),
        value.as_array(),
    ];
    for array in arrays.into_iter().flatten() {
        for item in array {
            let id = item
                .get("id")
                .and_then(Value::as_str)
                .or_else(|| item.get("name").and_then(Value::as_str))
                .or_else(|| item.as_str());
            if let Some(id) = id {
                let id = id.trim();
                if !id.is_empty() && !models.iter().any(|existing| existing == id) {
                    models.push(id.to_string());
                }
            }
        }
    }

    if models.is_empty() {
        return Err(ProtocolError::Parse("模型列表为空".to_string()));
    }
    Ok(models)
}

pub fn categorize_network_error(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        "请求超时".to_string()
    } else if error.is_connect() {
        "无法连接到端点".to_string()
    } else if error.is_decode() {
        "响应解析失败".to_string()
    } else {
        format!("网络错误：{error}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supplier::{Supplier, SupplierModel};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_supplier(base_url: String) -> Supplier {
        Supplier {
            id: "supplier-1".to_string(),
            name: "relay".to_string(),
            base_url,
            api_key: "sk-test".to_string(),
            protocols: Vec::new(),
            remark: None,
            last_probe: None,
            models: vec![SupplierModel {
                model: "test-model".to_string(),
                ..SupplierModel::default()
            }],
        }
    }

    #[tokio::test]
    async fn probe_confirms_all_three_protocols() {
        let server = MockServer::start().await;
        for endpoint in ["/chat/completions", "/responses", "/v1/messages"] {
            Mock::given(method("POST"))
                .and(path(endpoint))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})),
                )
                .mount(&server)
                .await;
        }

        let report = probe_supplier(&test_supplier(server.uri()), &reqwest::Client::new()).await;
        assert!(report.results.iter().all(|result| result.confirmed));
        assert_eq!(report.added, Protocol::ALL.to_vec());
        assert_eq!(report.protocols, Protocol::ALL.to_vec());
    }

    #[tokio::test]
    async fn probe_keeps_auth_failure_out_and_accepts_json_errors() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(401).set_body_json(serde_json::json!({"error": "no"})),
            )
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/responses"))
            .respond_with(
                ResponseTemplate::new(404).set_body_json(serde_json::json!({"error": "no"})),
            )
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(
                ResponseTemplate::new(400).set_body_json(serde_json::json!({"error": "no"})),
            )
            .mount(&server)
            .await;

        let report = probe_supplier(&test_supplier(server.uri()), &reqwest::Client::new()).await;
        let chat = report
            .results
            .iter()
            .find(|result| result.protocol == Protocol::ChatCompletions)
            .unwrap();
        assert!(!chat.confirmed);
        assert!(chat.reason.contains("鉴权"));
        assert_eq!(
            report.protocols,
            vec![Protocol::Responses, Protocol::Messages]
        );
    }

    #[test]
    fn endpoint_joins_paths() {
        assert_eq!(
            Protocol::ChatCompletions.endpoint("https://api.example.com/v1/"),
            "https://api.example.com/v1/chat/completions"
        );
        assert_eq!(
            Protocol::Responses.endpoint("https://api.example.com/v1"),
            "https://api.example.com/v1/responses"
        );
        assert_eq!(
            Protocol::Messages.endpoint("https://api.example.com"),
            "https://api.example.com/v1/messages"
        );
        assert_eq!(
            Protocol::Messages.endpoint("https://api.example.com/v1"),
            "https://api.example.com/v1/messages"
        );
        assert_eq!(
            Protocol::Messages.endpoint("https://api.example.com/v1/messages"),
            "https://api.example.com/v1/messages"
        );
    }

    #[test]
    fn normalize_protocols_orders_and_dedupes() {
        let protocols = Protocol::normalize_all([
            Protocol::Messages,
            Protocol::ChatCompletions,
            Protocol::Messages,
            Protocol::Responses,
        ]);
        assert_eq!(
            protocols,
            vec![
                Protocol::ChatCompletions,
                Protocol::Responses,
                Protocol::Messages
            ]
        );
    }

    #[test]
    fn parse_accepts_aliases() {
        assert_eq!(Protocol::parse("chat"), Some(Protocol::ChatCompletions));
        assert_eq!(Protocol::parse("anthropic"), Some(Protocol::Messages));
        assert_eq!(Protocol::parse("unknown"), None);
    }
}

use serde::{Deserialize, Serialize};
use std::time::Instant;
use reqwest::Client;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Result {
    pub ok: bool,
    pub latency_ms: i64,
    pub response: String, // truncated model response on success
    pub error: String,    // categorized error on failure
}

pub struct Tester {
    client: Client,
}

impl Default for Tester {
    fn default() -> Self {
        Self::new()
    }
}

impl Tester {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { client }
    }

    pub fn new_with_client(client: Client) -> Self {
        Self { client }
    }

    pub async fn test(&self, base_url: &str, api_key: &str, model: &str) -> Result {
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

        let req_body = ChatRequest {
            model: model.to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "say hi".to_string(),
            }],
            max_tokens: Some(1),
        };

        let start = Instant::now();
        let res = self.client.post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&req_body)
            .send()
            .await;

        let latency_ms = start.elapsed().as_millis() as i64;

        let resp = match res {
            Ok(r) => r,
            Err(e) => {
                return Result {
                    ok: false,
                    latency_ms,
                    response: String::new(),
                    error: categorize_network_error(e),
                };
            }
        };

        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Result {
                ok: false,
                latency_ms,
                response: String::new(),
                error: "invalid API key (auth error)".to_string(),
            };
        }
        if status == reqwest::StatusCode::NOT_FOUND {
            return Result {
                ok: false,
                latency_ms,
                response: String::new(),
                error: "endpoint not found — check base_url".to_string(),
            };
        }

        let raw_body = match resp.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => {
                return Result {
                    ok: false,
                    latency_ms,
                    response: String::new(),
                    error: format!("reading response: {}", e),
                };
            }
        };

        let body_str = String::from_utf8_lossy(&raw_body);

        let chat_resp: ChatResponse = match serde_json::from_slice(&raw_body) {
            Ok(cr) => cr,
            Err(_) => {
                return Result {
                    ok: false,
                    latency_ms,
                    response: String::new(),
                    error: format!("invalid response (status {}): {}", status.as_u16(), truncate(&body_str, 100)),
                };
            }
        };

        if let Some(err) = chat_resp.error {
            return Result {
                ok: false,
                latency_ms,
                response: String::new(),
                error: categorize_api_error(&err),
            };
        }

        if !status.is_success() {
            return Result {
                ok: false,
                latency_ms,
                response: String::new(),
                error: format!("unexpected status {}: {}", status.as_u16(), truncate(&body_str, 100)),
            };
        }

        let choices = match chat_resp.choices {
            Some(c) => c,
            None => {
                return Result {
                    ok: false,
                    latency_ms,
                    response: String::new(),
                    error: "no choices returned".to_string(),
                };
            }
        };

        if choices.is_empty() {
            return Result {
                ok: false,
                latency_ms,
                response: String::new(),
                error: "no choices returned".to_string(),
            };
        }

        let content = &choices[0].message.content;
        Result {
            ok: true,
            latency_ms,
            response: truncate(content, 50),
            error: String::new(),
        }
    }
}

// ── Model Discovery ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
}

/// Fetch the list of model IDs from `GET {base_url}/models`.
/// Returns `Ok(vec![])` if the endpoint returns an empty list.
/// Returns `Err` on network failure or non-2xx status.
pub async fn fetch_models(base_url: &str, api_key: &str) -> anyhow::Result<Vec<String>> {
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| Client::new());

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("network error: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        return Err(anyhow::anyhow!("HTTP {} from /models", status.as_u16()));
    }

    let models_resp: ModelsResponse = resp
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("failed to parse /models response: {}", e))?;

    Ok(models_resp.data.into_iter().map(|m| m.id).collect())
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: Option<u32>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Option<Vec<Choice>>,
    error: Option<ApiError>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    content: String,
}

#[derive(Deserialize)]
struct ApiError {
    message: String,
}

fn categorize_network_error(err: reqwest::Error) -> String {
    let msg = err.to_string().to_lowercase();
    if msg.contains("dns") || msg.contains("resolve") || msg.contains("unreachable") {
        "unreachable — DNS failure or connection refused".to_string()
    } else if err.is_timeout() || msg.contains("timeout") || msg.contains("deadline") {
        "timeout — server did not respond within 10s".to_string()
    } else if msg.contains("connection refused") {
        "connection refused".to_string()
    } else {
        format!("network error: {}", err)
    }
}

fn categorize_api_error(err: &ApiError) -> String {
    let msg = err.message.to_lowercase();
    if msg.contains("model") && (msg.contains("not found") || msg.contains("does not exist")) {
        format!("model not found: {}", err.message)
    } else if msg.contains("auth") || msg.contains("key") || msg.contains("credential") {
        format!("auth error: {}", err.message)
    } else {
        format!("API error: {}", err.message)
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(n).collect();
        format!("{}…", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path, header};

    #[tokio::test]
    async fn test_success() {
        let mock_server = MockServer::start().await;
        let response_body = serde_json::json!({
            "choices": [{
                "message": {
                    "content": "Hi there!"
                }
            }]
        });

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("Authorization", "Bearer sk-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&mock_server)
            .await;

        let tester = Tester::new();
        let result = tester.test(&mock_server.uri(), "sk-test", "gpt-4").await;

        assert!(result.ok, "Expected ok to be true, got error: {}", result.error);
        assert_eq!(result.response, "Hi there!");
        assert!(result.latency_ms >= 0);
    }

    #[tokio::test]
    async fn test_truncates_long_response() {
        let mock_server = MockServer::start().await;
        let long_content = "x".repeat(100);
        let response_body = serde_json::json!({
            "choices": [{
                "message": {
                    "content": long_content
                }
            }]
        });

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&mock_server)
            .await;

        let tester = Tester::new();
        let result = tester.test(&mock_server.uri(), "sk-test", "gpt-4").await;

        assert!(result.ok);
        assert_eq!(result.response.chars().count(), 51); // 50 chars + 1 ellipsis char
        assert!(result.response.ends_with('…'));
    }

    #[tokio::test]
    async fn test_auth_error_401() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let tester = Tester::new();
        let result = tester.test(&mock_server.uri(), "sk-bad", "gpt-4").await;

        assert!(!result.ok);
        assert!(result.error.contains("auth") || result.error.contains("key"));
    }

    #[tokio::test]
    async fn test_auth_error_403() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&mock_server)
            .await;

        let tester = Tester::new();
        let result = tester.test(&mock_server.uri(), "sk-bad", "gpt-4").await;

        assert!(!result.ok);
        assert!(result.error.contains("auth") || result.error.contains("key"));
    }

    #[tokio::test]
    async fn test_not_found_404() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let tester = Tester::new();
        let result = tester.test(&mock_server.uri(), "sk-test", "gpt-4").await;

        assert!(!result.ok);
        assert!(result.error.contains("not found"));
    }

    #[tokio::test]
    async fn test_model_not_found() {
        let mock_server = MockServer::start().await;
        let response_body = serde_json::json!({
            "error": {
                "message": "The model 'bad-model' does not exist"
            }
        });

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&mock_server)
            .await;

        let tester = Tester::new();
        let result = tester.test(&mock_server.uri(), "sk-test", "bad-model").await;

        assert!(!result.ok);
        assert!(result.error.to_lowercase().contains("model"));
        assert!(result.error.to_lowercase().contains("not found") || result.error.to_lowercase().contains("does not exist"));
    }

    #[tokio::test]
    async fn test_network_error() {
        let tester = Tester::new();
        // Use a non-existent domain to trigger a DNS lookup failure immediately
        let result = tester.test("http://cxc-nonexistent-domain-123.com", "sk-test", "gpt-4").await;

        assert!(!result.ok);
        assert!(
            result.error.contains("unreachable")
                || result.error.contains("DNS")
                || result.error.contains("network")
                || result.error.contains("timeout")
                || result.error.contains("connection refused")
        );
    }

    // ── fetch_models tests ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_fetch_models_success() {
        let mock_server = MockServer::start().await;
        let body = serde_json::json!({
            "data": [
                { "id": "gpt-4o" },
                { "id": "gpt-4" }
            ]
        });
        Mock::given(method("GET"))
            .and(path("/models"))
            .and(header("Authorization", "Bearer sk-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&mock_server)
            .await;

        let result = fetch_models(&mock_server.uri(), "sk-test").await;
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        let models = result.unwrap();
        assert_eq!(models, vec!["gpt-4o", "gpt-4"]);
    }

    #[tokio::test]
    async fn test_fetch_models_empty_list() {
        let mock_server = MockServer::start().await;
        let body = serde_json::json!({ "data": [] });
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&mock_server)
            .await;

        let result = fetch_models(&mock_server.uri(), "sk-test").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_fetch_models_auth_error() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let result = fetch_models(&mock_server.uri(), "sk-bad").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("401"));
    }

    #[tokio::test]
    async fn test_fetch_models_network_error() {
        // Non-existent domain — DNS failure
        let result = fetch_models("http://cxc-nonexistent-domain-456.com", "sk-test").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("network error"));
    }
}

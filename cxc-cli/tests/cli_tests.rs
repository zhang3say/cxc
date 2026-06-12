use cxc::cli::{self, Cli};
use cxc::config;
use cxc::target::{TargetAdapter, codex::CodexAdapter};
use tempfile::TempDir;
use clap::Parser;
use std::fs;

fn setup_env() -> (TempDir, TempDir) {
    let config_dir = tempfile::tempdir().unwrap();
    let codex_dir = tempfile::tempdir().unwrap();
    
    // Write minimal config.toml and auth.json to the codex dir
    fs::write(
        codex_dir.path().join("config.toml"),
        b"approval_policy = \"never\"\nmodel = \"gpt-4\"\n[model_providers.codex]\nbase_url = \"https://old.example.com/v1\"\n"
    ).unwrap();
    fs::write(
        codex_dir.path().join("auth.json"),
        b"{\n  \"auth_mode\": \"apikey\",\n  \"OPENAI_API_KEY\": \"sk-old\"\n}\n"
    ).unwrap();

    unsafe {
        std::env::set_var("CXC_TEST_CONFIG_DIR", config_dir.path());
        std::env::set_var("CXC_TEST_CODEX_DIR", codex_dir.path());
    }

    (config_dir, codex_dir)
}

#[tokio::test]
async fn test_cli_flow() {
    let (_c_dir, codex_dir) = setup_env();

    // Start a mock server for testing connectivity
    let mock_server = wiremock::MockServer::start().await;
    let response_body = serde_json::json!({
        "choices": [{
            "message": {
                "content": "Hi from mock!"
            }
        }]
    });
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/chat/completions"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&mock_server)
        .await;

    let mock_uri = mock_server.uri();

    // 1. Add provider "p1" via CLI
    let args_add1 = vec![
        "cxc", "provider", "add",
        "--name", "p1",
        "--base-url", &mock_uri,
        "--api-key", "sk-p1",
        "--model", "gpt-4-turbo",
        "--remark", "Primary provider"
    ];
    let cli_add1 = Cli::parse_from(args_add1);
    cli::run_cli(cli_add1).await.unwrap();

    // Verify config saved
    let cfg = config::load().unwrap();
    assert_eq!(cfg.providers.len(), 1);
    assert_eq!(cfg.providers[0].name, "p1");
    assert_eq!(cfg.active, "p1");

    // 2. Add provider "p2" via CLI
    let args_add2 = vec![
        "cxc", "provider", "add",
        "--name", "p2",
        "--base-url", &mock_uri,
        "--api-key", "sk-p2",
        "--model", "gpt-3.5-turbo"
    ];
    let cli_add2 = Cli::parse_from(args_add2);
    cli::run_cli(cli_add2).await.unwrap();

    let cfg2 = config::load().unwrap();
    assert_eq!(cfg2.providers.len(), 2);
    assert_eq!(cfg2.active, "p1"); // Active should still be the first one

    // 3. List providers via CLI
    let list_args = vec!["cxc", "provider", "list"];
    let list_cli = Cli::parse_from(list_args);
    cli::run_cli(list_cli).await.unwrap();

    // 4. Test p1 via CLI
    let test_args = vec!["cxc", "provider", "test", "p1"];
    let test_cli = Cli::parse_from(test_args);
    cli::run_cli(test_cli).await.unwrap();

    // Verify p1 last_ok is updated in config
    let loaded = config::load().unwrap();
    let p1 = config::get_provider(&loaded, "p1").unwrap();
    assert_eq!(p1.last_ok, Some(true));
    assert!(p1.latency_ms.is_some());

    // 5. Test all via CLI
    let test_all_args = vec!["cxc", "provider", "test", "--all"];
    let test_all_cli = Cli::parse_from(test_all_args);
    cli::run_cli(test_all_cli).await.unwrap();

    // 6. Switch to p2 via CLI
    let switch_args = vec!["cxc", "provider", "switch", "p2"];
    let switch_cli = Cli::parse_from(switch_args);
    cli::run_cli(switch_cli).await.unwrap();

    // Verify active pointer changed in cxc config
    let loaded = config::load().unwrap();
    assert_eq!(loaded.active, "p2");

    // Verify Codex config files mutated
    let adapter = CodexAdapter::new_with_dir(codex_dir.path());
    let codex_cfg = adapter.read().unwrap();
    assert_eq!(codex_cfg.base_url, mock_server.uri());
    assert_eq!(codex_cfg.api_key, "sk-p2");
    assert_eq!(codex_cfg.model, "gpt-3.5-turbo");
}

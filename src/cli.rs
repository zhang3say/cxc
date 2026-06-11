use clap::{Parser, Subcommand};
use crate::config::{self, Provider};
use crate::target::{TargetAdapter, TargetConfig, codex::CodexAdapter};
use std::io::{self, Write};
use anyhow::{Context, Result};
use url::Url;
use unicode_width::UnicodeWidthStr;

#[derive(Parser, Debug)]
#[command(name = "cxc", version, about = "Codex Cross-Connect")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Manage API relay providers")]
    Provider {
        #[command(subcommand)]
        subcommand: ProviderCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum ProviderCommands {
    #[command(about = "Add a new provider")]
    Add {
        #[arg(long, help = "Provider name (unique identifier)")]
        name: Option<String>,
        #[arg(long, help = "Base URL of the API relay endpoint")]
        base_url: Option<String>,
        #[arg(long, help = "API key for authentication")]
        api_key: Option<String>,
        #[arg(long, help = "Model name (e.g. gpt-4)")]
        model: Option<String>,
        #[arg(long, default_value = "responses", help = "Wire API protocol (default: responses)")]
        wire_api: String,
        #[arg(long, help = "Remark/note describing this provider")]
        remark: Option<String>,
    },
    #[command(about = "List all saved providers")]
    List,
    #[command(about = "Switch the active provider")]
    Switch {
        #[arg(help = "Provider name to switch to")]
        name: String,
    },
    #[command(about = "Test a provider's connectivity")]
    Test {
        #[arg(help = "Provider name to test (defaults to active)")]
        name: Option<String>,
        #[arg(short, long, help = "Test all saved providers concurrently")]
        all: bool,
    },
    #[command(about = "Edit an existing provider")]
    Edit {
        #[arg(help = "Provider name to edit")]
        old_name: String,
        #[arg(long, help = "New name for the provider")]
        name: Option<String>,
        #[arg(long, help = "New base URL of the API relay endpoint")]
        base_url: Option<String>,
        #[arg(long, help = "New API key for authentication")]
        api_key: Option<String>,
        #[arg(long, help = "New model name")]
        model: Option<String>,
        #[arg(long, help = "New wire API protocol")]
        wire_api: Option<String>,
        #[arg(long, help = "New remark/note for the provider")]
        remark: Option<String>,
    },
    #[command(about = "Remove a saved provider")]
    Remove {
        #[arg(help = "Provider name to remove")]
        name: String,
    },
}

pub async fn run_cli(cli: Cli) -> Result<()> {
    match cli.command {
        Some(Commands::Provider { subcommand }) => match subcommand {
            ProviderCommands::Add {
                name,
                base_url,
                api_key,
                model,
                wire_api,
                remark,
            } => {
                let mut cfg = config::load().context("Failed to load configuration")?;

                let is_interactive = name.is_none() || base_url.is_none() || api_key.is_none() || model.is_none();

                let name_val = prompt_if_empty(name, "Provider name");
                let base_url_val = prompt_if_empty(base_url, "Base URL (e.g. https://api.example.com/v1)");
                let api_key_val = prompt_if_empty(api_key, "API key");
                let model_val = prompt_if_empty(model, "Model (e.g. gpt-4)");

                let remark_val = if is_interactive {
                    let r = prompt_if_empty(remark, "Remark (optional)");
                    if r.is_empty() { None } else { Some(r) }
                } else {
                    remark
                };

                // Validate
                if name_val.is_empty() {
                    return Err(anyhow::anyhow!("name is required"));
                }
                validate_url(&base_url_val)?;
                if api_key_val.is_empty() {
                    return Err(anyhow::anyhow!("api_key is required"));
                }
                if model_val.is_empty() {
                    return Err(anyhow::anyhow!("model is required"));
                }

                let p = Provider {
                    name: name_val.clone(),
                    base_url: base_url_val,
                    api_key: api_key_val,
                    model: model_val,
                    wire_api: if wire_api.is_empty() { "responses".to_string() } else { wire_api },
                    remark: remark_val,
                    last_test: None,
                    latency_ms: None,
                    last_ok: None,
                };

                config::add_provider(&mut cfg, p)?;
                println!("✓ Added provider \"{}\"", name_val);
                if cfg.active == name_val {
                    println!("  Set as active provider.");
                }
            }
            ProviderCommands::List => {
                let cfg = config::load().context("Failed to load configuration")?;
                print_providers_table(&cfg);
            }
            ProviderCommands::Switch { name } => {
                let mut cfg = config::load().context("Failed to load configuration")?;

                if cfg.active == name {
                    println!("Provider \"{}\" is already active.", name);
                    return Ok(());
                }

                let p = config::get_provider(&cfg, &name)
                    .ok_or_else(|| {
                        let names: Vec<String> = cfg.providers.iter().map(|prov| prov.name.clone()).collect();
                        anyhow::anyhow!("provider \"{}\" not found\n\nAvailable providers: {}", name, names.join(", "))
                    })?
                    .clone();

                let adapter = CodexAdapter::new().context("Failed to initialize target adapter")?;
                let old_cfg = adapter.read().ok();

                println!("Switching Codex to provider \"{}\"…", name);

                let tc = TargetConfig {
                    base_url: p.base_url.clone(),
                    api_key: p.api_key.clone(),
                    model: p.model.clone(),
                    wire_api: if p.wire_api.is_empty() { "responses".to_string() } else { p.wire_api.clone() },
                };

                adapter.write(&tc).context("Failed to update Codex configuration")?;

                config::set_active(&mut cfg, &name)?;

                println!("✓ Switched to \"{}\"", name);
                if let Some(old) = old_cfg {
                    println!("  base_url: {} → {}", old.base_url, p.base_url);
                    println!("  model:    {} → {}", old.model, p.model);
                }
                println!("  (Codex config.toml and auth.json updated; .bak backups created)");
            }
            ProviderCommands::Test { name, all } => {
                let mut cfg = config::load().context("Failed to load configuration")?;

                if all {
                    if name.is_some() {
                        return Err(anyhow::anyhow!("cannot specify a provider name when using --all"));
                    }
                    if cfg.providers.is_empty() {
                        println!("No providers saved. Run `cxc provider add` to add one.");
                        return Ok(());
                    }

                    println!("Testing all {} providers concurrently…", cfg.providers.len());

                    let tester = std::sync::Arc::new(crate::connectivity::Tester::new());
                    let mut handles = Vec::new();

                    for p in &cfg.providers {
                        let tester = tester.clone();
                        let p = p.clone();
                        let handle = tokio::spawn(async move {
                            let res = tester.test(&p.base_url, &p.api_key, &p.model).await;
                            (p.name, res)
                        });
                        handles.push(handle);
                    }

                    let mut results = Vec::new();
                    for handle in handles {
                        let (p_name, res) = handle.await?;
                        results.push((p_name, res));
                    }

                    let mut any_failed = false;
                    for (i, p) in cfg.providers.clone().iter().enumerate() {
                        if i > 0 {
                            println!();
                        }
                        let res = results.iter().find(|(n, _)| n == &p.name).map(|(_, r)| r).unwrap();
                        println!("Testing provider \"{}\" ({}, model: {})…", p.name, p.base_url, p.model);
                        if res.ok {
                            println!("✓ Connected in {}ms", res.latency_ms);
                            if !res.response.is_empty() {
                                println!("  Model response: {:?}", res.response);
                            }
                        } else {
                            println!("✗ Failed: {}", res.error);
                            any_failed = true;
                        }

                        // Persist result
                        let _ = config::update_test_result(&mut cfg, &p.name, res.latency_ms, res.ok);
                    }

                    if any_failed {
                        return Err(anyhow::anyhow!("one or more connectivity tests failed"));
                    }
                } else {
                    let p = match name {
                        Some(ref n) => config::get_provider(&cfg, n)
                            .ok_or_else(|| anyhow::anyhow!("provider \"{}\" not found\n\nRun `cxc provider list` to see available providers", n))?,
                        None => config::get_active(&cfg)
                            .ok_or_else(|| anyhow::anyhow!("no active provider — run `cxc provider add` first"))?,
                    }.clone();

                    println!("Testing provider \"{}\" ({}, model: {})…", p.name, p.base_url, p.model);

                    let tester = crate::connectivity::Tester::new();
                    let res = tester.test(&p.base_url, &p.api_key, &p.model).await;

                    if res.ok {
                        println!("✓ Connected in {}ms", res.latency_ms);
                        if !res.response.is_empty() {
                            println!("  Model response: {:?}", res.response);
                        }
                    } else {
                        println!("✗ Failed: {}", res.error);
                    }

                    // Persist result
                    config::update_test_result(&mut cfg, &p.name, res.latency_ms, res.ok)?;

                    if !res.ok {
                        return Err(anyhow::anyhow!("connectivity test failed"));
                    }
                }
            }
            ProviderCommands::Edit {
                old_name,
                name,
                base_url,
                api_key,
                model,
                wire_api,
                remark,
            } => {
                let mut cfg = config::load().context("Failed to load configuration")?;

                let existing = config::get_provider(&cfg, &old_name)
                    .ok_or_else(|| anyhow::anyhow!("provider \"{}\" not found", old_name))?
                    .clone();

                let any_flags_set = name.is_some()
                    || base_url.is_some()
                    || api_key.is_some()
                    || model.is_some()
                    || wire_api.is_some()
                    || remark.is_some();

                let (name_val, base_url_val, api_key_val, model_val, wire_api_val, remark_val) = if any_flags_set {
                    (
                        name.unwrap_or(existing.name),
                        base_url.unwrap_or(existing.base_url),
                        api_key.unwrap_or(existing.api_key),
                        model.unwrap_or(existing.model),
                        wire_api.unwrap_or(existing.wire_api),
                        remark.or(existing.remark),
                    )
                } else {
                    (
                        prompt_with_default("Provider name", &existing.name),
                        prompt_with_default("Base URL", &existing.base_url),
                        prompt_with_default("API Key", &existing.api_key),
                        prompt_with_default("Model", &existing.model),
                        prompt_with_default("Wire API", &existing.wire_api),
                        {
                            let r_def = existing.remark.as_deref().unwrap_or("");
                            let res = prompt_with_default("Remark (optional)", r_def);
                            if res.is_empty() { None } else { Some(res) }
                        }
                    )
                };

                // Validate
                if name_val.is_empty() {
                    return Err(anyhow::anyhow!("name cannot be empty"));
                }
                validate_url(&base_url_val)?;
                if api_key_val.is_empty() {
                    return Err(anyhow::anyhow!("api_key cannot be empty"));
                }
                if model_val.is_empty() {
                    return Err(anyhow::anyhow!("model cannot be empty"));
                }

                let updated = Provider {
                    name: name_val.clone(),
                    base_url: base_url_val,
                    api_key: api_key_val,
                    model: model_val,
                    wire_api: if wire_api_val.is_empty() { "responses".to_string() } else { wire_api_val },
                    remark: remark_val,
                    last_test: None,
                    latency_ms: None,
                    last_ok: None,
                };

                config::edit_provider(&mut cfg, &old_name, updated.clone())?;
                println!("✓ Updated provider \"{}\"", old_name);

                if cfg.active == name_val {
                    println!("Updating active provider configuration in Codex...");
                    let adapter = CodexAdapter::new().context("Failed to initialize target adapter")?;
                    let tc = TargetConfig {
                        base_url: updated.base_url.clone(),
                        api_key: updated.api_key.clone(),
                        model: updated.model.clone(),
                        wire_api: if updated.wire_api.is_empty() { "responses".to_string() } else { updated.wire_api.clone() },
                    };
                    adapter.write(&tc).context("Failed to update Codex active configuration")?;
                    println!("✓ Codex config updated.");
                }
            }
            ProviderCommands::Remove { name } => {
                let mut cfg = config::load().context("Failed to load configuration")?;
                config::remove_provider(&mut cfg, &name)?;
                println!("✓ Removed provider \"{}\"", name);
            }
        },
        None => {
            crate::tui::run().await?;
        }
    }
    Ok(())
}

fn prompt_if_empty(val: Option<String>, prompt: &str) -> String {
    if let Some(v) = val {
        if !v.trim().is_empty() {
            return v.trim().to_string();
        }
    }
    print!("{}: ", prompt);
    let _ = io::stdout().flush();
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
    input.trim().to_string()
}

fn prompt_with_default(label: &str, def_val: &str) -> String {
    print!("{} [{}]: ", label, def_val);
    let _ = io::stdout().flush();
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
    let line = input.trim();
    if line.is_empty() {
        def_val.to_string()
    } else {
        line.to_string()
    }
}

fn validate_url(url_str: &str) -> Result<()> {
    if !url_str.starts_with("http://") && !url_str.starts_with("https://") {
        return Err(anyhow::anyhow!("invalid base_url \"{}\": must be a valid http/https URL", url_str));
    }
    Url::parse(url_str).map_err(|e| anyhow::anyhow!("invalid base_url \"{}\": {}", url_str, e))?;
    Ok(())
}

fn pad_right(s: &str, width: usize) -> String {
    let w = UnicodeWidthStr::width(s);
    if w >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - w))
    }
}

fn format_latency(latency: Option<i64>, last_ok: Option<bool>) -> String {
    match latency {
        Some(lat) => {
            let lat_str = format!("{}ms", lat);
            match last_ok {
                Some(true) => format!("✓ {}", lat_str),
                Some(false) => format!("✗ {}", lat_str),
                None => lat_str,
            }
        }
        None => "-".to_string(),
    }
}

fn print_providers_table(cfg: &config::Config) {
    if cfg.providers.is_empty() {
        println!("No providers saved. Run `cxc provider add` to add one.");
        return;
    }

    let mut col_name = 4;
    let mut col_url = 8;
    let mut col_model = 5;
    let mut col_last_test = 9;
    let mut col_latency = 7;
    let mut col_remark = 6;

    for p in &cfg.providers {
        col_name = col_name.max(UnicodeWidthStr::width(p.name.as_str()));
        col_url = col_url.max(UnicodeWidthStr::width(p.base_url.as_str()));
        col_model = col_model.max(UnicodeWidthStr::width(p.model.as_str()));
        let last_test_str = p.last_test.map(|t| t.to_rfc3339()).unwrap_or_else(|| "-".to_string());
        col_last_test = col_last_test.max(UnicodeWidthStr::width(last_test_str.as_str()));
        let latency_str = format_latency(p.latency_ms, p.last_ok);
        col_latency = col_latency.max(UnicodeWidthStr::width(latency_str.as_str()));
        col_remark = col_remark.max(p.remark.as_ref().map(|r| UnicodeWidthStr::width(r.as_str())).unwrap_or(0));
    }

    // Print headers
    let h_name = pad_right("NAME", col_name);
    let h_url = pad_right("BASE URL", col_url);
    let h_model = pad_right("MODEL", col_model);
    let h_test = pad_right("LAST TEST", col_last_test);
    let h_lat = pad_right("LATENCY", col_latency);
    let h_rem = pad_right("REMARK", col_remark);

    println!("  {}  {}  {}  {}  {}  {}", h_name, h_url, h_model, h_test, h_lat, h_rem);
    println!(
        "  {}  {}  {}  {}  {}  {}",
        "-".repeat(col_name),
        "-".repeat(col_url),
        "-".repeat(col_model),
        "-".repeat(col_last_test),
        "-".repeat(col_latency),
        "-".repeat(col_remark)
    );

    for p in &cfg.providers {
        let active = if p.name == cfg.active { "★ " } else { "  " };
        let last_test_str = p.last_test.map(|t| t.to_rfc3339()).unwrap_or_else(|| "-".to_string());
        let latency_str = format_latency(p.latency_ms, p.last_ok);
        let remark_str = p.remark.as_deref().unwrap_or("");

        let r_name = pad_right(&p.name, col_name);
        let r_url = pad_right(&p.base_url, col_url);
        let r_model = pad_right(&p.model, col_model);
        let r_test = pad_right(&last_test_str, col_last_test);
        let r_lat = pad_right(&latency_str, col_latency);
        let r_rem = pad_right(remark_str, col_remark);

        println!("{}{}  {}  {}  {}  {}  {}", active, r_name, r_url, r_model, r_test, r_lat, r_rem);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_url() {
        assert!(validate_url("https://api.openai.com/v1").is_ok());
        assert!(validate_url("http://localhost:8080/v1").is_ok());
        assert!(validate_url("api.openai.com/v1").is_err());
        assert!(validate_url("ftp://api.openai.com/v1").is_err());
        assert!(validate_url("https://invalid url.com").is_err());
    }

    #[test]
    fn test_pad_right() {
        assert_eq!(pad_right("abc", 5), "abc  ");
        assert_eq!(pad_right("测试", 6), "测试  ");
        assert_eq!(pad_right("abcdef", 4), "abcdef");
    }
}

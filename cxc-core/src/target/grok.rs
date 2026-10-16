use crate::config::{Config, GrokWebSearch, Provider};
use crate::target::{TargetAdapter, TargetConfig, TargetError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
use std::sync::OnceLock;
use toml_edit::{DocumentMut, Item, Table, Value};

const CONFIG_DIR_NAME: &str = ".grok";
const CONFIG_FILE_NAME: &str = "config.toml";
const MODELS_SECTION: &str = "models";
const MODEL_SECTION: &str = "model";
const MODEL_PROVIDERS_SECTION: &str = "model_providers";
/// 端点级字段：统一写在中转站块里，模型块内与之完全相同的副本会被清理
const ENDPOINT_FIELDS: [&str; 3] = ["base_url", "api_key", "api_backend"];

/// 一次性迁移（把 model 维度的配置整理成中转站维度）的结果
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct GrokReorgReport {
    /// 归入某个中转站的模型块数量
    pub models_grouped: usize,
    /// 新建的中转站块键
    pub created_providers: Vec<String>,
    /// 复用的已有中转站块键（含与 CXC 中转站同名的匹配）
    pub reused_providers: Vec<String>,
    /// 被改名的中转站块（旧键 → 新键）
    pub renamed_providers: Vec<(String, String)>,
    /// 无法归类的模型块（既没有端点字段，也没有可解析的 provider 引用）
    pub skipped_models: Vec<String>,
    /// 迁移后 `[models].web_search` 的取值
    pub web_search: Option<String>,
    /// 每个中转站块下引用的模型（含默认模型之外的附加模型）
    pub groups: Vec<(String, Vec<String>)>,
}

pub struct GrokAdapter {
    grok_dir: PathBuf,
}

impl GrokAdapter {
    pub fn new() -> Result<Self, TargetError> {
        let grok_dir = if let Ok(test_dir) = std::env::var("CXC_TEST_GROK_DIR") {
            PathBuf::from(test_dir)
        } else {
            let cfg = crate::config::load().ok();
            Self::resolve_grok_dir(cfg.as_ref())?
        };
        Ok(Self { grok_dir })
    }

    pub fn new_from_config(cfg: &crate::config::Config) -> Result<Self, TargetError> {
        let grok_dir = if let Ok(test_dir) = std::env::var("CXC_TEST_GROK_DIR") {
            PathBuf::from(test_dir)
        } else {
            Self::resolve_grok_dir(Some(cfg))?
        };
        Ok(Self { grok_dir })
    }

    fn resolve_grok_dir(cfg: Option<&crate::config::Config>) -> Result<PathBuf, TargetError> {
        let source = cfg
            .map(crate::config::effective_source)
            .unwrap_or(crate::config::SOURCE_WSL);

        let custom_dir = cfg.and_then(|c| {
            if !c.grok_custom_dir.is_empty() {
                let mut path_str = c.grok_custom_dir.clone();

                #[cfg(target_os = "linux")]
                {
                    if path_str.len() >= 2 && path_str.as_bytes()[1] == b':' {
                        let drive = path_str.chars().next().unwrap().to_ascii_lowercase();
                        let remaining = &path_str[2..];
                        let normalized = remaining.replace('\\', "/");
                        path_str = format!("/mnt/{}{}", drive, normalized);
                    } else {
                        path_str = path_str.replace('\\', "/");
                    }
                }

                return Some(PathBuf::from(path_str));
            }
            None
        });

        if let Some(dir) = custom_dir {
            Ok(dir)
        } else {
            Self::default_grok_dir(source)
        }
    }

    /// Resolve the default Grok config directory based on global source.
    ///
    /// - `"wsl"` on Linux  → `~/.grok`
    /// - `"app"` on Linux  → `/mnt/c/Users/<username>/.grok`
    /// - `"app"` on Windows → `%USERPROFILE%\.grok`
    /// - `"wsl"` on Windows → `\\wsl.localhost\<distro>\home\<user>\.grok`
    fn default_grok_dir(source: &str) -> Result<PathBuf, TargetError> {
        #[cfg(target_os = "linux")]
        {
            if source == "app" {
                if let Some(win_dir) = Self::detect_windows_grok_dir() {
                    return Ok(win_dir);
                }
                return Err(TargetError::ParseError(
                    "Unable to locate Windows home directory automatically from within WSL. Please specify a custom directory.".to_string(),
                ));
            }
            let home = dirs::home_dir().ok_or(TargetError::NoHomeDir)?;
            return Ok(home.join(CONFIG_DIR_NAME));
        }

        #[cfg(target_os = "windows")]
        {
            if source == "wsl" {
                if let Some(wsl_dir) = Self::cached_wsl_grok_dir() {
                    return Ok(wsl_dir);
                }
                return Err(TargetError::ParseError(
                    "Unable to locate WSL home directory automatically. Please specify a custom directory.".to_string(),
                ));
            }
            let home = dirs::home_dir().ok_or(TargetError::NoHomeDir)?;
            return Ok(home.join(CONFIG_DIR_NAME));
        }

        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            let _ = source;
            let home = dirs::home_dir().ok_or(TargetError::NoHomeDir)?;
            Ok(home.join(CONFIG_DIR_NAME))
        }
    }

    #[cfg(target_os = "linux")]
    fn detect_windows_grok_dir() -> Option<PathBuf> {
        let username = std::env::var("USER").ok()?;
        let candidate = PathBuf::from(format!("/mnt/c/Users/{}/{}", username, CONFIG_DIR_NAME));
        if candidate.exists() {
            return Some(candidate);
        }
        let home = PathBuf::from(format!("/mnt/c/Users/{}", username));
        if home.exists() {
            return Some(candidate);
        }
        if let Ok(entries) = fs::read_dir("/mnt/c/Users") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if name == username.to_lowercase() {
                    return Some(entry.path().join(CONFIG_DIR_NAME));
                }
            }
        }
        None
    }

    #[cfg(target_os = "windows")]
    fn cached_wsl_grok_dir() -> Option<PathBuf> {
        static WSL_GROK_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
        WSL_GROK_DIR.get_or_init(Self::detect_wsl_grok_dir).clone()
    }

    #[cfg(target_os = "windows")]
    fn detect_wsl_grok_dir() -> Option<PathBuf> {
        let mut first_valid_home: Option<PathBuf> = None;

        if let Ok(output) = std::process::Command::new("wsl")
            .args(["-e", "sh", "-lc", "wslpath -w \"$HOME\""])
            .output()
        {
            if output.status.success() {
                if let Some(candidate) =
                    Self::wsl_home_output_to_config_dir(&output.stdout, CONFIG_DIR_NAME)
                {
                    if candidate.exists() {
                        return Some(candidate);
                    }
                    first_valid_home = Some(candidate);
                }
            }
        }

        let mut scan_unc_path = |base_path: &str| -> Option<PathBuf> {
            let base = Path::new(base_path);
            if !base.exists() {
                return None;
            }

            if let Ok(distro_entries) = fs::read_dir(base) {
                for distro_entry in distro_entries.flatten() {
                    let home_path = distro_entry.path().join("home");
                    if !home_path.exists() {
                        continue;
                    }

                    if let Ok(user_entries) = fs::read_dir(&home_path) {
                        for user_entry in user_entries.flatten() {
                            let user_path = user_entry.path();
                            let grok_dir = user_path.join(CONFIG_DIR_NAME);
                            if grok_dir.exists() {
                                return Some(grok_dir);
                            }
                            if first_valid_home.is_none() && user_path.exists() {
                                first_valid_home = Some(grok_dir);
                            }
                        }
                    }
                }
            }
            None
        };

        if let Some(path) = scan_unc_path("\\\\wsl.localhost") {
            return Some(path);
        }
        if let Some(path) = scan_unc_path("\\\\wsl$") {
            return Some(path);
        }

        if let Ok(username) = std::env::var("USERNAME") {
            let username = username.to_lowercase();
            let distros = [
                "Ubuntu",
                "Debian",
                "openSUSE-Leap",
                "kali-linux",
                "Ubuntu-22.04",
                "Ubuntu-24.04",
            ];
            for distro in &distros {
                let candidate = PathBuf::from(format!(
                    "\\\\wsl.localhost\\{}\\home\\{}\\{}",
                    distro, username, CONFIG_DIR_NAME
                ));
                if candidate.exists() {
                    return Some(candidate);
                }
                let home =
                    PathBuf::from(format!("\\\\wsl.localhost\\{}\\home\\{}", distro, username));
                if first_valid_home.is_none() && home.exists() {
                    first_valid_home = Some(candidate);
                }

                let candidate = PathBuf::from(format!(
                    "\\\\wsl$\\{}\\home\\{}\\{}",
                    distro, username, CONFIG_DIR_NAME
                ));
                if candidate.exists() {
                    return Some(candidate);
                }
                let home = PathBuf::from(format!("\\\\wsl$\\{}\\home\\{}", distro, username));
                if first_valid_home.is_none() && home.exists() {
                    first_valid_home = Some(candidate);
                }
            }
        }

        first_valid_home
    }

    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    fn wsl_home_output_to_config_dir(output: &[u8], config_dir_name: &str) -> Option<PathBuf> {
        let path = String::from_utf8_lossy(output).trim().to_string();
        if path.is_empty() || path == "~" || path.contains("\\~") || path.contains("/~") {
            return None;
        }
        Some(PathBuf::from(path).join(config_dir_name))
    }

    pub fn new_with_dir<P: AsRef<Path>>(dir: P) -> Self {
        Self {
            grok_dir: dir.as_ref().to_path_buf(),
        }
    }

    pub fn config_path(&self) -> PathBuf {
        self.grok_dir.join(CONFIG_FILE_NAME)
    }

    /// Map Provider.wire_api to Grok's api_backend values.
    pub fn map_wire_api_to_api_backend(wire_api: &str) -> String {
        match wire_api.trim() {
            "" | "chat" | "chat_completions" => "chat_completions".to_string(),
            "responses" => "responses".to_string(),
            "messages" => "messages".to_string(),
            other => other.to_string(),
        }
    }

    /// Map Grok api_backend back to Provider.wire_api for read().
    fn map_api_backend_to_wire_api(api_backend: &str) -> String {
        match api_backend.trim() {
            "" | "chat_completions" => "chat_completions".to_string(),
            other => other.to_string(),
        }
    }

    /// 该中转站要写入的默认模型。web_search 指定的模型由调用方追加。
    pub fn managed_models(provider: &Provider) -> Vec<String> {
        let mut models: Vec<String> = Vec::new();
        push_unique(&mut models, &provider.model);
        models
    }

    /// 以中转站为主体写入：端点与协议写进 `[model_providers."<名称>"]`，模型块只保留
    /// 模型自身字段并引用该中转站。
    ///
    /// 受 CXC 管理的模型块里，`base_url` / `api_key` / `api_backend` 会被清掉——模型自身字段
    /// 优先级高于 provider 块，留着旧值会让本次切换不生效；其余字段原样保留。
    pub fn write_provider(&self, provider: &Provider) -> Result<(), TargetError> {
        let key = provider_key(&provider.name)?;
        let mut models = Self::managed_models(provider);
        if models.is_empty() {
            return Err(TargetError::ParseError(
                "Grok 模型不能为空，无法写入配置".to_string(),
            ));
        }
        if let Some(GrokWebSearch::Model { model }) = &provider.grok_web_search {
            if model.trim().is_empty() {
                return Err(TargetError::ParseError(
                    "web_search 指定的模型不能为空".to_string(),
                ));
            }
            // web_search 指到的模型也要有可解析的引用
            push_unique(&mut models, model);
        }

        if !self.grok_dir.exists() {
            fs::create_dir_all(&self.grok_dir).map_err(|err| TargetError::WriteError {
                path: self.grok_dir.clone(),
                source: err,
            })?;
        }

        let config_path = self.config_path();
        backup(&config_path)?;

        let toml_data = if config_path.exists() {
            fs::read_to_string(&config_path).map_err(|err| TargetError::ReadError {
                path: config_path.clone(),
                source: err,
            })?
        } else {
            // Minimal Grok config scaffold for first-time write
            String::from("[models]\ndefault = \"\"\n")
        };

        let mut doc = toml_data
            .parse::<DocumentMut>()
            .map_err(|e| TargetError::ParseError(e.to_string()))?;

        let api_backend = Self::map_wire_api_to_api_backend(&provider.wire_api);

        // [models].default 指向该中转站的默认模型
        ensure_table(&mut doc, MODELS_SECTION);
        doc[MODELS_SECTION]["default"] = toml_edit::value(&provider.model);

        // 中转站块承载端点级设置
        write_provider_block(
            &mut doc,
            &key,
            &provider.base_url,
            &provider.api_key,
            &api_backend,
        );

        // 各模型块只保留模型自身字段，并引用该中转站
        for model in &models {
            ensure_table(&mut doc, MODEL_SECTION);
            ensure_standard_table(&mut doc, MODEL_SECTION, model);
            let block = &mut doc[MODEL_SECTION][model.as_str()];
            block["model"] = toml_edit::value(model.as_str());
            block["model_provider"] = toml_edit::value(key.as_str());
            clear_model_endpoint_fields(block);
        }

        apply_web_search(&mut doc, provider)?;

        let out = doc.to_string();
        let _ = out
            .parse::<DocumentMut>()
            .map_err(|e| TargetError::SerializeError(format!("Written TOML is invalid: {}", e)))?;

        write_secure_file(&config_path, out.as_bytes()).map_err(|err| TargetError::WriteError {
            path: config_path.clone(),
            source: err,
        })?;

        Ok(())
    }

    /// 一次性迁移：把所有 `[model.*]` 按端点分组，收拢进 `[model_providers.*]`，
    /// 让模型块只保留 `model_provider` 引用。
    ///
    /// 键的取名规则：端点匹配到某个 CXC Grok 中转站 → 用该中转站名称（必要时把已有同端点
    /// 的块改名过去）；否则复用已存在的同端点块；再否则用 `base_url` 的主机名新建。
    pub fn reorganize_provider_centric(
        &self,
        cfg: &Config,
    ) -> Result<GrokReorgReport, TargetError> {
        let config_path = self.config_path();
        let toml_data = fs::read_to_string(&config_path).map_err(|err| TargetError::ReadError {
            path: config_path.clone(),
            source: err,
        })?;

        let mut doc = toml_data
            .parse::<DocumentMut>()
            .map_err(|e| TargetError::ParseError(e.to_string()))?;

        let existing_providers: BTreeMap<String, Endpoint> = collect_providers(&doc);
        let model_entries = collect_model_endpoints(&doc);

        let mut report = GrokReorgReport::default();
        let mut groups: Vec<Group> = Vec::new();

        for entry in model_entries {
            let Some(endpoint) = entry.endpoint else {
                report.skipped_models.push(entry.model.clone());
                continue;
            };
            match groups.iter_mut().find(|g| g.endpoint == endpoint) {
                Some(group) => group.models.push(entry.model),
                None => groups.push(Group {
                    endpoint,
                    models: vec![entry.model],
                }),
            }
        }

        let mut taken: HashSet<String> = existing_providers.keys().cloned().collect();
        let mut resolved: Vec<(String, &Group, Option<String>)> = Vec::new();

        for group in &groups {
            let existing_match = existing_providers
                .iter()
                .find(|(_, ep)| ep.matches(&group.endpoint))
                .map(|(k, _)| k.clone());
            let cxc_match = cfg
                .grok_providers
                .iter()
                .find(|p| group.endpoint.matches_values(&p.base_url, &p.api_key))
                .map(|p| p.name.clone())
                .filter(|name| !name.trim().is_empty());

            let (key, renamed_from) = match (cxc_match, existing_match) {
                (Some(name), existing) => {
                    let renamed = existing.filter(|old| old != &name);
                    (name, renamed)
                }
                (None, Some(existing)) => (existing, None),
                (None, None) => {
                    let base = host_key(&group.endpoint.base_url);
                    let key = unique_key(&base, &taken);
                    (key, None)
                }
            };
            taken.insert(key.clone());
            resolved.push((key, group, renamed_from));
        }

        for (key, group, renamed_from) in &resolved {
            if let Some(old_key) = renamed_from {
                let moved = doc
                    .get_mut(MODEL_PROVIDERS_SECTION)
                    .and_then(|v| v.as_table_mut())
                    .and_then(|t| t.remove(old_key));
                if let Some(item) = moved {
                    ensure_table(&mut doc, MODEL_PROVIDERS_SECTION);
                    doc[MODEL_PROVIDERS_SECTION]
                        .as_table_mut()
                        .expect("model_providers table")
                        .insert(key, item);
                    report
                        .renamed_providers
                        .push((old_key.clone(), key.clone()));
                }
            }

            let api_backend = if group.endpoint.api_backend.is_empty() {
                "chat_completions".to_string()
            } else {
                group.endpoint.api_backend.clone()
            };
            write_provider_block(
                &mut doc,
                key,
                &group.endpoint.base_url,
                &group.endpoint.api_key,
                &api_backend,
            );
            if existing_providers.contains_key(key) || renamed_from.is_some() {
                report.reused_providers.push(key.clone());
            } else {
                report.created_providers.push(key.clone());
            }

            for model in &group.models {
                ensure_table(&mut doc, MODEL_SECTION);
                ensure_standard_table(&mut doc, MODEL_SECTION, model);
                let block = &mut doc[MODEL_SECTION][model.as_str()];
                block["model_provider"] = toml_edit::value(key.as_str());
                clear_model_endpoint_fields(block);
            }
            report.models_grouped += group.models.len();
            report.groups.push((key.clone(), group.models.clone()));
        }

        report.web_search = doc
            .get(MODELS_SECTION)
            .and_then(|v| v.as_table())
            .and_then(|t| t.get("web_search"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // 去重（同名的 reused/created 只记一次）
        report.reused_providers.sort();
        report.reused_providers.dedup();
        report.created_providers.sort();
        report.created_providers.dedup();

        let out = doc.to_string();
        if out == toml_data {
            // 已经是中转站结构，不做任何改写（也不产生新的备份）
            return Ok(report);
        }
        let _ = out
            .parse::<DocumentMut>()
            .map_err(|e| TargetError::SerializeError(format!("Written TOML is invalid: {}", e)))?;

        backup(&config_path)?;
        write_secure_file(&config_path, out.as_bytes()).map_err(|err| TargetError::WriteError {
            path: config_path.clone(),
            source: err,
        })?;

        Ok(report)
    }
}

impl TargetAdapter for GrokAdapter {
    fn name(&self) -> &'static str {
        "Grok"
    }

    fn read(&self) -> Result<TargetConfig, TargetError> {
        let config_path = self.config_path();
        let toml_data = fs::read_to_string(&config_path).map_err(|err| TargetError::ReadError {
            path: config_path.clone(),
            source: err,
        })?;

        let doc = toml_data
            .parse::<DocumentMut>()
            .map_err(|e| TargetError::ParseError(e.to_string()))?;

        let model = doc
            .get(MODELS_SECTION)
            .and_then(|m| m.as_table())
            .and_then(|t| t.get("default"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let mut base_url = String::new();
        let mut api_key = String::new();
        let mut api_backend: Option<String> = None;

        let model_table = doc
            .get(MODEL_SECTION)
            .and_then(|m| m.as_table())
            .and_then(|t| t.get(model.as_str()))
            .and_then(|v| v.as_table());

        let mut provider_key = None;
        if let Some(table) = model_table {
            base_url = table
                .get("base_url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            api_key = table
                .get("api_key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            api_backend = table
                .get("api_backend")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            provider_key = table
                .get("model_provider")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }

        // 模型块没有自带端点时，跟随 model_provider 引用读取中转站块
        if let Some(pkey) = provider_key {
            if let Some(provider_table) = doc
                .get(MODEL_PROVIDERS_SECTION)
                .and_then(|m| m.as_table())
                .and_then(|t| t.get(pkey.as_str()))
                .and_then(|v| v.as_table())
            {
                if base_url.is_empty() {
                    base_url = provider_table
                        .get("base_url")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                }
                if api_key.is_empty() {
                    api_key = provider_table
                        .get("api_key")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                }
                if api_backend.is_none() {
                    api_backend = provider_table
                        .get("api_backend")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
            }
        }

        let wire_api = Self::map_api_backend_to_wire_api(api_backend.as_deref().unwrap_or(""));

        Ok(TargetConfig {
            base_url,
            api_key,
            model,
            wire_api,
        })
    }

    fn write(&self, config: &TargetConfig) -> Result<(), TargetError> {
        // 兼容路径：没有 Provider 上下文时，用 base_url 的主机名作为中转站块键。
        // web_search 留空，写入时按缺省跟随默认模型。
        let provider = Provider {
            name: host_key(&config.base_url),
            base_url: config.base_url.clone(),
            api_key: config.api_key.clone(),
            model: config.model.clone(),
            wire_api: config.wire_api.clone(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
            grok_web_search: None,
        };
        self.write_provider(&provider)
    }
}

/// 端点三元组，用于把模型块按中转站分组
#[derive(Debug, Clone, PartialEq, Eq)]
struct Endpoint {
    base_url: String,
    api_key: String,
    api_backend: String,
}

impl Endpoint {
    fn normalized_base_url(&self) -> &str {
        self.base_url.trim_end_matches('/')
    }

    fn matches(&self, other: &Endpoint) -> bool {
        self.normalized_base_url() == other.normalized_base_url() && self.api_key == other.api_key
    }

    fn matches_values(&self, base_url: &str, api_key: &str) -> bool {
        self.normalized_base_url() == base_url.trim().trim_end_matches('/') && self.api_key == api_key
    }
}

struct Group {
    endpoint: Endpoint,
    models: Vec<String>,
}

struct ModelEntry {
    model: String,
    endpoint: Option<Endpoint>,
}

/// 中转站块的键：直接用中转站名称（写入时会自动加引号，允许点、空格与中文）
fn provider_key(name: &str) -> Result<String, TargetError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(TargetError::ParseError(
            "Grok 中转站名称不能为空：它同时用作 [model_providers.<名称>] 的键".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

/// `base_url` 的主机名，作为迁移时新建中转站块的兜底键
fn host_key(base_url: &str) -> String {
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        return "relay".to_string();
    }
    match url::Url::parse(trimmed) {
        Ok(url) => url
            .host_str()
            .map(|host| host.to_string())
            .unwrap_or_else(|| trimmed.to_string()),
        Err(_) => trimmed
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or("relay")
            .to_string(),
    }
}

fn unique_key(base: &str, taken: &HashSet<String>) -> String {
    if !taken.contains(base) {
        return base.to_string();
    }
    let mut index = 2usize;
    loop {
        let candidate = format!("{}-{}", base, index);
        if !taken.contains(&candidate) {
            return candidate;
        }
        index += 1;
    }
}

fn push_unique(models: &mut Vec<String>, model: &str) {
    let trimmed = model.trim();
    if trimmed.is_empty() || models.iter().any(|m| m == trimmed) {
        return;
    }
    models.push(trimmed.to_string());
}

/// 确保外层 section 表存在（不输出额外的段头行）
fn ensure_table(doc: &mut DocumentMut, section: &str) {
    if doc.get(section).and_then(|v| v.as_table()).is_none() {
        let mut table = Table::new();
        table.set_implicit(true);
        doc.insert(section, Item::Table(table));
    }
}

/// 确保 section 下的子表以标准表形式存在（避免 `key = { ... }` 内联写法）
fn ensure_standard_table(doc: &mut DocumentMut, section: &str, key: &str) {
    let existing = doc
        .get(section)
        .and_then(|v| v.as_table())
        .and_then(|t| t.get(key))
        .cloned();
    match existing {
        Some(Item::Table(_)) => {}
        Some(Item::Value(Value::InlineTable(inline))) => {
            let mut table = inline.into_table();
            table.set_implicit(false);
            doc[section][key] = Item::Table(table);
        }
        _ => {
            let mut table = Table::new();
            table.set_implicit(false);
            doc[section][key] = Item::Table(table);
        }
    }
}

/// 把端点级设置写入 `[model_providers.<key>]`
fn write_provider_block(
    doc: &mut DocumentMut,
    key: &str,
    base_url: &str,
    api_key: &str,
    api_backend: &str,
) {
    ensure_table(doc, MODEL_PROVIDERS_SECTION);
    ensure_standard_table(doc, MODEL_PROVIDERS_SECTION, key);
    let block = &mut doc[MODEL_PROVIDERS_SECTION][key];
    block["base_url"] = toml_edit::value(base_url);
    block["api_key"] = toml_edit::value(api_key);
    block["api_backend"] = toml_edit::value(api_backend);
}

/// 清理模型块里的端点字段。
///
/// 端点与协议统一由中转站块承载：模型块里的 `base_url` / `api_key` / `api_backend` 一律清掉，
/// 否则旧值会盖住本次选中的中转站（模型自身字段优先级高于 provider 块）。其余字段原样保留。
fn clear_model_endpoint_fields(block: &mut Item) {
    let Some(table) = block.as_table_mut() else {
        return;
    };
    for field in ENDPOINT_FIELDS {
        table.remove(field);
    }
}

/// 按 Provider 的策略处置 `[models].web_search`。未设置时跟随默认模型。
fn apply_web_search(doc: &mut DocumentMut, provider: &Provider) -> Result<(), TargetError> {
    let policy = provider
        .grok_web_search
        .clone()
        .unwrap_or(GrokWebSearch::Follow);
    if matches!(policy, GrokWebSearch::Keep) {
        return Ok(());
    }
    ensure_table(doc, MODELS_SECTION);
    match &policy {
        GrokWebSearch::Keep => {}
        GrokWebSearch::Unset => {
            if let Some(table) = doc[MODELS_SECTION].as_table_mut() {
                table.remove("web_search");
            }
        }
        GrokWebSearch::Follow => {
            doc[MODELS_SECTION]["web_search"] = toml_edit::value(provider.model.trim());
        }
        GrokWebSearch::Model { model } => {
            doc[MODELS_SECTION]["web_search"] = toml_edit::value(model.trim());
        }
    }
    Ok(())
}

fn collect_providers(doc: &DocumentMut) -> BTreeMap<String, Endpoint> {
    let mut providers = BTreeMap::new();
    if let Some(table) = doc.get(MODEL_PROVIDERS_SECTION).and_then(|v| v.as_table()) {
        for (key, item) in table.iter() {
            if let Some(block) = item.as_table() {
                let field = |name: &str| {
                    block
                        .get(name)
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string()
                };
                providers.insert(
                    key.to_string(),
                    Endpoint {
                        base_url: field("base_url"),
                        api_key: field("api_key"),
                        api_backend: field("api_backend"),
                    },
                );
            }
        }
    }
    providers
}

/// 逐个模型块解析出有效端点：模型块自带字段优先，其次跟随 model_provider 引用
fn collect_model_endpoints(doc: &DocumentMut) -> Vec<ModelEntry> {
    let mut entries = Vec::new();
    let providers = collect_providers(doc);
    let Some(models) = doc.get(MODEL_SECTION).and_then(|v| v.as_table()) else {
        return entries;
    };

    for (model, item) in models.iter() {
        let Some(block) = item.as_table() else {
            continue;
        };
        let field = |name: &str| {
            block
                .get(name)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };

        let mut base_url = field("base_url");
        let mut api_key = field("api_key");
        let mut api_backend = field("api_backend");

        let provider_ref = block
            .get("model_provider")
            .and_then(|v| v.as_str())
            .and_then(|key| providers.get(key));
        if let Some(provider) = provider_ref {
            if base_url.is_empty() {
                base_url = provider.base_url.clone();
            }
            if api_key.is_empty() {
                api_key = provider.api_key.clone();
            }
            if api_backend.is_empty() {
                api_backend = provider.api_backend.clone();
            }
        }

        let has_endpoint = !(base_url.is_empty() && api_key.is_empty());
        entries.push(ModelEntry {
            model: model.to_string(),
            endpoint: has_endpoint.then_some(Endpoint {
                base_url,
                api_key,
                api_backend,
            }),
        });
    }
    entries
}

fn backup(path: &Path) -> Result<(), TargetError> {
    if !path.exists() {
        return Ok(());
    }
    let data = fs::read(path).map_err(|err| TargetError::ReadError {
        path: path.to_path_buf(),
        source: err,
    })?;
    let mut backup_path = path.to_path_buf();
    let new_name = match path.file_name() {
        Some(name) => {
            let mut s = name.to_os_string();
            s.push(".bak");
            s
        }
        None => return Ok(()),
    };
    backup_path.set_file_name(new_name);

    write_secure_file(&backup_path, &data).map_err(|err| TargetError::WriteError {
        path: backup_path.clone(),
        source: err,
    })?;

    Ok(())
}

fn write_secure_file<P: AsRef<Path>>(path: P, contents: &[u8]) -> std::io::Result<()> {
    fs::write(&path, contents)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&path, perms)?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const SAMPLE_CONFIG: &str = r#"[cli]
installer = "internal"

[models]
default = "grok-4.5"
default_reasoning_effort = "high"

[model."grok-4.5"]
base_url = "https://old.example.com/v1"
api_key = "sk-old-key"
api_backend = "chat_completions"
context_window = 222000
reasoning_effort = "high"

[marketplace]
official_marketplace_auto_installed = true

[ui]
theme = "grokday"
yolo = false
"#;

    fn setup_grok_dir() -> (TempDir, GrokAdapter) {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(CONFIG_FILE_NAME), SAMPLE_CONFIG).unwrap();
        let adapter = GrokAdapter::new_with_dir(dir.path());
        (dir, adapter)
    }

    fn provider(name: &str, base_url: &str, api_key: &str, model: &str) -> Provider {
        Provider {
            name: name.to_string(),
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            wire_api: "responses".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
            grok_web_search: None,
        }
    }

    fn parse_adapter(adapter: &GrokAdapter) -> DocumentMut {
        fs::read_to_string(adapter.config_path())
            .unwrap()
            .parse::<DocumentMut>()
            .unwrap()
    }

    /// 把 TOML 值读成字符串（字符串、整数与布尔都支持，便于断言）
    fn value_to_string(value: &Item) -> String {
        if let Some(s) = value.as_str() {
            return s.to_string();
        }
        if let Some(i) = value.as_integer() {
            return i.to_string();
        }
        if let Some(b) = value.as_bool() {
            return b.to_string();
        }
        value.to_string().trim().to_string()
    }

    fn model_field(adapter: &GrokAdapter, model: &str, field: &str) -> Option<String> {
        let doc = parse_adapter(adapter);
        doc.get(MODEL_SECTION)
            .and_then(|v| v.as_table())
            .and_then(|t| t.get(model))
            .and_then(|item| item.get(field))
            .map(value_to_string)
    }

    fn provider_field(adapter: &GrokAdapter, key: &str, field: &str) -> Option<String> {
        let doc = parse_adapter(adapter);
        doc.get(MODEL_PROVIDERS_SECTION)
            .and_then(|v| v.as_table())
            .and_then(|t| t.get(key))
            .and_then(|item| item.get(field))
            .map(value_to_string)
    }

    fn default_model(adapter: &GrokAdapter) -> Option<String> {
        let doc = parse_adapter(adapter);
        doc.get(MODELS_SECTION)
            .and_then(|v| v.as_table())
            .and_then(|t| t.get("default"))
            .map(value_to_string)
    }

    #[test]
    fn test_name_returns_grok() {
        let (_dir, adapter) = setup_grok_dir();
        assert_eq!(adapter.name(), "Grok");
    }

    #[test]
    fn test_read() {
        let (_dir, adapter) = setup_grok_dir();
        let cfg = adapter.read().unwrap();
        assert_eq!(cfg.model, "grok-4.5");
        assert_eq!(cfg.base_url, "https://old.example.com/v1");
        assert_eq!(cfg.api_key, "sk-old-key");
        assert_eq!(cfg.wire_api, "chat_completions");
    }

    #[test]
    fn test_read_follows_provider_reference() {
        let dir = tempfile::tempdir().unwrap();
        let config = r#"[models]
default = "grok-4.6"

[model_providers."g2a"]
base_url = "https://g2a.example.com/v1"
api_key = "g2a-key"
api_backend = "responses"

[model."grok-4.6"]
model = "grok-4.6"
model_provider = "g2a"
"#;
        fs::write(dir.path().join(CONFIG_FILE_NAME), config).unwrap();
        let adapter = GrokAdapter::new_with_dir(dir.path());

        let cfg = adapter.read().unwrap();
        assert_eq!(cfg.model, "grok-4.6");
        assert_eq!(cfg.base_url, "https://g2a.example.com/v1");
        assert_eq!(cfg.api_key, "g2a-key");
        assert_eq!(cfg.wire_api, "responses");
    }

    #[test]
    fn test_read_model_level_fields_win_over_provider() {
        let dir = tempfile::tempdir().unwrap();
        let config = r#"[models]
default = "grok-4.6"

[model_providers."g2a"]
base_url = "https://g2a.example.com/v1"
api_key = "g2a-key"
api_backend = "responses"

[model."grok-4.6"]
model = "grok-4.6"
model_provider = "g2a"
api_key = "per-model-key"
"#;
        fs::write(dir.path().join(CONFIG_FILE_NAME), config).unwrap();
        let adapter = GrokAdapter::new_with_dir(dir.path());

        let cfg = adapter.read().unwrap();
        assert_eq!(cfg.base_url, "https://g2a.example.com/v1");
        assert_eq!(cfg.api_key, "per-model-key");
    }

    #[test]
    fn test_read_missing_provider_reference_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let config = r#"[models]
default = "grok-4.6"

[model."grok-4.6"]
model = "grok-4.6"
model_provider = "nope"
"#;
        fs::write(dir.path().join(CONFIG_FILE_NAME), config).unwrap();
        let adapter = GrokAdapter::new_with_dir(dir.path());

        let cfg = adapter.read().unwrap();
        assert_eq!(cfg.model, "grok-4.6");
        assert_eq!(cfg.base_url, "");
        assert_eq!(cfg.api_key, "");
    }

    #[test]
    fn test_write_provider_writes_provider_block_and_reference() {
        let (_dir, adapter) = setup_grok_dir();
        let p = provider("g2a", "https://g2a.example.com/v1", "g2a-key", "grok-4.5");
        adapter.write_provider(&p).unwrap();

        assert_eq!(provider_field(&adapter, "g2a", "base_url").unwrap(), "https://g2a.example.com/v1");
        assert_eq!(provider_field(&adapter, "g2a", "api_key").unwrap(), "g2a-key");
        assert_eq!(provider_field(&adapter, "g2a", "api_backend").unwrap(), "responses");
        assert_eq!(model_field(&adapter, "grok-4.5", "model_provider").unwrap(), "g2a");
        assert_eq!(model_field(&adapter, "grok-4.5", "model").unwrap(), "grok-4.5");
        assert_eq!(default_model(&adapter).unwrap(), "grok-4.5");

        // 端点字段已收拢到中转站块
        assert!(model_field(&adapter, "grok-4.5", "base_url").is_none());
        assert!(model_field(&adapter, "grok-4.5", "api_key").is_none());
        assert!(model_field(&adapter, "grok-4.5", "api_backend").is_none());
        // 其它模型自身字段不受影响
        assert_eq!(model_field(&adapter, "grok-4.5", "context_window").unwrap(), "222000");
        assert_eq!(model_field(&adapter, "grok-4.5", "reasoning_effort").unwrap(), "high");

        let cfg = adapter.read().unwrap();
        assert_eq!(cfg.base_url, "https://g2a.example.com/v1");
        assert_eq!(cfg.api_key, "g2a-key");
        assert_eq!(cfg.wire_api, "responses");
    }

    #[test]
    fn test_write_provider_key_with_dots_is_quoted() {
        let (_dir, adapter) = setup_grok_dir();
        let p = provider(
            "g2a.serverholdplease.lol",
            "https://g2a.serverholdplease.lol/v1",
            "g2a-key",
            "grok-4.6",
        );
        adapter.write_provider(&p).unwrap();

        let content = fs::read_to_string(adapter.config_path()).unwrap();
        assert!(
            content.contains("[model_providers.\"g2a.serverholdplease.lol\"]"),
            "provider key should be quoted, got:\n{}",
            content
        );
        assert!(
            model_field(&adapter, "grok-4.6", "model_provider").unwrap() == "g2a.serverholdplease.lol"
        );
    }

    #[test]
    fn test_write_provider_clears_stale_endpoint_fields() {
        let (_dir, adapter) = setup_grok_dir();
        // 模型块里是旧中转站的端点，切换后必须清掉，否则旧值会盖住新中转站
        let p = provider("g2a", "https://g2a.example.com/v1", "g2a-key", "grok-4.5");
        adapter.write_provider(&p).unwrap();

        assert!(model_field(&adapter, "grok-4.5", "base_url").is_none());
        assert_eq!(
            provider_field(&adapter, "g2a", "base_url").unwrap(),
            "https://g2a.example.com/v1"
        );

        let cfg = adapter.read().unwrap();
        assert_eq!(cfg.base_url, "https://g2a.example.com/v1");
        assert_eq!(cfg.api_key, "g2a-key");
    }

    #[test]
    fn test_write_provider_leaves_other_model_blocks() {
        let (_dir, adapter) = setup_grok_dir();
        let p = provider("g2a", "https://g2a.example.com/v1", "g2a-key", "grok-4.6");

        adapter.write_provider(&p).unwrap();

        assert_eq!(
            model_field(&adapter, "grok-4.6", "model_provider").unwrap(),
            "g2a"
        );
        assert_eq!(default_model(&adapter).unwrap(), "grok-4.6");
        // 只写默认模型，不改动文件里已有的其它模型块
        assert!(model_field(&adapter, "grok-4.5", "model_provider").is_none());
        assert_eq!(
            model_field(&adapter, "grok-4.5", "base_url").unwrap(),
            "https://old.example.com/v1"
        );
    }

    #[test]
    fn test_write_provider_preserves_unrelated_sections() {
        let (_dir, adapter) = setup_grok_dir();
        let p = provider("g2a", "https://g2a.example.com/v1", "g2a-key", "grok-4.5");
        adapter.write_provider(&p).unwrap();

        let content = fs::read_to_string(adapter.config_path()).unwrap();
        for check in [
            "installer",
            "marketplace",
            "theme",
            "yolo",
            "default_reasoning_effort",
        ] {
            assert!(
                content.contains(check),
                "Unrelated key '{}' was lost:\n{}",
                check,
                content
            );
        }
    }

    #[test]
    fn test_write_provider_web_search_defaults_to_follow() {
        let (_dir, adapter) = setup_grok_dir();
        let p = provider("g2a", "https://g2a.example.com/v1", "g2a-key", "grok-4.6");
        assert!(p.grok_web_search.is_none());

        adapter.write_provider(&p).unwrap();

        let doc = parse_adapter(&adapter);
        assert_eq!(
            doc[MODELS_SECTION].get("web_search").and_then(|v| v.as_str()),
            Some("grok-4.6")
        );
    }

    #[test]
    fn test_write_provider_web_search_follow() {
        let (_dir, adapter) = setup_grok_dir();
        let mut p = provider("g2a", "https://g2a.example.com/v1", "g2a-key", "grok-4.6");
        p.grok_web_search = Some(GrokWebSearch::Follow);

        adapter.write_provider(&p).unwrap();

        let doc = parse_adapter(&adapter);
        assert_eq!(
            doc[MODELS_SECTION].get("web_search").and_then(|v| v.as_str()),
            Some("grok-4.6")
        );
    }

    #[test]
    fn test_write_provider_web_search_custom_model_gets_reference() {
        let (_dir, adapter) = setup_grok_dir();
        let mut p = provider("g2a", "https://g2a.example.com/v1", "g2a-key", "grok-4.6");
        p.grok_web_search = Some(GrokWebSearch::Model {
            model: "grok-4.5".to_string(),
        });

        adapter.write_provider(&p).unwrap();

        let doc = parse_adapter(&adapter);
        assert_eq!(
            doc[MODELS_SECTION].get("web_search").and_then(|v| v.as_str()),
            Some("grok-4.5")
        );
        // web_search 指到的模型也必须能解析到该中转站
        assert_eq!(
            model_field(&adapter, "grok-4.5", "model_provider").unwrap(),
            "g2a"
        );
    }

    #[test]
    fn test_write_provider_web_search_unset_removes_field() {
        let dir = tempfile::tempdir().unwrap();
        let config = r#"[models]
default = "grok-4.5"
web_search = "grok-4.5"

[model."grok-4.5"]
base_url = "https://old.example.com/v1"
api_key = "sk-old-key"
api_backend = "chat_completions"
"#;
        fs::write(dir.path().join(CONFIG_FILE_NAME), config).unwrap();
        let adapter = GrokAdapter::new_with_dir(dir.path());

        let mut p = provider("g2a", "https://g2a.example.com/v1", "g2a-key", "grok-4.5");
        p.grok_web_search = Some(GrokWebSearch::Unset);
        adapter.write_provider(&p).unwrap();

        let doc = parse_adapter(&adapter);
        assert!(doc[MODELS_SECTION].get("web_search").is_none());
    }

    #[test]
    fn test_write_provider_web_search_keep_leaves_field_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let config = r#"[models]
default = "grok-4.5"
web_search = "other-model"

[model."grok-4.5"]
base_url = "https://old.example.com/v1"
api_key = "sk-old-key"
api_backend = "chat_completions"
"#;
        fs::write(dir.path().join(CONFIG_FILE_NAME), config).unwrap();
        let adapter = GrokAdapter::new_with_dir(dir.path());

        let mut p = provider("g2a", "https://g2a.example.com/v1", "g2a-key", "grok-4.5");
        p.grok_web_search = Some(GrokWebSearch::Keep);
        adapter.write_provider(&p).unwrap();

        let doc = parse_adapter(&adapter);
        assert_eq!(
            doc[MODELS_SECTION].get("web_search").and_then(|v| v.as_str()),
            Some("other-model")
        );
    }

    #[test]
    fn test_write_provider_requires_name() {
        let (_dir, adapter) = setup_grok_dir();
        let p = provider("   ", "https://g2a.example.com/v1", "g2a-key", "grok-4.5");
        assert!(adapter.write_provider(&p).is_err());
    }

    #[test]
    fn test_write() {
        let (dir, adapter) = setup_grok_dir();
        let new_cfg = TargetConfig {
            base_url: "https://new.example.com/v1".to_string(),
            api_key: "sk-new-key".to_string(),
            model: "grok-4.5".to_string(),
            wire_api: "chat_completions".to_string(),
        };
        adapter.write(&new_cfg).unwrap();

        let got = adapter.read().unwrap();
        assert_eq!(got.base_url, new_cfg.base_url);
        assert_eq!(got.api_key, new_cfg.api_key);
        assert_eq!(got.model, new_cfg.model);
        assert_eq!(got.wire_api, "chat_completions");

        assert!(dir.path().join("config.toml.bak").exists());
    }

    #[test]
    fn test_write_maps_wire_api() {
        assert_eq!(
            GrokAdapter::map_wire_api_to_api_backend(""),
            "chat_completions"
        );
        assert_eq!(
            GrokAdapter::map_wire_api_to_api_backend("chat"),
            "chat_completions"
        );
        assert_eq!(
            GrokAdapter::map_wire_api_to_api_backend("responses"),
            "responses"
        );
        assert_eq!(
            GrokAdapter::map_wire_api_to_api_backend("messages"),
            "messages"
        );
        assert_eq!(
            GrokAdapter::map_wire_api_to_api_backend("custom"),
            "custom"
        );
    }

    #[test]
    fn test_write_sets_api_backend_responses() {
        let (_dir, adapter) = setup_grok_dir();
        let p = provider("relay", "https://relay.example.com/v1", "sk-relay", "grok-4.5");
        adapter.write_provider(&p).unwrap();

        let content = fs::read_to_string(adapter.config_path()).unwrap();
        assert!(
            content.contains("api_backend = \"responses\""),
            "expected api_backend responses, got:\n{}",
            content
        );

        let got = adapter.read().unwrap();
        assert_eq!(got.wire_api, "responses");
    }

    #[test]
    fn test_write_creates_files_from_scratch() {
        let dir = tempfile::tempdir().unwrap();
        let adapter = GrokAdapter::new_with_dir(dir.path());

        let p = provider("cxc", "https://new.example.com/v1", "sk-new-key", "grok-build");
        adapter.write_provider(&p).unwrap();

        let got = adapter.read().unwrap();
        assert_eq!(got.base_url, "https://new.example.com/v1");
        assert_eq!(got.api_key, "sk-new-key");
        assert_eq!(got.model, "grok-build");
        assert_eq!(got.wire_api, "responses");
        assert!(!dir.path().join("config.toml.bak").exists());
    }

    #[test]
    fn test_write_creates_directory_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("sub").join("grok");
        let adapter = GrokAdapter::new_with_dir(&nested);

        let p = provider("g2a", "https://g2a.example.com/v1", "g2a-key", "grok-4.5");
        adapter.write_provider(&p).unwrap();

        assert!(nested.join(CONFIG_FILE_NAME).exists());
        let got = adapter.read().unwrap();
        assert_eq!(got.base_url, "https://g2a.example.com/v1");
    }

    #[test]
    fn test_write_switches_model_default() {
        let (_dir, adapter) = setup_grok_dir();
        let p = provider("g2a", "https://g2a.example.com/v1", "g2a-key", "custom-model");
        adapter.write_provider(&p).unwrap();

        let got = adapter.read().unwrap();
        assert_eq!(got.model, "custom-model");
        assert_eq!(got.base_url, "https://g2a.example.com/v1");

        let content = fs::read_to_string(adapter.config_path()).unwrap();
        assert!(content.contains("default = \"custom-model\""));
        // Old model section should still exist (we only write the managed ones)
        assert!(content.contains("grok-4.5") || content.contains("old.example.com"));
    }

    #[test]
    fn test_new_from_config_uses_loaded_custom_dir() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = crate::config::Config {
            grok_source: Some("wsl".to_string()),
            grok_custom_dir: dir.path().to_string_lossy().to_string(),
            ..crate::config::Config::default()
        };

        let adapter = GrokAdapter::new_from_config(&cfg).unwrap();
        assert_eq!(adapter.config_path(), dir.path().join(CONFIG_FILE_NAME));
    }

    #[test]
    fn test_write_produces_valid_toml() {
        let (_dir, adapter) = setup_grok_dir();
        let p = provider("g2a", "https://g2a.example.com/v1", "g2a-key", "grok-4.5");
        adapter.write_provider(&p).unwrap();

        let data = fs::read_to_string(adapter.config_path()).unwrap();
        let parsed = data.parse::<DocumentMut>();
        assert!(parsed.is_ok(), "Written TOML is invalid: {:?}", parsed.err());
    }

    // ---- 一次性迁移 ----

    const MODEL_CENTRIC_CONFIG: &str = r#"[models]
default = "grok-4.6"
web_search = "grok-4.5"

# g2a 中转站
[model."grok-4.6"]
base_url = "https://g2a.example.com/v1"
api_key = "g2a-key"
api_backend = "responses"
context_window = 222000

[model."grok-4.5"]
base_url = "https://g2a.example.com/v1"
api_key = "g2a-key"
api_backend = "responses"
reasoning_effort = "high"

[model."grok-4.7"]
base_url = "https://x666.example.com/v1"
api_key = "x666-key"
api_backend = "responses"

[model."official-model"]
context_window = 500000
"#;

    #[test]
    fn test_reorganize_creates_relay_blocks_and_references() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(CONFIG_FILE_NAME), MODEL_CENTRIC_CONFIG).unwrap();
        let adapter = GrokAdapter::new_with_dir(dir.path());
        let cfg = crate::config::Config::default();

        let report = adapter.reorganize_provider_centric(&cfg).unwrap();

        assert_eq!(report.models_grouped, 3);
        assert_eq!(report.skipped_models, vec!["official-model".to_string()]);
        assert_eq!(report.created_providers.len(), 2);
        assert!(report.created_providers.contains(&"g2a.example.com".to_string()));
        assert!(report.created_providers.contains(&"x666.example.com".to_string()));
        assert_eq!(report.web_search, Some("grok-4.5".to_string()));

        // 共享同一端点的两个模型收拢到一个块
        assert_eq!(
            model_field(&adapter, "grok-4.6", "model_provider").unwrap(),
            "g2a.example.com"
        );
        assert_eq!(
            model_field(&adapter, "grok-4.5", "model_provider").unwrap(),
            "g2a.example.com"
        );
        assert_eq!(
            model_field(&adapter, "grok-4.7", "model_provider").unwrap(),
            "x666.example.com"
        );
        // 重复的端点字段被清理，模型自身字段保留
        assert!(model_field(&adapter, "grok-4.6", "base_url").is_none());
        assert!(model_field(&adapter, "grok-4.6", "api_key").is_none());
        assert_eq!(model_field(&adapter, "grok-4.6", "context_window").unwrap(), "222000");
        assert_eq!(model_field(&adapter, "grok-4.5", "reasoning_effort").unwrap(), "high");
        // 无法归类的模型块保持原样
        assert_eq!(
            model_field(&adapter, "official-model", "context_window").unwrap(),
            "500000"
        );
        assert!(model_field(&adapter, "official-model", "model_provider").is_none());
        // 注释保留
        let content = fs::read_to_string(adapter.config_path()).unwrap();
        assert!(content.contains("# g2a 中转站"));
        assert!(dir.path().join("config.toml.bak").exists());
    }

    #[test]
    fn test_reorganize_uses_cxc_provider_name() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(CONFIG_FILE_NAME), MODEL_CENTRIC_CONFIG).unwrap();
        let adapter = GrokAdapter::new_with_dir(dir.path());
        let cfg = crate::config::Config {
            grok_providers: vec![provider(
                "mustore",
                "https://g2a.example.com/v1",
                "g2a-key",
                "grok-4.6",
            )],
            ..crate::config::Config::default()
        };

        let report = adapter.reorganize_provider_centric(&cfg).unwrap();

        assert!(report.created_providers.contains(&"mustore".to_string()));
        assert_eq!(
            model_field(&adapter, "grok-4.6", "model_provider").unwrap(),
            "mustore"
        );
        assert_eq!(
            model_field(&adapter, "grok-4.5", "model_provider").unwrap(),
            "mustore"
        );
        // 另一个中转站仍按主机名建块
        assert_eq!(
            model_field(&adapter, "grok-4.7", "model_provider").unwrap(),
            "x666.example.com"
        );
    }

    #[test]
    fn test_reorganize_renames_existing_block_to_cxc_name() {
        let dir = tempfile::tempdir().unwrap();
        let config = r#"[models]
default = "grok-4.6"

[model_providers."g2a.example.com"]
base_url = "https://g2a.example.com/v1"
api_key = "g2a-key"
api_backend = "responses"

[model."grok-4.6"]
model = "grok-4.6"
model_provider = "g2a.example.com"
"#;
        fs::write(dir.path().join(CONFIG_FILE_NAME), config).unwrap();
        let adapter = GrokAdapter::new_with_dir(dir.path());
        let cfg = crate::config::Config {
            grok_providers: vec![provider(
                "mustore",
                "https://g2a.example.com/v1",
                "g2a-key",
                "grok-4.6",
            )],
            ..crate::config::Config::default()
        };

        let report = adapter.reorganize_provider_centric(&cfg).unwrap();

        assert_eq!(
            report.renamed_providers,
            vec![("g2a.example.com".to_string(), "mustore".to_string())]
        );
        assert_eq!(provider_field(&adapter, "mustore", "api_key").unwrap(), "g2a-key");
        assert!(provider_field(&adapter, "g2a.example.com", "api_key").is_none());
        assert_eq!(
            model_field(&adapter, "grok-4.6", "model_provider").unwrap(),
            "mustore"
        );
    }

    #[test]
    fn test_reorganize_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(CONFIG_FILE_NAME), MODEL_CENTRIC_CONFIG).unwrap();
        let adapter = GrokAdapter::new_with_dir(dir.path());
        let cfg = crate::config::Config::default();

        adapter.reorganize_provider_centric(&cfg).unwrap();
        let first = fs::read_to_string(adapter.config_path()).unwrap();
        let report = adapter.reorganize_provider_centric(&cfg).unwrap();
        let second = fs::read_to_string(adapter.config_path()).unwrap();

        assert_eq!(first, second);
        assert!(report.created_providers.is_empty());
        assert!(report.renamed_providers.is_empty());
        assert_eq!(report.models_grouped, 3);
        assert_eq!(report.reused_providers.len(), 2);
    }

    #[test]
    fn test_reorganize_no_model_section() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(CONFIG_FILE_NAME), "[models]\ndefault = \"\"\n").unwrap();
        let adapter = GrokAdapter::new_with_dir(dir.path());

        let report = adapter
            .reorganize_provider_centric(&crate::config::Config::default())
            .unwrap();
        assert_eq!(report, GrokReorgReport::default());
    }

    #[test]
    fn test_write_provider_keeps_model_subtables() {
        let dir = tempfile::tempdir().unwrap();
        let config = r#"[models]
default = "gpt-5.6-terra"

[model_providers."old"]
base_url = "http://47.88.78.223:8080/v1"
api_key = "old-key"
api_backend = "responses"
extra_headers = { OpenAI-Service-Tier = "fast" }

[model."gpt-5.6-terra"]
model_provider = "old"

[model."gpt-5.6-terra".extra_headers]
OpenAI-Service-Tier = "fast"
"#;
        fs::write(dir.path().join(CONFIG_FILE_NAME), config).unwrap();
        let adapter = GrokAdapter::new_with_dir(dir.path());
        let p = provider("mustore", "http://47.88.78.223:8080/v1", "new-key", "gpt-5.6-terra");
        adapter.write_provider(&p).unwrap();

        let doc = parse_adapter(&adapter);
        // 中转站块里的额外字段保留
        let provider_headers = doc[MODEL_PROVIDERS_SECTION]["old"]
            .get("extra_headers")
            .map(value_to_string)
            .unwrap_or_default();
        assert!(
            provider_headers.contains("OpenAI-Service-Tier") && provider_headers.contains("fast"),
            "provider extra_headers should survive, got: {}",
            provider_headers
        );
        // 模型块里的子表保留
        assert!(doc[MODEL_SECTION]["gpt-5.6-terra"]
            .get("extra_headers")
            .is_some());
        assert_eq!(
            model_field(&adapter, "gpt-5.6-terra", "model_provider").unwrap(),
            "mustore"
        );
    }

    #[test]
    fn test_reorganize_keeps_provider_extra_fields() {
        let dir = tempfile::tempdir().unwrap();
        let config = r#"[models]
default = "grok-4.6"

[model_providers."mustore"]
base_url = "https://g2a.example.com/v1"
api_key = "g2a-key"
api_backend = "responses"
max_request_bytes = 25000000

[model."grok-4.6"]
base_url = "https://g2a.example.com/v1"
api_key = "g2a-key"
api_backend = "responses"
"#;
        fs::write(dir.path().join(CONFIG_FILE_NAME), config).unwrap();
        let adapter = GrokAdapter::new_with_dir(dir.path());

        let report = adapter
            .reorganize_provider_centric(&crate::config::Config::default())
            .unwrap();

        assert_eq!(report.reused_providers, vec!["mustore".to_string()]);
        assert!(report.created_providers.is_empty());
        let doc = parse_adapter(&adapter);
        assert_eq!(
            doc[MODEL_PROVIDERS_SECTION]["mustore"]
                .get("max_request_bytes")
                .and_then(|v| v.as_integer()),
            Some(25000000)
        );
        assert_eq!(
            model_field(&adapter, "grok-4.6", "model_provider").unwrap(),
            "mustore"
        );
        assert!(model_field(&adapter, "grok-4.6", "base_url").is_none());
    }

    #[test]
    fn test_host_key_and_unique_key() {
        assert_eq!(host_key("https://g2a.serverholdplease.lol/v1"), "g2a.serverholdplease.lol");
        assert_eq!(host_key("http://47.116.29.231:8080/v1"), "47.116.29.231");
        assert_eq!(host_key(""), "relay");
        assert_eq!(host_key("not a url"), "not a url");

        let taken: HashSet<String> = ["relay".to_string()].into_iter().collect();
        assert_eq!(unique_key("relay", &taken), "relay-2");
        assert_eq!(unique_key("fresh", &taken), "fresh");
    }
}

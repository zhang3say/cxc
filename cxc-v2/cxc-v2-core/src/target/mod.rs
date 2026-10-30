pub mod claude;
pub mod codex;
pub mod grok;
pub mod opencode;

use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::agent::{AgentConfig, AgentKind};
use crate::config::Env;
use crate::supplier::Supplier;

#[derive(Error, Debug)]
pub enum TargetError {
    #[error("读取配置文件失败 {path}: {source}")]
    ReadError {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("解析配置失败: {0}")]
    ParseError(String),
    #[error("序列化配置失败: {0}")]
    SerializeError(String),
    #[error("写入配置文件失败 {path}: {source}")]
    WriteError {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("找不到用户家目录")]
    NoHomeDir,
    #[error("找不到 {agent} 在 {env} 环境下的配置目录：{path}\n\n建议：在设置里填写自定义目录")]
    ConfigDirNotFound {
        agent: &'static str,
        env: &'static str,
        path: String,
    },
}

pub trait AgentAdapter {
    fn kind(&self) -> AgentKind;
    fn config_path(&self, env: Env, custom_dir: Option<&Path>) -> Result<PathBuf, TargetError>;
    fn apply(
        &self,
        supplier: &Supplier,
        config: &AgentConfig,
        custom_dir: Option<&Path>,
        env: Env,
    ) -> Result<(), TargetError>;
}

pub fn adapter_for(kind: AgentKind) -> Box<dyn AgentAdapter> {
    match kind {
        AgentKind::Codex => Box::new(codex::CodexAdapter::new()),
        AgentKind::Claude => Box::new(claude::ClaudeAdapter::new()),
        AgentKind::Grok => Box::new(grok::GrokAdapter::new()),
        AgentKind::OpenCode => Box::new(opencode::OpenCodeAdapter::new()),
    }
}

/// 解析 agent 在指定环境下的配置目录。自定义目录优先；否则按环境推导。
pub fn resolve_config_dir(
    kind: AgentKind,
    env: Env,
    custom_dir: Option<&Path>,
) -> Result<PathBuf, TargetError> {
    if let Some(dir) = custom_dir {
        return Ok(dir.to_path_buf());
    }

    #[cfg(target_os = "linux")]
    {
        match env {
            Env::Wsl => {
                let home = dirs::home_dir().ok_or(TargetError::NoHomeDir)?;
                Ok(default_dir_in_home(kind, &home))
            }
            Env::Windows => {
                windows_home_from_wsl(kind).ok_or_else(|| TargetError::ConfigDirNotFound {
                    agent: kind.display_name(),
                    env: env.label(),
                    path: "Windows 用户目录".to_string(),
                })
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        match env {
            Env::Windows => {
                let home = dirs::home_dir().ok_or(TargetError::NoHomeDir)?;
                if kind == AgentKind::OpenCode {
                    if let Some(appdata) = dirs::config_dir() {
                        return Ok(appdata.join("opencode"));
                    }
                }
                Ok(default_dir_in_home(kind, &home))
            }
            Env::Wsl => {
                let home =
                    wsl_home_from_windows().ok_or_else(|| TargetError::ConfigDirNotFound {
                        agent: kind.display_name(),
                        env: env.label(),
                        path: "WSL 用户目录".to_string(),
                    })?;
                Ok(default_dir_in_home(kind, &home))
            }
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        match env {
            Env::Wsl => {
                let home = dirs::home_dir().ok_or(TargetError::NoHomeDir)?;
                Ok(default_dir_in_home(kind, &home))
            }
            Env::Windows => Err(TargetError::ConfigDirNotFound {
                agent: kind.display_name(),
                env: env.label(),
                path: "Windows 用户目录".to_string(),
            }),
        }
    }
}

pub fn default_dir_in_home(kind: AgentKind, home: &Path) -> PathBuf {
    match kind {
        AgentKind::Codex => home.join(".codex"),
        AgentKind::Claude => home.join(".claude"),
        AgentKind::Grok => home.join(".grok"),
        AgentKind::OpenCode => home.join(".config").join("opencode"),
    }
}

#[cfg(target_os = "linux")]
fn windows_home_from_wsl(kind: AgentKind) -> Option<PathBuf> {
    let username = std::env::var("USER").ok()?;
    let candidates = [
        PathBuf::from(format!("/mnt/c/Users/{username}")),
        PathBuf::from(format!("/mnt/c/Users/{}", username.to_lowercase())),
    ];
    for home in candidates {
        if !home.exists() {
            continue;
        }
        let dir = match kind {
            AgentKind::OpenCode => home.join("AppData").join("Roaming").join("opencode"),
            _ => default_dir_in_home(kind, &home),
        };
        if dir.exists() {
            return Some(dir);
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn wsl_home_from_windows() -> Option<PathBuf> {
    let output = std::process::Command::new("wsl")
        .args(["-e", "sh", "-lc", "wslpath -w \"$HOME\""])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        return None;
    }
    Some(PathBuf::from(path))
}

pub fn backup(path: &Path) -> Result<(), TargetError> {
    if !path.exists() {
        return Ok(());
    }
    let data = fs::read(path).map_err(|source| TargetError::ReadError {
        path: path.to_path_buf(),
        source,
    })?;
    let mut backup_path = path.to_path_buf();
    let name = match path.file_name() {
        Some(name) => {
            let mut name = name.to_os_string();
            name.push(".bak");
            name
        }
        None => return Ok(()),
    };
    backup_path.set_file_name(name);
    write_secure_file(&backup_path, &data).map_err(|source| TargetError::WriteError {
        path: backup_path,
        source,
    })
}

pub fn write_secure_file(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

pub fn read_to_string(path: &Path) -> Result<String, TargetError> {
    fs::read_to_string(path).map_err(|source| TargetError::ReadError {
        path: path.to_path_buf(),
        source,
    })
}

pub fn provider_key(supplier: &Supplier) -> String {
    format!("cxc-{}", crate::supplier::short_id(&supplier.id))
}

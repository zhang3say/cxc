use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::agent::{AgentKind, CustomDirs};
use crate::config::Source;
use crate::target;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DetectedAgent {
    pub kind: AgentKind,
    pub installed: bool,
    pub binary: Option<String>,
    pub config_dir: Option<String>,
    pub configured: bool,
    pub source: Source,
}

pub fn detect_all(source: Source, custom_dirs: &CustomDirs) -> Vec<DetectedAgent> {
    AgentKind::ALL
        .iter()
        .map(|kind| detect_one(*kind, source, custom_dirs))
        .collect()
}

pub fn detect_one(kind: AgentKind, source: Source, custom_dirs: &CustomDirs) -> DetectedAgent {
    let binary = find_binary(kind.binary_name(), source);
    let config_dir = target::resolve_config_dir(kind, source, custom_dirs.get(kind)).ok();
    let configured = config_dir.as_ref().map(|dir| dir.exists()).unwrap_or(false);

    DetectedAgent {
        kind,
        installed: binary.is_some(),
        binary: binary.map(|path| path.to_string_lossy().to_string()),
        config_dir: config_dir.map(|path| path.to_string_lossy().to_string()),
        configured,
        source,
    }
}

fn find_binary(name: &str, source: Source) -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        if source == Source::App
            && let Some(path) = windows_which(name)
        {
            return Some(path);
        }
    }
    #[cfg(target_os = "windows")]
    {
        if source == Source::Wsl {
            return wsl_which(name);
        }
    }
    let _ = source;
    which::which(name).ok()
}

#[cfg(target_os = "linux")]
fn windows_which(name: &str) -> Option<PathBuf> {
    let cmd = if std::path::Path::new("/mnt/c/Windows/System32/cmd.exe").exists() {
        "/mnt/c/Windows/System32/cmd.exe"
    } else {
        "cmd.exe"
    };
    let output = std::process::Command::new(cmd)
        .args(["/c", "where", name])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()?
        .trim()
        .to_string();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

#[cfg(target_os = "windows")]
fn wsl_which(name: &str) -> Option<PathBuf> {
    let output = std::process::Command::new("wsl")
        .args(["-e", "sh", "-lc", &format!("command -v {name}")])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

/// 当前进程实际使用的 PATH，用于设置页展示与排查。
pub fn current_path() -> String {
    std::env::var("PATH").unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_current_host_agents() {
        let dirs = CustomDirs::default();
        let agents = detect_all(Source::Wsl, &dirs);
        assert_eq!(agents.len(), 4);
        assert_eq!(agents[0].kind, AgentKind::Codex);
    }
}

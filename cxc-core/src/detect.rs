use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::agent::{AgentKind, CustomDirs};
use crate::config::Env;
use crate::target;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DetectedAgent {
    pub kind: AgentKind,
    pub env: Env,
    pub installed: bool,
    pub binary: Option<String>,
    pub config_dir: Option<String>,
    pub configured: bool,
}

/// 当前机器能访问到的环境。不可达的环境不会出现在界面上。
pub fn reachable_envs() -> Vec<Env> {
    #[cfg(target_os = "linux")]
    {
        let mut envs = vec![Env::Wsl];
        if std::path::Path::new("/mnt/c/Windows/System32/cmd.exe").exists() {
            envs.push(Env::Windows);
        }
        envs
    }

    #[cfg(target_os = "windows")]
    {
        let mut envs = vec![Env::Windows];
        if wsl_available() {
            envs.insert(0, Env::Wsl);
        }
        envs
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        vec![Env::Wsl]
    }
}

pub fn detect_all(custom_dirs: &CustomDirs) -> Vec<DetectedAgent> {
    let mut detected = Vec::new();
    for env in reachable_envs() {
        for kind in AgentKind::ALL {
            detected.push(detect_one(kind, env, custom_dirs));
        }
    }
    detected
}

pub fn detect_one(kind: AgentKind, env: Env, custom_dirs: &CustomDirs) -> DetectedAgent {
    let binary = find_binary(kind.binary_name(), env);
    let config_dir = target::resolve_config_dir(kind, env, custom_dirs.get(kind, env)).ok();
    let configured = config_dir.as_ref().map(|dir| dir.exists()).unwrap_or(false);

    DetectedAgent {
        kind,
        env,
        installed: binary.is_some(),
        binary: binary.map(|path| path.to_string_lossy().to_string()),
        config_dir: config_dir.map(|path| path.to_string_lossy().to_string()),
        configured,
    }
}

fn find_binary(name: &str, env: Env) -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        if env == Env::Windows {
            return windows_which(name);
        }
    }
    #[cfg(target_os = "windows")]
    {
        if env == Env::Wsl {
            return wsl_which(name);
        }
    }
    let _ = env;
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
fn wsl_available() -> bool {
    std::process::Command::new("wsl")
        .args(["-e", "true"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
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
    fn reachable_envs_always_contains_local_env() {
        let envs = reachable_envs();
        assert!(!envs.is_empty());
        assert!(envs.contains(&Env::Wsl) || envs.contains(&Env::Windows));
    }

    #[test]
    fn detects_every_reachable_agent() {
        let dirs = CustomDirs::default();
        let agents = detect_all(&dirs);
        assert_eq!(agents.len(), AgentKind::ALL.len() * reachable_envs().len());
        assert!(agents.iter().any(|agent| agent.kind == AgentKind::Codex));
    }
}

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

/// 界面上环境的展示信息。`host` 表示当前进程直接运行在这个环境里。
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct EnvInfo {
    pub env: Env,
    pub label: String,
    pub host: bool,
}

pub fn env_infos() -> Vec<EnvInfo> {
    let envs = reachable_envs();
    let distro = if envs.contains(&Env::Wsl) {
        wsl_distro_name()
    } else {
        None
    };
    envs.into_iter()
        .map(|env| EnvInfo {
            env,
            label: env_label(env, distro.as_deref()),
            host: is_host_env(env),
        })
        .collect()
}

fn is_host_env(env: Env) -> bool {
    #[cfg(target_os = "windows")]
    {
        env == Env::Windows
    }
    #[cfg(target_os = "linux")]
    {
        env == Env::Wsl
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        env == Env::Wsl
    }
}

fn env_label(env: Env, distro: Option<&str>) -> String {
    match env {
        Env::Windows => "Windows".to_string(),
        Env::Wsl => match distro.map(str::trim).filter(|name| !name.is_empty()) {
            Some(name) => format!("WSL ({name})"),
            None => "WSL".to_string(),
        },
    }
}

fn wsl_distro_name() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let output = crate::process::command("wsl")
            .args(["-e", "sh", "-lc", "printf %s \"$WSL_DISTRO_NAME\""])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return (!name.is_empty()).then_some(name);
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("WSL_DISTRO_NAME")
            .ok()
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty())
    }
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
    which::which(name).ok().or_else(|| {
        let home = dirs::home_dir()?;
        find_in_candidates(name, &home)
    })
}

/// 各家安装脚本常把可执行文件放进自己的目录，且只写入交互式 shell 的配置。
/// 这里补一份固定候选路径，避免非登录/非交互环境里 `which` 找不到。
fn candidate_bins(name: &str) -> &'static [&'static str] {
    match name {
        "codex" => &[
            "$HOME/.local/bin/codex",
            "$HOME/.codex/bin/codex",
            "/usr/local/bin/codex",
        ],
        "claude" => &[
            "$HOME/.local/bin/claude",
            "$HOME/.claude/local/claude",
            "/usr/local/bin/claude",
        ],
        "grok" => &[
            "$HOME/.grok/bin/grok",
            "$HOME/.local/bin/grok",
            "/usr/local/bin/grok",
            "/usr/bin/grok",
        ],
        "opencode" => &[
            "$HOME/.opencode/bin/opencode",
            "$HOME/.local/bin/opencode",
            "/usr/local/bin/opencode",
        ],
        _ => &[],
    }
}

fn candidate_paths(name: &str, home: &std::path::Path) -> Vec<PathBuf> {
    candidate_bins(name)
        .iter()
        .map(|raw| match raw.strip_prefix("$HOME/") {
            Some(rest) => home.join(rest),
            None => PathBuf::from(raw),
        })
        .collect()
}

fn find_in_candidates(name: &str, home: &std::path::Path) -> Option<PathBuf> {
    candidate_paths(name, home)
        .into_iter()
        .find(|path| is_executable(path))
}

fn is_executable(path: &std::path::Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

#[cfg(target_os = "linux")]
fn windows_which(name: &str) -> Option<PathBuf> {
    let cmd = if std::path::Path::new("/mnt/c/Windows/System32/cmd.exe").exists() {
        "/mnt/c/Windows/System32/cmd.exe"
    } else {
        "cmd.exe"
    };
    let output = crate::process::command(cmd)
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
    crate::process::command("wsl")
        .args(["-e", "true"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn wsl_which(name: &str) -> Option<PathBuf> {
    // `wsl -e sh -lc` 只加载登录配置，安装脚本写进 .bashrc 的 PATH 不会生效。
    let candidates = candidate_bins(name)
        .iter()
        .map(|raw| format!("\"{raw}\""))
        .collect::<Vec<_>>()
        .join(" ");
    let script = if candidates.is_empty() {
        format!("command -v {name}")
    } else {
        format!(
            "command -v {name} || for p in {candidates}; do [ -x \"$p\" ] && {{ echo \"$p\"; break; }}; done"
        )
    };
    let output = crate::process::command("wsl")
        .args(["-e", "sh", "-lc", &script])
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

    #[test]
    fn grok_candidates_include_installer_location() {
        let home = std::path::Path::new("/home/tester");
        assert!(
            candidate_paths("grok", home).contains(&home.join(".grok/bin/grok")),
            "Grok Build 安装脚本默认放在 ~/.grok/bin"
        );
    }

    #[test]
    fn finds_binary_outside_path() {
        let temp = tempfile::tempdir().unwrap();
        let bin_dir = temp.path().join(".grok/bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let bin = bin_dir.join("grok");
        std::fs::write(&bin, b"#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        assert_eq!(find_in_candidates("grok", temp.path()), Some(bin));
    }

    #[cfg(unix)]
    #[test]
    fn ignores_non_executable_candidate() {
        let temp = tempfile::tempdir().unwrap();
        let bin_dir = temp.path().join(".grok/bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let bin = bin_dir.join("grok");
        std::fs::write(&bin, b"not a binary").unwrap();
        assert_eq!(find_in_candidates("grok", temp.path()), None);
    }

    #[test]
    fn env_label_includes_wsl_distro() {
        assert_eq!(env_label(Env::Wsl, Some("Ubuntu")), "WSL (Ubuntu)");
        assert_eq!(env_label(Env::Wsl, Some("  ")), "WSL");
        assert_eq!(env_label(Env::Wsl, None), "WSL");
        assert_eq!(env_label(Env::Windows, Some("Ubuntu")), "Windows");
    }
}

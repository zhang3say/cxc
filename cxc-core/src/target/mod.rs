pub mod codex;
pub mod claude;
pub mod grok;

use thiserror::Error;
use std::path::PathBuf;

#[derive(Error, Debug)]
pub enum TargetError {
    #[error("Failed to read config file at {path}: {source}")]
    ReadError {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to parse config: {0}")]
    ParseError(String),
    #[error("Failed to serialize config: {0}")]
    SerializeError(String),
    #[error("Failed to write config file to {path}: {source}")]
    WriteError {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to find home directory")]
    NoHomeDir,
    #[error("Claude CLI 配置目录不存在：{path}\n\n可能原因：\n- {reason}\n\n建议：\n- {suggestion}")]
    ClaudeConfigDirNotFound {
        path: PathBuf,
        reason: String,
        suggestion: String,
    },
    #[error("Claude CLI 配置格式错误：{path}\n{details}\n\n配置文件未被修改，请手动修复后重试")]
    ClaudeConfigInvalid {
        path: PathBuf,
        details: String,
    },
}

pub struct TargetConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub wire_api: String,
}

pub trait TargetAdapter {
    fn name(&self) -> &'static str;
    fn read(&self) -> Result<TargetConfig, TargetError>;
    fn write(&self, config: &TargetConfig) -> Result<(), TargetError>;
}

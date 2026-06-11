pub mod codex;

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

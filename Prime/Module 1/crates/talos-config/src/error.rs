use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to extract configuration: {0}")]
    Extract(String),
    #[error("invalid configuration: {0}")]
    Validation(String),
}

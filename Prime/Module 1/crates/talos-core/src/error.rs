use talos_config::ConfigError;
use thiserror::Error;

/// Shared error taxonomy (M01). Retry policy is owned by workers.
#[derive(Debug, Error)]
pub enum TalosError {
    #[error("validation error: {0}")]
    Validation(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("not implemented: {0}")]
    NotImplemented(String),

    #[error("transient error: {0}")]
    Transient(String),

    #[error("permanent error: {0}")]
    Permanent(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl TalosError {
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Transient(_))
    }

    pub fn class_label(&self) -> &'static str {
        match self {
            Self::Validation(_) => "validation",
            Self::Config(_) => "config",
            Self::NotImplemented(_) => "not_implemented",
            Self::Transient(_) => "transient",
            Self::Permanent(_) => "permanent",
            Self::Internal(_) => "internal",
        }
    }

    pub fn not_implemented(component: impl Into<String>) -> Self {
        Self::NotImplemented(component.into())
    }
}

impl From<ConfigError> for TalosError {
    fn from(value: ConfigError) -> Self {
        Self::Config(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transient_is_retryable() {
        assert!(TalosError::Transient("timeout".into()).is_retryable());
        assert!(!TalosError::Validation("bad".into()).is_retryable());
        assert!(!TalosError::NotImplemented("gpu".into()).is_retryable());
    }
}

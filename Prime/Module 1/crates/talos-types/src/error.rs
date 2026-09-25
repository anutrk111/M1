use std::fmt;

/// Contract invariant violation detected inside `talos-types`.
///
/// `talos-types` has no dependency on `talos-core`; `talos-core` maps this into
/// `TalosError::Validation`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractError(pub String);

impl ContractError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "contract violation: {}", self.0)
    }
}

impl std::error::Error for ContractError {}

use std::fmt;

/// A stitch-plan or geometry failure the caller can show to a digitiser.
///
/// The message names the failed check and the useful next step. It does not
/// include file contents, credentials, or other private data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanError {
    message: String,
}

impl PlanError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for PlanError {}

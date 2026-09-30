use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Structured typed error hierarchy across all Portfolio Forge engines.
#[derive(Error, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlatformError {
    #[error("I/O Error: {0}")]
    Io(String),

    #[error("Serialization/Deserialization Error: {0}")]
    Codec(String),

    #[error("Authentication/Authorization Failure: {0}")]
    Auth(String),

    #[error("Resource Not Found: {0}")]
    NotFound(String),

    #[error("Validation Invariant Violation: {0}")]
    Validation(String),

    #[error("Rate Limit Exceeded: {0}")]
    RateLimited(String),

    #[error("Concurrency / Lock Conflict: {0}")]
    Conflict(String),

    #[error("Storage Engine Error: {0}")]
    Storage(String),

    #[error("Internal Engine Fault: {0}")]
    Internal(String),
}

/// RFC 7807 Problem Details representation for HTTP/API boundaries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProblemDetails {
    pub r#type: String,
    pub title: String,
    pub status: u16,
    pub detail: String,
    pub instance: Option<String>,
}

impl From<PlatformError> for ProblemDetails {
    fn from(err: PlatformError) -> Self {
        let (status, title) = match &err {
            PlatformError::NotFound(_) => (404, "Not Found"),
            PlatformError::Auth(_) => (401, "Unauthorized"),
            PlatformError::Validation(_) => (422, "Unprocessable Entity"),
            PlatformError::RateLimited(_) => (429, "Too Many Requests"),
            PlatformError::Conflict(_) => (409, "Conflict"),
            _ => (500, "Internal Server Error"),
        };

        Self {
            r#type: format!(
                "https://portfolioforge.dev/errors/{}",
                title.to_lowercase().replace(' ', "-")
            ),
            title: title.to_string(),
            status,
            detail: err.to_string(),
            instance: None,
        }
    }
}

pub type PlatformResult<T> = Result<T, PlatformError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_to_problem_details() {
        let err = PlatformError::NotFound("Key 42 not found in SSTable".into());
        let problem: ProblemDetails = err.into();
        assert_eq!(problem.status, 404);
        assert_eq!(problem.title, "Not Found");
        assert!(problem.detail.contains("Key 42"));
    }
}

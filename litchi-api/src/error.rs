use thiserror::Error;

#[derive(Debug, Error)]
pub enum LitchiApiError {
    #[error("Http error: {0:?}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Authentication error: {0}")]
    AuthError(String),
    #[error("HTTP error (code: {0}): {1}")]
    HTTPError(u16, String),
    #[error("Invalid mission JSON format: {0}")]
    MissionFormatError(String),
    #[error("Response format error: {0} ({1})")]
    ResponseFormateError(String, String),
}

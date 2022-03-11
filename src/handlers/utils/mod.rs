use axum::{
    http::StatusCode,
};

// Utility function for mapping errors into 500 http response
pub fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
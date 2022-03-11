use axum::{
    http::StatusCode,
    extract::{Extension}
};
use sqlx::postgres::PgPool;

pub async fn basic_handler() -> &'static str {
    ":q!"
}

pub async fn connection_pool_extractor_handler(
    Extension(pool): Extension<PgPool>
) -> Result<String, (StatusCode, String)> {
    sqlx::query_scalar("SELECT * FROM track;")
        .fetch_one(&pool)
        .await
        .map_err(internal_error)
}

// Utility function for mapping errors into 500 http response
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
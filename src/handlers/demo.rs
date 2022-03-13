use axum::{
    http::StatusCode,
    extract::{Extension}
};
use sqlx::postgres::PgPool;

use crate::utils;

pub async fn basic_handler() -> &'static str {
    ":q!"
}

pub async fn connection_pool_extractor_handler(
    Extension(pool): Extension<PgPool>
) -> Result<String, (StatusCode, String)> {
    sqlx::query_scalar("SELECT * FROM track;")
        .fetch_one(&pool)
        .await
        .map_err(|e| utils::internal_error(Box::new(e)))
}
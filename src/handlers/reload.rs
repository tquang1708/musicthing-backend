use axum::{
    http::StatusCode,
    extract::{Extension}
};
use sqlx::postgres::PgPool;

pub async fn reload_handler(
    Extension(_pool): Extension<PgPool>
) -> Result<&'static str, (StatusCode, String)> {
    Ok("TBD - reload")
}

pub async fn hard_reload_handler(
    Extension(_pool): Extension<PgPool>
) -> Result<&'static str, (StatusCode, String)> {
    Ok("TBD - hard reload")
}
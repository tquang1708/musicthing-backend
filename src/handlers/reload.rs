use axum::{
    http::StatusCode,
    extract::{Extension}
};
use sqlx::postgres::PgPool;

use crate::utils;

pub async fn reload_handler(
    Extension(pool): Extension<PgPool>
) -> Result<(), (StatusCode, String)> {
    load_db(pool).await
}

pub async fn hard_reload_handler(
    Extension(pool): Extension<PgPool>
) -> Result<(), (StatusCode, String)> {
    clear_data(pool.clone()).await?;
    load_db(pool).await
}

async fn load_db(pool: PgPool) -> Result<(), (StatusCode, String)> {
    Ok(())
}

async fn clear_data(pool: PgPool) -> Result<(), (StatusCode, String)> {
    let tables = [
        "album",
        "album_track",
        "artist",
        "artist_album",
        "artist_track",
        "track"
    ];

    for table in tables.iter() {
        match sqlx::query(format!("DELETE FROM {}", table).as_str())
            .execute(&pool)
            .await 
        {
            Ok(_) => continue,
            Err(e) => return Err(utils::internal_error(e)),
        }
    };

    Ok(())
}
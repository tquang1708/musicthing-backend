use axum::{
    http::StatusCode,
    extract::{Extension}
};
use sqlx::postgres::PgPool;

use crate::utils;

// reload_handler for loading database metadata from music directory
pub async fn reload_handler(
    Extension(pool): Extension<PgPool>
) -> Result<(), (StatusCode, String)> {
    load_db(pool).await
}

// same as above but with wiping the db beforehand
pub async fn hard_reload_handler(
    Extension(pool): Extension<PgPool>
) -> Result<(), (StatusCode, String)> {
    clear_data(pool.clone()).await?;
    load_db(pool).await
}

// load database metadata from path
async fn load_db(pool: PgPool) -> Result<(), (StatusCode, String)> {
    // get music_directory path
    let config: utils::Config;
    match utils::parse_cfg().map_err(utils::internal_error) {
        Ok(x) => config = x,
        Err(e) => return Err(e),
    }
    let music_directory = config.music_directory;
    
    remove_old_metadata(pool.clone()).await?;
    load_new_metadata(pool.clone(), music_directory).await
}

// remove old metadata from files that have been changed, or files that have been deleted
async fn remove_old_metadata(pool: PgPool) -> Result<(), (StatusCode, String)> {
    Ok(())
}

// load new metadata from given music directory path
async fn load_new_metadata(pool: PgPool, path: String) -> Result<(), (StatusCode, String)> {
    Ok(())
}

// wipe the database
async fn clear_data(pool: PgPool) -> Result<(), (StatusCode, String)> {
    // tables to clear from
    let tables = [
        "album",
        "album_track",
        "artist",
        "artist_album",
        "artist_track",
        "track"
    ];

    // iterate over tables then delete from them
    for table in tables.iter() {
        match sqlx::query(format!("DELETE FROM {}", table).as_str())
            .execute(&pool)
            .await 
        {
            Ok(_) => continue,
            Err(e) => return Err(utils::internal_error(Box::new(e))),
        }
    };

    Ok(())
}
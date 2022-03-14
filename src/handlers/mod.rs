use sqlx::types::time::PrimitiveDateTime;

pub mod demo;
pub mod reload;
pub mod list;
pub mod play;

// structs for interfacing with the database
#[derive(sqlx::FromRow, Debug)]
pub struct TrackDB {
    track_id: i32,
    track_name: Option<String>,
    path: String,
    last_modified: PrimitiveDateTime,
}

#[derive(sqlx::FromRow, Debug)]
pub struct ArtistDB {
    artist_id: i32,
    artist_name: Option<String>,
}

#[derive(sqlx::FromRow, Debug)]
pub struct AlbumDB {
    album_id: i32,
    album_name: Option<String>,
}

// constant vector of recognized extensions
pub const RECOGNIZED_EXTENSIONS: &[&str] = &["mp3", "flac"];
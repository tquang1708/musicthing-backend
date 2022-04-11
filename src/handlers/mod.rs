use sqlx::types::time::PrimitiveDateTime;

pub mod demo;
pub mod reload;
pub mod list;
pub mod play;

pub mod tag_parser;

// structs for interfacing with the database
#[derive(sqlx::FromRow, Debug)]
#[allow(dead_code)]
pub struct DBTrack {
    track_id: i32,
    track_name: Option<String>,
    path: String,
    last_modified: PrimitiveDateTime,
    length_seconds: i32,
}

#[derive(sqlx::FromRow, Debug)]
#[allow(dead_code)]
pub struct DBArtist {
    artist_id: i32,
    artist_name: Option<String>,
}

#[derive(sqlx::FromRow, Debug)]
#[allow(dead_code)]
pub struct DBAlbum {
    album_id: i32,
    album_name: Option<String>,
}

#[derive(sqlx::FromRow, Debug)]
#[allow(dead_code)]
pub struct DBArt {
    art_id: i32,
    hash: Vec<u8>,
    path: String,
}

// constant vector of recognized extensions
pub const RECOGNIZED_EXTENSIONS: &[&str] = &["mp3", "flac"];
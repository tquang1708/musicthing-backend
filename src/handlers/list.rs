use axum::{
    http::StatusCode,
    response::{Json},
    extract::{Extension},
};
use sqlx::postgres::PgPool;
use serde::{
    Serialize,
    Deserialize,
};

use std::{
    error::Error,
    path::Path
};

use crate::utils::{
    internal_error,
    parse_cfg,
};
use crate::handlers::{
    AlbumDB,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Root {
    pub albums: Vec<Album>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Album {
    pub name: String,
    pub album_artist_name: String,
    pub discs: Vec<Disc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Disc {
    pub number: i32,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Track {
    pub number: i32,
    pub artist: String,
    pub name: String,
    pub path: String,
}


pub async fn list_handler(
    Extension(pool): Extension<PgPool>
) -> Result<Json<Root>, (StatusCode, String)> {
    Ok(Json(generate_root(pool.clone()).await.map_err(internal_error)?))
}

async fn generate_root(pool: PgPool) -> Result<Root, Box<dyn Error>> {
    // gather all albums with tracks sorted alphabetically
    let albums = sqlx::query_as!(AlbumDB, "SELECT DISTINCT album.album_id, album_name FROM album \
        JOIN album_track ON (album.album_id = album_track.album_id)
        ORDER BY (album_name)")
        .fetch_all(&pool)
        .await?;
    
    // iterate over albums to generate discs and tracks
    let mut album_structs = Vec::new();
    for album in albums.iter() {
        // gather all discs
        let discs = sqlx::query_scalar!("SELECT DISTINCT disc_no FROM album_track WHERE album_id = ($1) ORDER BY (disc_no)",
            album.album_id)
            .fetch_all(&pool)
            .await?;

        // construct disc_structs
        let mut disc_structs = Vec::new();
        for disc in discs.iter() {
            // gather all tracks on disc
            let tracks = sqlx::query!("SELECT track_no, artist_name, track_name, path FROM track \
                JOIN artist_track ON (track.track_id = artist_track.track_id) \
                JOIN artist ON (artist_track.artist_id = artist.artist_id) \
                JOIN album_track ON (track.track_id = album_track.track_id) \
                WHERE album_id = ($1) AND disc_no = ($2) \
                ORDER BY (track_no)",
                album.album_id, *disc)
                .fetch_all(&pool)
                .await?;

            let track_structs = tracks.iter().map(|track| Track {
                    number: track.track_no.unwrap_or(0),
                    artist: track.artist_name.clone().unwrap_or("Unknown Artist".to_string()),
                    name: track.track_name.clone().unwrap_or("Untitled".to_string()),
                    path: Path::new(&track.path)
                        .strip_prefix(parse_cfg().expect("config.json corrupted or not found").music_directory)
                        .expect("audio file not part of music directory")
                        .to_string_lossy().into_owned(),
                }).collect();

            // construct disc_struct
            let disc_struct = Disc {
                number: (*disc).unwrap_or(0),
                tracks: track_structs,
            };
            disc_structs.push(disc_struct);
        };

        // get album artist
        let album_artist_name = sqlx::query_scalar!("SELECT artist_name FROM album \
            JOIN artist_album ON (album.album_id = artist_album.album_id) \
            JOIN artist ON (artist.artist_id = artist_album.artist_id) \
            WHERE album.album_id = ($1)", album.album_id)
            .fetch_one(&pool)
            .await?;

        // construct album_struct
        let album_struct = Album {
            name: album.album_name.clone().unwrap_or("Unknown Album".to_string()),
            album_artist_name: album_artist_name.unwrap_or("Unknown Artist".to_string()),
            discs: disc_structs,
        };
        album_structs.push(album_struct);
    }

    let json_root = Root {
        albums: album_structs,
    };

    Ok(json_root)
}
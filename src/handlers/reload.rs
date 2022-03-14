use axum::{
    http::StatusCode,
    extract::{Extension}
};
use sqlx::postgres::PgPool;

use blake3::hash;
use id3::TagLike;
use metaflac;

use std::{
    path::Path,
    fs::read,
    error::Error,
};

use crate::utils::{
    parse_cfg,
    internal_error
};
use crate::handlers::{
    Track,
};

// reload_handler for loading database metadata from music directory
pub async fn reload_handler(
    Extension(pool): Extension<PgPool>
) -> Result<(), (StatusCode, String)> {
    load_db(pool).await.map_err(internal_error)
}

// same as above but with wiping the db beforehand
pub async fn hard_reload_handler(
    Extension(pool): Extension<PgPool>
) -> Result<(), (StatusCode, String)> {
    clear_data(pool.clone()).await.map_err(internal_error)?;
    load_db(pool).await.map_err(internal_error)
}

// helper struct
struct TrackInfo {
    track_name: String,
    artist_name: String,
    album_name: String,
    album_artist_name: String,
    path_str: String,
    checksum: Vec<u8>,
}

// given a path to a track, add the track's metadata to the database
async fn add_track_from_path(pool: PgPool, path_str: String) -> Result<(), Box<dyn Error>> {
    // delete the track if the path already exists in the database
    sqlx::query!("DELETE FROM track WHERE path = $1", path_str)
        .execute(&pool)
        .await?;

    // generate track's checksum
    let path = Path::new(&path_str);
    let checksum = hash(read(path)?.as_slice()).as_bytes().to_vec();

    // read relevant tags information
    // supporting only mp3 and flac for now
    let extension = path
        .extension().ok_or(format!("File at {} has no extension", path_str))?
        .to_str().ok_or(format!("File at {} has invalid extension", path_str))?;

    match extension {
        "mp3" => {
            let tag = id3::Tag::read_from_path(path)?;
            let track_info = TrackInfo {
                track_name: tag.title().unwrap_or("").to_string(),
                artist_name: tag.artist().unwrap_or("").to_string(),
                album_name: tag.album().unwrap_or("").to_string(),
                album_artist_name: tag.album_artist().unwrap_or("").to_string(),
                path_str: path_str,
                checksum: checksum,
            };
            add_track_from_info(pool.clone(), track_info).await?;
        },
        "flac" => {
            let tag = metaflac::Tag::read_from_path(path)?;
            match tag.vorbis_comments() {
                Some(x) => {
                    let track_info = TrackInfo {
                        track_name: x.comments["TITLE"][0].clone(),
                        artist_name: x.comments["ARTIST"][0].clone(),
                        album_name: x.comments["ALBUM"][0].clone(),
                        album_artist_name: x.comments["ALBUMARTIST"][0].clone(),
                        path_str: path_str,
                        checksum: checksum,
                    };
                    add_track_from_info(pool.clone(), track_info).await?;
                },
                None => {
                    let track_info = TrackInfo {
                        track_name: String::from(""),
                        artist_name: String::from(""),
                        album_name: String::from(""),
                        album_artist_name: String::from(""),
                        path_str: path_str,
                        checksum: checksum,
                    };
                    add_track_from_info(pool.clone(), track_info).await?;
                }
            }
        },
        _ => Err(format!("File at {0} has unsupported extension {1}", path_str, extension))?,
    };

    Ok(())
}

// given all track's information, add the track to the db
async fn add_track_from_info(pool: PgPool, track_info: TrackInfo) -> Result<(), Box<dyn Error>> {
    // insert into track database
    let track_id = sqlx::query_scalar!("INSERT INTO track (track_name, path, checksum) \
        VALUES ($1, $2, $3) RETURNING track_id",
        track_info.track_name,
        track_info.path_str,
        track_info.checksum)
        .fetch_one(&pool)
        .await?;
    
    // insert artist if artist not in database. there is an unique constraint on artist_name
    let artist_id  = sqlx::query_scalar!("INSERT INTO artist (artist_name) VALUES ($1) \
        ON CONFLICT DO NOTHING RETURNING artist_id",
        track_info.artist_name)
        .fetch_one(&pool)
        .await?;

    // update artisttrack table if not already in database
    // track_id is unique in artist_track table
    // in other words, each track should only have 1 artist tag associated with it
    sqlx::query!("INSERT INTO artist_track (artist_id, track_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        artist_id, track_id)
        .execute(&pool)
        .await?;
        
    // insert album. note that album with same name by different artists should be treated
    // as different albums
    // first we get an album_id where both album_name and artist_id matches what we have
    // insert album artist first in case album artist isn't already in artist
    let album_artist_id = sqlx::query_scalar!("INSERT INTO artist (artist_name) VALUES ($1) \
        ON CONFLICT DO NOTHING RETURNING artist_id",
        track_info.album_artist_name)
        .fetch_one(&pool)
        .await?;
    let album_id_with_same_name = sqlx::query_scalar!("SELECT (album.album_id) FROM album \
        JOIN artist_album ON (album.album_id = artist_album.album_id) \
        WHERE album_name = ($1) AND artist_id = ($2)",
        track_info.album_name,
        album_artist_id)
        .fetch_optional(&pool)
        .await?;
    
    // if album_id exists, it's the same album as our current track's
    // there should be no more than 1 album that matches this query
    let album_id;
    match album_id_with_same_name {
        Some(a) => {
            // using this album id for our next query
            album_id = a;
        },
        None => {
            // no album exists with both the same name and the same album artist
            // so this album should be separate from others
            album_id = sqlx::query_scalar!("INSERT INTO album (album_name) VALUES ($1) RETURNING album_id",
                track_info.album_name)
                .fetch_one(&pool)
                .await?;

            // insert into artist_album table
            sqlx::query!("INSERT INTO artist_album (artist_id, album_id) VALUES ($1, $2)",
                artist_id, album_id)
                .execute(&pool)
                .await?;
        },
    };

    // insert into the album_track table
    sqlx::query!("INSERT INTO album_track (album_id, track_id) VALUES ($1, $2)",
        album_id, track_id)
        .execute(&pool)
        .await?;

    Ok(())
}

// given a track id, remove the track's metadata from the database
async fn delete_track(pool: PgPool, track_id: i32) -> Result<(), Box<dyn Error>> {
    sqlx::query!("DELETE FROM track WHERE track_id = ($1)", track_id)
        .execute(&pool)
        .await?;
    
    Ok(())
}

// load database metadata from path
async fn load_db(pool: PgPool) -> Result<(), Box<dyn Error>> {
    // get music_directory path
    let config = parse_cfg()?;
    let music_directory = Path::new(config.music_directory.as_str());
    
    update_old_metadata(pool.clone()).await?;
    load_new_metadata(pool.clone(), music_directory).await
}

// update old metadata from files that have been changed, or files that have been deleted
async fn update_old_metadata(pool: PgPool) -> Result<(), Box<dyn Error>> {
    // get all paths
    let tracks: Vec<Track> = sqlx::query_as("SELECT * FROM track")
        .fetch_all(&pool)
        .await?;

    // iterate over paths, delete tracks that are invalid and update tracks with differing checksum
    for track in tracks.iter() {
        let path_str = track.path.as_str();
        let path = Path::new(path_str);
        let track_id = track.track_id;
        let checksum = track.checksum.as_slice();

        if !path.exists() {
            // delete metadata if track no longer exists
            delete_track(pool.clone(), track_id).await?;
        } else {
            let new_checksum = hash(read(path)?.as_slice());
            if checksum != new_checksum.as_bytes() {
                // update metadata if track's checksum is different
                delete_track(pool.clone(), track_id).await?;
                add_track_from_path(pool.clone(), path_str.to_string()).await?;
            }
        }
    };

    Ok(())
}

// load new metadata from given music directory path
async fn load_new_metadata(pool: PgPool, music_dir: &Path) -> Result<(), Box<dyn Error>> {
    Ok(())
}

// wipe the database
async fn clear_data(pool: PgPool) -> Result<(), Box<dyn Error>> {
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
        sqlx::query(format!("DELETE FROM {}", table).as_str())
            .execute(&pool)
            .await?;
    };

    Ok(())
}
use std::{
    path::Path,
    fs::read_dir,
};
use axum::{
    http::StatusCode,
    extract::{Extension}
};
use tower::BoxError;
use sqlx::{
    postgres::PgPool,
    types::time::PrimitiveDateTime
};
use id3::TagLike;
use metaflac;
use async_recursion::async_recursion;
use crate::{
    utils::{parse_cfg, internal_error},
    handlers::{DBTrack, RECOGNIZED_EXTENSIONS},
};

// helper struct
#[derive(Debug)]
struct TrackInfo {
    track_name: String,
    artist_name: String,
    album_name: String,
    album_artist_name: String,
    track_number: u32,
    disc_number: u32,
    path_str: String,
    last_modified: PrimitiveDateTime,
}

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

// load database metadata from path
async fn load_db(pool: PgPool) -> Result<(), BoxError> {
    // get music_directory path
    let config = parse_cfg()?;
    let music_directory = config.music_directory;
    
    update_old_metadata(pool.clone()).await?;
    load_new_metadata(pool.clone(), music_directory).await
}

// wipe the database
async fn clear_data(pool: PgPool) -> Result<(), BoxError> {
    // tables to clear from
    let tables = [
        "album_track",
        "artist_album",
        "artist_track",
        "album",
        "artist",
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

// update old metadata from files that have been changed, or files that have been deleted
async fn update_old_metadata(pool: PgPool) -> Result<(), BoxError> {
    // get all paths
    let tracks = sqlx::query_as!(DBTrack, "SELECT * FROM track")
        .fetch_all(&pool)
        .await?;

    // iterate over paths, delete tracks that are invalid and update tracks with differing last modified date
    for track in tracks.iter() {
        let path_str = track.path.as_str();
        let path = Path::new(path_str);
        let track_id = track.track_id;
        let last_modified = track.last_modified;

        if !path.exists() {
            // delete metadata if track no longer exists
            delete_track(pool.clone(), track_id).await?;
        } 
        else {
            let new_modified = PrimitiveDateTime::from(path.metadata()?.modified()?);
            if last_modified < new_modified {
                // update metadata if track's modified time is later
                delete_track(pool.clone(), track_id).await?;
                add_track_from_path(pool.clone(), path_str.to_string()).await?;
            }
        }
    };

    Ok(())
}

// load new metadata from given music directory path
// basically recursively going down the directory then calling add_track_from_path on audio files
#[async_recursion]
async fn load_new_metadata(pool: PgPool, music_dir: String) -> Result<(), BoxError> {
    let path = Path::new(&music_dir);

    let extension = path.extension();
    match extension {
        Some(ext) => {
            // if extension is recognized we add the music track
            if RECOGNIZED_EXTENSIONS.iter().any(|i| *i == ext.to_str().expect("Path isn't a valid UTF-8 string")) {
                add_track_from_path(pool.clone(), music_dir).await?;
            } // otherwise it's an unrecognized extension
        },
        None => {
            // no extension means it can be a directory
            // if it's a directory, run the command on its subdirectory
            if path.is_dir() {
                for entry in read_dir(path)? {
                    let entry = entry?;
                    load_new_metadata(
                        pool.clone(), 
                        entry.path().to_str().ok_or("Path isn't a valid UTF-8 string")?.to_string()
                    ).await?;
                }
            } // otherwise ignore
        },
    };

    Ok(())
}

// given a path to a track, add the track's metadata to the database
// if path's track already in the database, it's assumed the track is correct, so we skip it
async fn add_track_from_path(pool: PgPool, path_str: String) -> Result<(), BoxError> {
    // check if track is already in database
    let already_exists = sqlx::query_scalar!("SELECT (track_id) FROM track WHERE path = ($1)", path_str)
        .fetch_optional(&pool)
        .await?;
    if already_exists.is_some() {
        return Ok(()); // early return
    };

    // get track's last modified date
    let path = Path::new(&path_str);
    let last_modified = PrimitiveDateTime::from(path.metadata()?.modified()?);

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
                track_number: tag.track().unwrap_or(0),
                disc_number: tag.disc().unwrap_or(0),
                path_str: path_str,
                last_modified: last_modified,
            };
            add_track_from_info(pool.clone(), track_info).await?;
        },
        "flac" => {
            let tag = metaflac::Tag::read_from_path(path)?;
            match tag.vorbis_comments() {
                Some(x) => {
                    let track_info = TrackInfo {
                        track_name: x.comments.get("TITLE").unwrap_or(&Vec::new()).get(0).unwrap_or(&"".to_string()).to_string(),
                        artist_name: x.comments.get("ARTIST").unwrap_or(&Vec::new()).get(0).unwrap_or(&"".to_string()).to_string(),
                        album_name: x.comments.get("ALBUM").unwrap_or(&Vec::new()).get(0).unwrap_or(&"".to_string()).to_string(),
                        album_artist_name: x.comments.get("ALBUMARTIST").unwrap_or(&Vec::new()).get(0).unwrap_or(&"".to_string()).to_string(),
                        track_number: x.comments.get("TRACKNUMBER").unwrap_or(&Vec::new()).get(0).unwrap_or(&"0".to_string()).to_string().parse::<u32>()?,
                        disc_number: x.comments.get("DISCNUMBER").unwrap_or(&Vec::new()).get(0).unwrap_or(&"0".to_string()).to_string().parse::<u32>()?,
                        path_str: path_str,
                        last_modified: last_modified,
                    };
                    add_track_from_info(pool.clone(), track_info).await?;
                },
                None => {
                    let track_info = TrackInfo {
                        track_name: String::from(""),
                        artist_name: String::from(""),
                        album_name: String::from(""),
                        album_artist_name: String::from(""),
                        track_number: 0,
                        disc_number: 0,
                        path_str: path_str,
                        last_modified: last_modified,
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
async fn add_track_from_info(pool: PgPool, track_info: TrackInfo) -> Result<(), BoxError> {
    // insert into track database
    let track_id = sqlx::query_scalar!("INSERT INTO track (track_name, path, last_modified) \
        VALUES ($1, $2, $3) RETURNING track_id",
        track_info.track_name,
        track_info.path_str,
        track_info.last_modified)
        .fetch_one(&pool)
        .await?;
    
    // insert artist if artist not in database. there is an unique constraint on artist_name
    let artist_id = insert_artist_from_name(pool.clone(), track_info.artist_name).await?;

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
    let album_artist_id = insert_artist_from_name(pool.clone(), track_info.album_artist_name).await?;
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
                album_artist_id, album_id)
                .execute(&pool)
                .await?;
        },
    };

    // insert into the album_track table
    sqlx::query!("INSERT INTO album_track (album_id, track_id, track_no, disc_no) VALUES ($1, $2, $3, $4)",
        album_id, track_id, track_info.track_number as i32, track_info.disc_number as i32)
        .execute(&pool)
        .await?;

    Ok(())
}

// given an artist name, either insert the artist into the db or return the id of the pre-existing entry
async fn insert_artist_from_name(pool: PgPool, name: String) -> Result<i32, BoxError> {
    let artist_id: i32;
    let artist_id_optional = sqlx::query_scalar!("INSERT INTO artist (artist_name) VALUES ($1) \
        ON CONFLICT DO NOTHING RETURNING artist_id",
        name)
        .fetch_optional(&pool)
        .await?;
    match artist_id_optional {
        Some(id) => artist_id = id,
        None => {
            artist_id = sqlx::query_scalar!("SELECT (artist_id) FROM artist WHERE artist_name = ($1)",
                name)
                .fetch_one(&pool)
                .await?;
        },
    };

    Ok(artist_id)
}

// given a track id, remove the track's metadata from the database
async fn delete_track(pool: PgPool, track_id: i32) -> Result<(), BoxError> {
    // delete the actual track record - should also delete other relations due to ON DELETE CASCADE
    sqlx::query!("DELETE FROM track WHERE track_id = ($1)", track_id)
        .execute(&pool)
        .await?;
    
    Ok(())
}
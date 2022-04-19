use std::{
    path::Path,
    fs::File,
    io::Write,
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
use walkdir::WalkDir;
use blake3;
use anyhow::{Context, Result};

use crate::{
    utils::{SharedState, Config, internal_error},
    handlers::{
        DBTrack, 
        RECOGNIZED_EXTENSIONS, 
        tag_parser::{TrackInfo, parse_tag}
    },
};

// reload_handler for loading database metadata from music directory
pub async fn reload_handler(
    Extension(pool): Extension<PgPool>,
    Extension(config): Extension<Config>,
    Extension(state): Extension<SharedState>
) -> Result<(), (StatusCode, String)> {
    // load data
    load_db(config, pool).await.map_err(internal_error)?;

    // update shared state to mark that the list cache is outdated
    state.write().await.list_cache_outdated = true;
    Ok(())
}

// same as above but with wiping the db beforehand
pub async fn hard_reload_handler(
    Extension(pool): Extension<PgPool>,
    Extension(config): Extension<Config>,
    Extension(state): Extension<SharedState>
) -> Result<(), (StatusCode, String)> {
    // clear data
    clear_data(pool.clone()).await.map_err(internal_error)?;

    // load data
    load_db(config, pool).await.map_err(internal_error)?;

    // update shared state to mark that the list cache is outdated
    state.write().await.list_cache_outdated = true;

    Ok(())
}

// load database metadata from path
async fn load_db(config: Config, pool: PgPool) -> Result<(), BoxError> {    
    update_old_metadata(pool.clone(), &config.art_directory).await?;
    load_new_metadata(pool.clone(), &config.music_directory, &config.art_directory).await
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
async fn update_old_metadata(pool: PgPool, art_dir: &str) -> Result<(), BoxError> {
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
                add_track_from_path(pool.clone(), path_str, art_dir).await?;
            }
        }
    };

    Ok(())
}

// load new metadata from given music directory path
// basically recursively going down the directory then calling add_track_from_path on audio files
async fn load_new_metadata(pool: PgPool, music_dir: &str, art_dir: &str) -> Result<(), BoxError> {
    // silently discards of errors
    for dir in WalkDir::new(music_dir).follow_links(true).into_iter().filter_map(|e| e.ok()) {
        // only care if it has an extension
        let extension = dir.path().extension();
        if let Some(ext) = extension {
            // if extension is recognized we add the music track
            if RECOGNIZED_EXTENSIONS.iter().any(|i| i == &ext) {
                add_track_from_path(
                    pool.clone(), 
                    dir.path().to_str().expect("Path isn't a valid UTF-8 string"), 
                    art_dir
                ).await?;
            };
        };
    };

    Ok(())
}

// given a path to a track, add the track's metadata to the database
// if path's track already in the database, it's assumed the track is correct, so we skip it
async fn add_track_from_path(pool: PgPool, path_str: &str, art_dir: &str) -> Result<(), BoxError> {
    // check if track is already in database
    // procesing 
    let already_exists = sqlx::query_scalar!("SELECT (track_id) FROM track WHERE path = ($1)", path_str)
        .fetch_optional(&pool)
        .await?;
    if already_exists.is_some() {
        return Ok(()); // early return
    };

    // parse track's tag then add based on info
    add_track_from_info(pool.clone(), parse_tag(path_str)?, art_dir).await?;
    Ok(())
}

// given all track's information, add the track to the db
async fn add_track_from_info(pool: PgPool, track_info: TrackInfo, art_dir: &str) -> Result<(), BoxError> {
    // trim null characters from texts
    let clean_track_name = track_info.track_name.trim_matches(char::from(0));
    let clean_artist_name_temp = track_info.artist_name.trim_matches(char::from(0));
    let clean_album_artist_name_temp = track_info.album_artist_name.trim_matches(char::from(0));
    let clean_album_name = track_info.album_name.trim_matches(char::from(0));

    // in the case either artist_name or album_artist_name is empty, go with the other one
    let clean_artist_name;
    let clean_album_artist_name;
    if clean_artist_name_temp == "" {
        clean_artist_name = clean_album_artist_name_temp;
        clean_album_artist_name = clean_album_artist_name_temp;
    } else {
        clean_artist_name = clean_artist_name_temp;
        if clean_album_artist_name_temp == "" {
            clean_album_artist_name = clean_artist_name;
        } else {
            clean_album_artist_name = clean_album_artist_name_temp;
        }
    }

    let track_id = sqlx::query_scalar!("INSERT INTO track (track_name, path, last_modified, length_seconds) \
        VALUES ($1, $2, $3, $4) RETURNING track_id",
        clean_track_name,
        track_info.path_str,
        track_info.last_modified,
        track_info.length_seconds as i32)
        .fetch_one(&pool)
        .await?;

    // insert new art if there is one
    let mut art_id = None;
    if let Some(art) = track_info.art {
        // calculate hash
        let art_hash = blake3::hash(&art);
        let art_hash_bytes = art_hash.as_bytes().to_vec();

        // check if hash in database
        let existing_art_id = sqlx::query_scalar!("SELECT art_id FROM art \
            WHERE hash = ($1)",
            art_hash_bytes)
            .fetch_optional(&pool)
            .await?;

        if existing_art_id.is_some() {
            // if already in database use that one instead
            art_id = Some(existing_art_id.unwrap()); // guarantee to not be none
        } else {
            // else insert new art
            // write to directory
            let new_art_name = art_hash.to_hex().to_string();
            let new_art_directory = format!("{0}/{1}", art_dir, new_art_name);
            let mut file = File::create(&new_art_directory)
                .context(format!("Creation of {} error. Maybe the arts directory in config.json does not exist?", &new_art_directory))?;
            file.write_all(&art)?;

            // insert to db
            art_id = Some(
                sqlx::query_scalar!("INSERT INTO art (hash, path) VALUES ($1, $2) RETURNING art_id",
                art_hash_bytes, &new_art_directory)
                .fetch_one(&pool)
                .await?
            );
        };
    };

    // if there is art connect it with track
    if let Some(curr_art_id) = art_id {
        sqlx::query!("INSERT INTO track_art (track_id, art_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            track_id, curr_art_id)
            .execute(&pool)
            .await?;
    };
    
    // insert artist if artist not in database. there is an unique constraint on artist_name
    let artist_id = insert_artist_from_name(pool.clone(), clean_artist_name).await?;

    // update artisttrack table if not already in database
    // track_id is unique in artist_track table
    // in other words, each track should only have 1 artist tag associated with it
    sqlx::query!("INSERT INTO artist_track (artist_id, track_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        artist_id, track_id)
        .execute(&pool)
        .await?;

    // update artistart table if not already in database
    // similarly, artist_id is unique in artist_art table
    if let Some(curr_art_id) = art_id {
        sqlx::query!("INSERT INTO artist_art (artist_id, art_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            artist_id, curr_art_id)
            .execute(&pool)
            .await?;
    };
        
    // insert album. note that album with same name by different artists should be treated
    // as different albums
    // first we get an album_id where both album_name and artist_id matches what we have
    // insert album artist first in case album artist isn't already in artist
    let album_artist_id = insert_artist_from_name(pool.clone(), clean_album_artist_name).await?;
    let album_id_with_same_name = sqlx::query_scalar!("SELECT (album.album_id) FROM album \
        JOIN artist_album ON (album.album_id = artist_album.album_id) \
        WHERE album_name = ($1) AND artist_id = ($2)",
        clean_album_name,
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
                clean_album_name)
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

    // insert into album_art table
    if let Some(curr_art_id) = art_id {
        sqlx::query!("INSERT INTO album_art (album_id, art_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            album_id, curr_art_id)
            .execute(&pool)
            .await?;
    };

    Ok(())
}

// given an artist name, either insert the artist into the db or return the id of the pre-existing entry
async fn insert_artist_from_name(pool: PgPool, name: &str) -> Result<i32, BoxError> {
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
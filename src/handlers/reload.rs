use std::{
    path::Path,
    collections::HashMap
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

use crate::{
    utils::{SharedState, AlbumCache, Config},
    handlers::{
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
    // start reload only if one isn't already running
    {
        let state_read = state.read().await;

        if state_read.reload_running {
            return Err((StatusCode::SERVICE_UNAVAILABLE, "A Reload task is already running".to_string()));
        }
    }

    // update state to say a reload is running
    state.write().await.reload_running = true;

    // if function did not early return start reloading in separate thread
    let state_clone = state.clone();
    tokio::spawn(async move {
            load_db(pool.clone(), config.clone(), state_clone.clone()).await.expect("Panicked on load_db in reload");
            // update state to say reload finished
            state_clone.write().await.reload_running = false;
        }
    );

    // outdate the cache
    state.write().await.album_cache = AlbumCache {
        list_album_cache_outdated: true,
        list_album_cache: None,
    };
    state.write().await.album_id_cache = HashMap::new();

    Ok(())
}

// same as above but with wiping the db beforehand
pub async fn hard_reload_handler(
    Extension(pool): Extension<PgPool>,
    Extension(config): Extension<Config>,
    Extension(state): Extension<SharedState>
) -> Result<(), (StatusCode, String)> {
    // start reload only if one isn't already running
    {
        let state_read = state.read().await;

        if state_read.reload_running {
            return Err((StatusCode::SERVICE_UNAVAILABLE, "A Reload task is already running".to_string()));
        }
    }

    // update state to say a reload is running
    state.write().await.reload_running = true;

    // if function did not early return start reloading in separate thread
    let state_clone = state.clone();
    tokio::spawn(async move {
            // silently ignore error here for now
            clear_data(pool.clone(), state_clone.clone()).await.expect("Panicked on clear_data in hard_reload");
            load_db(pool.clone(), config.clone(), state_clone.clone()).await.expect("Panicked on load_data in hard_reload");

            // update state to say reload finished
            state_clone.write().await.reload_running = false;
        }
    );

    // recreate cache
    state.write().await.album_cache = AlbumCache {
        list_album_cache_outdated: true,
        list_album_cache: None,
    };
    state.write().await.album_id_cache = HashMap::new();

    Ok(())
}

// wipe the database
async fn clear_data(pool: PgPool, state: SharedState) -> Result<(), BoxError> {
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

    // recreate cache
    state.write().await.album_cache = AlbumCache {
        list_album_cache_outdated: true,
        list_album_cache: None,
    };
    state.write().await.album_id_cache = HashMap::new();

    Ok(())
}

// load database metadata from path
async fn load_db(pool: PgPool, config: Config, state: SharedState) -> Result<(), BoxError> {    
    update_old_metadata(&pool, &config, &state).await?;
    load_new_metadata(&pool, &config, &state).await?;

    // recreate cache
    state.write().await.album_cache = AlbumCache {
        list_album_cache_outdated: true,
        list_album_cache: None,
    };
    state.write().await.album_id_cache = HashMap::new();

    Ok(())
}

// update old metadata from files that have been changed, or files that have been deleted
async fn update_old_metadata(pool: &PgPool, config: &Config, state: &SharedState) -> Result<(), BoxError> {
    // struct for interfacing with the database
    struct DBTrack {
        track_id: i32,
        path: String,
        last_modified: PrimitiveDateTime,
    }

    // get all paths
    let tracks = sqlx::query_as!(DBTrack, "SELECT path, track_id, last_modified FROM track")
        .fetch_all(pool)
        .await?;

    // iterate over paths, delete tracks that are invalid and update tracks with differing last modified date
    for track in tracks.iter() {
        let path = Path::new(&track.path);
        let path_full = Path::new(&config.music_directory).join(path);
        let track_id = track.track_id;
        let last_modified = track.last_modified;

        if !path_full.exists() {
            // delete metadata if track no longer exists
            delete_track(pool, track_id).await?;
        } 
        else {
            let new_modified = PrimitiveDateTime::from(path_full.metadata()?.modified()?);
            if last_modified < new_modified {
                // update metadata if track's modified time is later
                delete_track(pool, track_id).await?;
                add_track_from_path(pool, config, state, path).await?;
            }
        }
    };

    // recreate cache
    state.write().await.album_cache = AlbumCache {
        list_album_cache_outdated: true,
        list_album_cache: None,
    };
    state.write().await.album_id_cache = HashMap::new();

    Ok(())
}

// load new metadata from given music directory path
// basically recursively going down the directory then calling add_track_from_path on audio files
async fn load_new_metadata(pool: &PgPool, config: &Config, state: &SharedState) -> Result<(), BoxError> {
    // silently discards of errors
    for dir in WalkDir::new(&config.music_directory).follow_links(true).into_iter().filter_map(|e| e.ok()) {
        // only care if it has an extension
        let extension = dir.path().extension();
        if let Some(ext) = extension {
            // if extension is recognized we add the music track
            if RECOGNIZED_EXTENSIONS.iter().any(|i| i == &ext) {
                add_track_from_path(
                    pool, config, state,
                    dir.path().strip_prefix(&config.music_directory)?,
                ).await?;
            };
        };
    };

    // recreate cache
    state.write().await.album_cache = AlbumCache {
        list_album_cache_outdated: true,
        list_album_cache: None,
    };
    state.write().await.album_id_cache = HashMap::new();

    Ok(())
}

// given a path to a track, add the track's metadata to the database
// if path's track already in the database, it's assumed the track is correct, so we skip it
async fn add_track_from_path(pool: &PgPool, config: &Config, state: &SharedState, path: &Path) -> Result<(), BoxError> {
    // check if track is already in database
    // procesing 
    let already_exists = sqlx::query_scalar!("SELECT (track_id) FROM track WHERE path = ($1)",
        &path.to_string_lossy())
        .fetch_optional(pool)
        .await?;
    if already_exists.is_some() {
        return Ok(()); // early return
    };

    // parse track's tag then add based on info
    add_track_from_info(pool, parse_tag(pool, config, path).await?).await?;

    // recreate cache
    state.write().await.album_cache = AlbumCache {
        list_album_cache_outdated: true,
        list_album_cache: None,
    };
    state.write().await.album_id_cache = HashMap::new();
    Ok(())
}

// given all track's information, add the track to the db
async fn add_track_from_info(pool: &PgPool, track_info: TrackInfo) -> Result<(), BoxError> {
    // trim null characters from texts
    let clean_track_name = &(track_info.track_name.replace(char::from(0), ""));
    let clean_artist_name_temp = &(track_info.artist_name.replace(char::from(0), ""));
    let clean_album_artist_name_temp = &(track_info.album_artist_name.replace(char::from(0), ""));
    let clean_album_name = &(track_info.album_name.replace(char::from(0), ""));

    // in the case either artist_name or album_artist_name is empty, go with the other one
    let clean_artist_name;
    let clean_album_artist_name;
    if clean_artist_name_temp == "Unknown Artist" {
        clean_artist_name = clean_album_artist_name_temp;
        clean_album_artist_name = clean_album_artist_name_temp;
    } else {
        clean_artist_name = clean_artist_name_temp;
        if clean_album_artist_name_temp == "Unknown Artist" {
            clean_album_artist_name = clean_artist_name;
        } else {
            clean_album_artist_name = clean_album_artist_name_temp;
        }
    }

    // insert track
    let track_id = sqlx::query_scalar!("INSERT INTO track (track_name, path, last_modified, length_seconds) \
        VALUES ($1, $2, $3, $4) RETURNING track_id",
        clean_track_name,
        track_info.path_str,
        track_info.last_modified,
        track_info.length_seconds as i32)
        .fetch_one(pool)
        .await?;

    // connect art with track if track has art
    if let Some(curr_art_id) = track_info.art_id {
        sqlx::query!("INSERT INTO track_art (track_id, art_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            track_id, curr_art_id)
            .execute(pool)
            .await?;
    };
    
    // insert artist if artist not in database. there is an unique constraint on artist_name
    let artist_id = insert_artist_from_name(pool, clean_artist_name).await?;

    // update artisttrack table if not already in database
    // track_id is unique in artist_track table
    // in other words, each track should only have 1 artist tag associated with it
    sqlx::query!("INSERT INTO artist_track (artist_id, track_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        artist_id, track_id)
        .execute(pool)
        .await?;

    // update artistart table if not already in database
    // similarly, artist_id is unique in artist_art table
    if let Some(curr_art_id) = track_info.art_id {
        sqlx::query!("INSERT INTO artist_art (artist_id, art_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            artist_id, curr_art_id)
            .execute(pool)
            .await?;
    };
        
    // insert album. note that album with same name by different artists should be treated
    // as different albums
    // first we get an album_id where both album_name and artist_id matches what we have
    // insert album artist first in case album artist isn't already in artist
    let album_artist_id = insert_artist_from_name(pool, clean_album_artist_name).await?;
    let album_id_with_same_name = sqlx::query_scalar!("SELECT (album.album_id) FROM album \
        JOIN artist_album ON (album.album_id = artist_album.album_id) \
        WHERE album_name = ($1) AND artist_id = ($2)",
        clean_album_name,
        album_artist_id)
        .fetch_optional(pool)
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
                .fetch_one(pool)
                .await?;

            // insert into artist_album table
            sqlx::query!("INSERT INTO artist_album (artist_id, album_id) VALUES ($1, $2)",
                album_artist_id, album_id)
                .execute(pool)
                .await?;
        },
    };

    // insert into the album_track table
    sqlx::query!("INSERT INTO album_track (album_id, track_id, track_no, disc_no) VALUES ($1, $2, $3, $4)",
        album_id, track_id, track_info.track_number as i32, track_info.disc_number as i32)
        .execute(pool)
        .await?;

    // insert into album_art table
    if let Some(curr_art_id) = track_info.art_id {
        sqlx::query!("INSERT INTO album_art (album_id, art_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            album_id, curr_art_id)
            .execute(pool)
            .await?;
    };

    Ok(())
}

// given an artist name, either insert the artist into the db or return the id of the pre-existing entry
async fn insert_artist_from_name(pool: &PgPool, name: &str) -> Result<i32, BoxError> {
    let artist_id: i32;
    let artist_id_optional = sqlx::query_scalar!("INSERT INTO artist (artist_name) VALUES ($1) \
        ON CONFLICT DO NOTHING RETURNING artist_id",
        name)
        .fetch_optional(pool)
        .await?;
    match artist_id_optional {
        Some(id) => artist_id = id,
        None => {
            artist_id = sqlx::query_scalar!("SELECT (artist_id) FROM artist WHERE artist_name = ($1)",
                name)
                .fetch_one(pool)
                .await?;
        },
    };

    Ok(artist_id)
}

// given a track id, remove the track's metadata from the database
async fn delete_track(pool: &PgPool, track_id: i32) -> Result<(), BoxError> {
    // delete the actual track record - should also delete other relations due to ON DELETE CASCADE
    sqlx::query!("DELETE FROM track WHERE track_id = ($1)", track_id)
        .execute(pool)
        .await?;
    
    Ok(())
}
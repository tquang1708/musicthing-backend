use axum::{
    http::StatusCode,
    response::{Json},
    extract::{Extension},
};
use axum_macros::debug_handler;
use tower::BoxError;
use sqlx::postgres::PgPool;
use crate::{
    utils::{
        internal_error,
        SharedState,
        ListRoot, ListAlbumDeprecating, ListDisc, ListTrack,
        ListAlbum},
};

// if list_cache is outdated based on state, calculate new list_cache and update state
// list_cache being a listing of the files available on the database
#[debug_handler]
pub async fn list_handler(
    Extension(pool): Extension<PgPool>,
    Extension(state): Extension<SharedState>,
) -> Result<Json<Option<ListRoot>>, (StatusCode, String)> {
    {
        let state_read = state.read().await;

        // if list_cache is not outdated return the cached list_cache
        // in addition, list_cache is ok to unwrap here since None is a valid value for an empty db
        if !state_read.list_cache_outdated {
            return Ok(Json(state_read.list_cache.clone())); // only cloning the list_cache
        }
    } // separating read lock in different scope to prevent it blocking writes

    // if function did not early return in previous step this means list cache is outdated
    // update state with new list cache
    let new_list_cache = generate_root(&pool).await.map_err(internal_error)?;
    state.write().await.list_cache = new_list_cache.clone();

    // change list_cache_outdated to false to indicate list_cache has been updated
    state.write().await.list_cache_outdated = false;

    // return the appropriate json
    Ok(Json(new_list_cache))
}

async fn generate_root(pool: &PgPool) -> Result<Option<ListRoot>, BoxError> {
    // struct for interfacing with the db
    struct DBAlbum {
        album_id: i32,
        album_name: String,
    }

    // gather all albums with tracks sorted alphabetically
    let albums = sqlx::query_as!(DBAlbum, "SELECT DISTINCT album.album_id, album_name FROM album \
        JOIN album_track ON (album.album_id = album_track.album_id)
        ORDER BY (album_name)")
        .fetch_all(pool)
        .await?;
    
    // iterate over albums to generate discs and tracks if there's anything in albums
    if albums.len() > 0 {
        let mut album_structs = Vec::new();
        for album in albums.iter() {
            // gather all discs
            let discs = sqlx::query_scalar!("SELECT DISTINCT disc_no FROM album_track WHERE album_id = ($1) ORDER BY (disc_no)",
                album.album_id)
                .fetch_all(pool)
                .await?;

            // construct disc_structs
            let mut disc_structs = Vec::new();
            for disc in discs.iter() {
                // gather all tracks on disc
                let tracks = sqlx::query!("SELECT track_no, artist_name, track_name, path, length_seconds FROM track \
                    JOIN artist_track ON (track.track_id = artist_track.track_id) \
                    JOIN artist ON (artist_track.artist_id = artist.artist_id) \
                    JOIN album_track ON (track.track_id = album_track.track_id) \
                    WHERE album_id = ($1) AND disc_no = ($2) \
                    ORDER BY (track_no)",
                    album.album_id, *disc)
                    .fetch_all(pool)
                    .await?;

                let track_structs = tracks.iter().map(|track| ListTrack {
                        number: track.track_no.unwrap_or(0),
                        artist: track.artist_name.clone(),
                        name: track.track_name.clone(),
                        path: track.path.clone(),
                        length_seconds: track.length_seconds,
                    }).collect();

                // construct disc_struct
                let disc_struct = ListDisc {
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
                .fetch_one(pool)
                .await?;

            // get album art path
            let album_art_path_optional = sqlx::query_scalar!("SELECT path FROM art \
                JOIN album_art ON (album_art.art_id = art.art_id) \
                JOIN album ON (album.album_id = album_art.album_id) \
                WHERE album.album_id = ($1)", album.album_id)
                .fetch_optional(pool)
                .await?;
            
            let album_art_path_actual;
            if let Some(album_art_path) = album_art_path_optional {
                album_art_path_actual = album_art_path;
            } else {
                album_art_path_actual = "".to_string();
            }

            // construct album_struct
            let album_struct = ListAlbumDeprecating {
                name: album.album_name.clone(),
                album_artist_name: album_artist_name,
                album_art_path: album_art_path_actual,
                discs: disc_structs,
            };
            album_structs.push(album_struct);
        };

        let json_root = ListRoot {
            albums: album_structs,
        };

        Ok(Some(json_root))
    } else {
        // otherwise return nothing
        Ok(None)
    }
}

pub async fn list_album_handler(
    Extension(pool): Extension<PgPool>,
    Extension(state): Extension<SharedState>,
) -> Result<Json<Option<Vec<ListAlbum>>>, (StatusCode, String)> {
    {
        let state_read = state.read().await;

        if !state_read.list_album_cache_outdated {
            return Ok(Json(state_read.list_album_cache.clone())); // only cloning the list_album_cache
        }
    }

    // if function did not early return in previous step this means list cache is outdated
    // update state with new list cache
    let new_list_album_cache = list_album(&pool).await.map_err(internal_error)?;
    state.write().await.list_album_cache = new_list_album_cache.clone();
    state.write().await.list_album_cache_outdated = false;

    // return the appropriate json
    Ok(Json(new_list_album_cache))
}

async fn list_album(pool: &PgPool) -> Result<Option<Vec<ListAlbum>>, BoxError> {
    // query all relevant information
    let albums = sqlx::query_as!(ListAlbum, "SELECT DISTINCT 
        album.album_id as id, 
        album_name as name, 
        artist_name, 
        path as art_path FROM album \
        JOIN artist_album ON (album.album_id = artist_album.album_id) \
        JOIN artist ON (artist.artist_id = artist_album.artist_id) \
        JOIN album_art ON (album_art.album_id = album.album_id) \
        JOIN art ON (album_art.art_id = art.art_id)
        ORDER BY (album_name)")
        .fetch_all(pool)
        .await?;

    // just return
    Ok(Some(albums))
}
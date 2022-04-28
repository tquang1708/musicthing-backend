use axum::{
    http::StatusCode,
    response::{Json},
    extract::{Extension, Path},
};
use tower::BoxError;
use sqlx::postgres::PgPool;
use std::collections::HashMap;
use crate::{
    utils::{
        internal_error,
        SharedState, AlbumCache,
        ListAlbum, ListAlbumID, ListDisc, ListTrack,
    },
};

pub async fn list_albums_handler(
    Extension(pool): Extension<PgPool>,
    Extension(state): Extension<SharedState>,
) -> Result<Json<Option<Vec<ListAlbum>>>, (StatusCode, String)> {
    {
        let cache_read = state.read().await;

        if !cache_read.album_cache.list_album_cache_outdated {
            return Ok(Json(cache_read.album_cache.list_album_cache.clone())); // only cloning the list_album_cache
        }
    }

    // if function did not early return in previous step this means list cache is outdated
    // update state with new list cache
    let new_list_album_cache = list_albums(&pool).await.map_err(internal_error)?;
    state.write().await.album_cache = AlbumCache {
        list_album_cache_outdated: false,
        list_album_cache: new_list_album_cache.clone(),
    };

    // return the appropriate json
    Ok(Json(new_list_album_cache))
}

async fn list_albums(pool: &PgPool) -> Result<Option<Vec<ListAlbum>>, BoxError> {
    // query all relevant information
    let albums = sqlx::query_as!(ListAlbum, r#"SELECT DISTINCT
        album.album_id as id, 
        album_name as name, 
        artist_name, 
        path as "art_path?" FROM album
        JOIN artist_album ON (album.album_id = artist_album.album_id)
        JOIN artist ON (artist.artist_id = artist_album.artist_id)
        LEFT OUTER JOIN album_art ON (album_art.album_id = album.album_id)
        LEFT OUTER JOIN art ON (album_art.art_id = art.art_id)
        ORDER BY (album_name)"#)
        .fetch_all(pool)
        .await?;

    // just return
    Ok(Some(albums))
}

pub async fn list_album_id_handler(
    Extension(pool): Extension<PgPool>,
    Extension(state): Extension<SharedState>,
    Path(params): Path<HashMap<String, String>>,
) -> Result<Json<Option<ListAlbumID>>, (StatusCode, String)> {
    // obtain requested album_id
    let id = params.get("id").expect("key id not found in parameter");

    // check if cache exists
    {
        let cache_read = state.read().await;

        if let Some(album_id_cache) = cache_read.album_id_cache.get(id) {
            return Ok(Json(Some(album_id_cache.clone())));
        }
    }

    // otherwise write new cache
    let new_list_album_id_cache = list_album_id(&pool, &id).await.map_err(internal_error)?;
    if let Some(ref actual_album_id_cache) = new_list_album_id_cache {
        state.write().await.album_id_cache.insert(id.to_string(), actual_album_id_cache.clone());
    };

    // return the appropriate json
    Ok(Json(new_list_album_id_cache))
}

// if list_cache is outdated based on state, calculate new list_cache and update state
// list_cache being a listing of the files available on the database
async fn list_album_id(pool: &PgPool, id: &str) -> Result<Option<ListAlbumID>, BoxError> {
    // struct for interfacing
    struct DBAlbum {
        id: i32,
        name: String,
        album_artist_name: String,
        art_path: Option<String>,
    }

    // return early if parsing fails
    let id_parse = id.parse::<i32>();
    if id_parse.is_err() {
        return Ok(None);
    };
    let id_int = id_parse.unwrap();

    // get our album
    let album = sqlx::query_as!(DBAlbum, r#"SELECT DISTINCT 
        album.album_id as id, 
        album_name as name, 
        artist_name as album_artist_name, 
        path as "art_path?" FROM album
        JOIN artist_album ON (album.album_id = artist_album.album_id)
        JOIN artist ON (artist.artist_id = artist_album.artist_id)
        LEFT OUTER JOIN album_art ON (album_art.album_id = album.album_id)
        LEFT OUTER JOIN art ON (album_art.art_id = art.art_id)
        WHERE album.album_id = ($1)"#, id_int)
        .fetch_optional(pool)
        .await?;

    // if there is an album
    if let Some(alb) = album {
        // gather all discs
        let discs = sqlx::query_scalar!("SELECT DISTINCT disc_no FROM album_track 
            WHERE album_id = ($1) ORDER BY (disc_no)", id_int)
        .fetch_all(pool)
        .await?;

        // construct disc_struct
        let mut disc_structs: Vec<ListDisc> = Vec::new();
        for disc in discs {
            // gather all tracks on disc
            let tracks = sqlx::query!(r#"SELECT track.track_id as track_id, track_no, artist_name, track_name, track.path as path, art.path as "art_path?", length_seconds FROM track
                JOIN artist_track ON (track.track_id = artist_track.track_id)
                JOIN artist ON (artist_track.artist_id = artist.artist_id)
                JOIN album_track ON (track.track_id = album_track.track_id)
                LEFT OUTER JOIN track_art ON (track_art.track_id = track.track_id)
                LEFT OUTER JOIN art ON (track_art.art_id = art.art_id)
                WHERE album_id = ($1) AND disc_no = ($2)
                ORDER BY (track_no)"#,
                id_int, disc)
                .fetch_all(pool)
                .await?;

            let track_structs = tracks.iter().map(|track| ListTrack {
                    id: track.track_id,
                    number: track.track_no.unwrap_or(0),
                    artist: track.artist_name.clone(),
                    name: track.track_name.clone(),
                    path: track.path.clone(),
                    art_path: track.art_path.clone(),
                    length_seconds: track.length_seconds,
                }).collect();

            // construct disc_struct
            disc_structs.push(ListDisc {
                number: disc.unwrap_or(0),
                tracks: track_structs,
            });
        };

        Ok(Some(ListAlbumID {
            id: alb.id,
            name: alb.name,
            album_artist_name: alb.album_artist_name,
            art_path: alb.art_path,
            discs: disc_structs,
        }))
    } else {
        Ok(None)
    }
}
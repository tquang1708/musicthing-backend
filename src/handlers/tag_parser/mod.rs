use std::{
    path::{Path, PathBuf},
    time::Duration,
    fs::{File, read, read_dir},
    io::Write,
};
use sqlx::{
    types::time::PrimitiveDateTime,
    PgPool
};
use blake3;
use anyhow::{Context, Result};
use tower::BoxError;

use id3::TagLike;
use metaflac;
use mp3_duration;
use crate::handlers::IMAGE_EXTENSIONS;

// helper struct
#[derive(Debug)]
pub struct TrackInfo {
    pub track_name: String,
    pub artist_name: String,
    pub album_name: String,
    pub album_artist_name: String,
    pub track_number: u32,
    pub disc_number: u32,
    pub length_seconds: u64,
    pub art_id: Option<i32>,
    pub path_str: String,
    pub last_modified: PrimitiveDateTime,
}

pub async fn parse_tag(path_str: &str, pool: &PgPool, art_dir: &str) -> Result<TrackInfo, BoxError> {
    // get track's last modified date
    let path = Path::new(path_str);
    let last_modified = PrimitiveDateTime::from(path.metadata()?.modified()?);

    // read relevant tags information
    // supporting only mp3 and flac for now
    let extension = path
        .extension().ok_or(format!("File at {} has no extension", path_str))?
        .to_str().ok_or(format!("File at {} has invalid extension", path_str))?;

    match extension {
        "mp3" => {
            parse_mp3(path, path_str, last_modified, pool, art_dir).await
        },
        "flac" => {
            parse_flac(path, path_str, last_modified, pool, art_dir).await
        },
        _ => {
            Err(format!("File at {0} has unsupported extension {1}", path_str, extension))?
        },
    }
}

async fn parse_mp3(path: &Path, path_str: &str, last_modified: PrimitiveDateTime, pool: &PgPool, art_dir: &str) -> Result<TrackInfo, BoxError> {
    // get tag
    let tag_optional = id3::Tag::read_from_path(path).ok();

    // get length
    let track_length = mp3_duration::from_path(path).unwrap_or(Duration::new(0, 0)).as_secs();

    // get image file in parent dir
    let picture = get_picture_in_dir(path)?;

    Ok(
        match tag_optional {
            Some(tag) => {
                // get picture
                let mut art_id = None;
                if let Some(picture_dir) = picture {
                    art_id = Some(get_art_id(&read(picture_dir)?, pool, art_dir).await?);
                } else {
                    let mut pictures_iter = tag.pictures();
                    loop {
                        if let Some(picture_curr) = pictures_iter.next() {
                            if picture_curr.picture_type == id3::frame::PictureType::CoverFront {
                                // if cover front we can stop
                                art_id = Some(get_art_id(&picture_curr.data, pool, art_dir).await?);
                                break;
                            }
                        } else {
                            break;
                        }
                    };
                };

                TrackInfo {
                    track_name: tag.title().unwrap_or("Untitled").to_string(),
                    artist_name: tag.artist().unwrap_or("Unknown Artist").to_string(),
                    album_name: tag.album().unwrap_or("Unknown Album").to_string(),
                    album_artist_name: tag.album_artist().unwrap_or("Unknown Artist").to_string(),
                    track_number: tag.track().unwrap_or(0),
                    disc_number: tag.disc().unwrap_or(0),
                    length_seconds: track_length,
                    art_id: art_id,
                    path_str: path_str.to_string(),
                    last_modified: last_modified,
                }
            },
            None => {
                TrackInfo {
                    track_name: String::from("Untitled"),
                    artist_name: String::from("Unknown Artist"),
                    album_name: String::from("Unknown Album"),
                    album_artist_name: String::from("Unknown Artist"),
                    track_number: 0,
                    disc_number: 0,
                    length_seconds: track_length,
                    art_id: None,
                    path_str: path_str.to_string(),
                    last_modified: last_modified,
                }
            }             
        }           
    )
}

async fn parse_flac(path: &Path, path_str: &str, last_modified: PrimitiveDateTime, pool: &PgPool, art_dir: &str) -> Result<TrackInfo, BoxError> {
    // get image file in parent dir
    let picture = get_picture_in_dir(path)?;

    // get tag
    let tag_optional = metaflac::Tag::read_from_path(path).ok();
    
    Ok(
        match tag_optional {
            Some(tag) => {
                // get length
                let track_length;
                if let Some(streaminfo) = tag.get_streaminfo() {
                    track_length = streaminfo.total_samples / streaminfo.sample_rate as u64;
                } else {
                    track_length = 0;
                };

                // get pictures
                // exact same interface as id3 apparently for pictures
                let mut art_id = None;
                if let Some(picture_dir) = picture {
                    art_id = Some(get_art_id(&read(picture_dir)?, pool, art_dir).await?);
                } else {
                    let mut pictures_iter = tag.pictures();
                    loop {
                        if let Some(picture_curr) = pictures_iter.next() {
                            if picture_curr.picture_type == metaflac::block::PictureType::CoverFront {
                                // if cover front we can stop
                                art_id = Some(get_art_id(&picture_curr.data, pool, art_dir).await?);
                                break;
                            }
                        } else {
                            break;
                        }
                    };
                };

                match tag.vorbis_comments() {
                    Some(comment) => {
                        TrackInfo {
                            track_name: comment.title().unwrap_or(&vec!["Untitled".to_string()]).join(", "),
                            artist_name: comment.artist().unwrap_or(&vec!["Unknown Artist".to_string()]).join(", "),
                            album_name: comment.album().unwrap_or(&vec!["Unknown Album".to_string()]).join(", "),
                            album_artist_name: comment.album_artist().unwrap_or(&vec!["Unknown Artist".to_string()]).join(", "),
                            track_number: comment.track().unwrap_or(0),
                            disc_number: comment.comments.get("DISCNUMBER").unwrap_or(&Vec::new()).get(0).unwrap_or(&"0".to_string()).to_string().parse::<u32>().unwrap_or(0),
                            length_seconds: track_length,
                            art_id: art_id,
                            path_str: path_str.to_string(),
                            last_modified: last_modified,
                        }
                    },
                    None => {
                        TrackInfo {
                            track_name: String::from("Untitled"),
                            artist_name: String::from("Unknown Artist"),
                            album_name: String::from("Unknown Album"),
                            album_artist_name: String::from("Unknown Artist"),
                            track_number: 0,
                            disc_number: 0,
                            length_seconds: track_length,
                            art_id: art_id,
                            path_str: path_str.to_string(),
                            last_modified: last_modified,
                        }   
                    }
                }
            },
            None => {
                TrackInfo {
                    track_name: String::from("Untitled"),
                    artist_name: String::from("Unknown Artist"),
                    album_name: String::from("Unknown Album"),
                    album_artist_name: String::from("Unknown Artist"),
                    track_number: 0,
                    disc_number: 0,
                    length_seconds: 0,
                    art_id: None,
                    path_str: path_str.to_string(),
                    last_modified: last_modified,
                }
            }
        }
    )
}

// get an image file in the current directory
fn get_picture_in_dir(path: &Path) -> Result<Option<PathBuf>, BoxError> {
    // get parent
    let parent = path.parent().unwrap(); // since this is called in parse_mp3/flac, this is guaranteed to not be a directory

    // find first image
    for dir in read_dir(parent)? {
        let path = dir?.path();
        if let Some(ext) = path.extension() {
            if IMAGE_EXTENSIONS.iter().any(|i| i == &ext) {
                return Ok(Some(path.to_path_buf()));
            }
        }
    }

    // else return none
    Ok(None)
}

// check if picture's already in the database
// insert new art if there isn't one
async fn get_art_id(picture_data: &[u8], pool: &PgPool, art_dir: &str) -> Result<i32, BoxError> {
    let art_id: i32;

    // calculate hash
    let art_hash = blake3::hash(picture_data);
    let art_hash_bytes = art_hash.as_bytes().to_vec();

    // check if hash in database
    let existing_art_id = sqlx::query_scalar!("SELECT art_id FROM art \
        WHERE hash = ($1)",
        art_hash_bytes)
        .fetch_optional(pool)
        .await?;

    if existing_art_id.is_some() {
        // if already in database use that one instead
        art_id = existing_art_id.unwrap(); // guarantee to not be none
    } else {
        // else insert new art
        // write to directory
        let new_art_name = art_hash.to_hex().to_string();
        let new_art_directory = format!("{0}/{1}", art_dir, new_art_name);
        let mut file = File::create(&new_art_directory)
            .context(format!("Creation of {} error. Maybe the arts directory in config.json does not exist?", &new_art_directory))?;
        file.write_all(picture_data)?;

        // insert to db
        art_id = sqlx::query_scalar!("INSERT INTO art (hash, path) VALUES ($1, $2) RETURNING art_id",
            art_hash_bytes, &new_art_directory)
            .fetch_one(pool)
            .await?;
    };

    Ok(art_id)
}
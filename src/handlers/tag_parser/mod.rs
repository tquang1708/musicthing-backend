use std::{
    path::{Path, PathBuf},
    time::Duration,
    fs::{File, DirEntry, read, read_dir},
    io::Write,
};
use itertools::Itertools;
use sqlx::{
    types::time::PrimitiveDateTime,
    PgPool
};
use blake3;
use anyhow::{Context, Result};
use tower::BoxError;

use id3::TagLike;
use mp3_duration;
use metaflac;
use mp4ameta;
use crate::{
    handlers::IMAGE_EXTENSIONS,
    utils::Config
};

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

pub async fn parse_tag(pool: &PgPool, config: &Config, path: &Path) -> Result<TrackInfo, BoxError> {
    // get track's last modified date
    let path_full = Path::new(&config.music_directory).join(path);
    let last_modified = PrimitiveDateTime::from(path_full.metadata()?.modified()?);

    // read relevant tags information
    // supporting only mp3 and flac for now
    let extension = path
        .extension()
        .ok_or(format!("File at {} has no extension", path.to_string_lossy()))?
        .to_str();

    match extension {
        Some("mp3") => {
            parse_mp3(pool, path, &path_full, last_modified, &config.art_directory).await
        },
        Some("flac") => {
            parse_flac(pool, path, &path_full, last_modified, &config.art_directory).await
        },
        Some("m4a") => {
            parse_m4a(pool, path, &path_full, last_modified, &config.art_directory).await
        },
        _ => {
            Err(format!("File at {0} has unsupported extension", path.to_string_lossy()))?
        },
    }
}

async fn parse_mp3(
    pool: &PgPool, 
    path: &Path, 
    path_full: &Path,
    last_modified: PrimitiveDateTime,
    art_dir: &str
) -> Result<TrackInfo, BoxError> {
    // get tag
    let tag_optional = id3::Tag::read_from_path(path_full).ok();

    // get length
    let track_length = mp3_duration::from_path(path_full).unwrap_or(Duration::new(0, 0)).as_secs();

    // get path
    let path_str = path.to_string_lossy().to_string();

    Ok(
        match tag_optional {
            Some(tag) => {
                // get picture
                let mut art_id = None;
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

                // in case the track has no embedded cover find it in dir
                if art_id.is_none() {
                    // get image file in parent dir
                    let picture = get_picture_in_dir(path_full)?;
                    if let Some(picture_dir) = picture {
                        art_id = Some(get_art_id(&read(picture_dir)?, pool, art_dir).await?);
                    };
                };

                TrackInfo {
                    track_name: tag.title().unwrap_or(&path_str).to_string(),
                    artist_name: tag.artist().unwrap_or("Unknown Artist").to_string(),
                    album_name: tag.album().unwrap_or("Unknown Album").to_string(),
                    album_artist_name: tag.album_artist().unwrap_or("Unknown Artist").to_string(),
                    track_number: tag.track().unwrap_or(0),
                    disc_number: tag.disc().unwrap_or(0),
                    length_seconds: track_length,
                    art_id: art_id,
                    path_str: path.to_string_lossy().to_string(),
                    last_modified: last_modified,
                }
            },
            None => {
                TrackInfo {
                    track_name: path_str,
                    artist_name: String::from("Unknown Artist"),
                    album_name: String::from("Unknown Album"),
                    album_artist_name: String::from("Unknown Artist"),
                    track_number: 0,
                    disc_number: 0,
                    length_seconds: track_length,
                    art_id: None,
                    path_str: path.to_string_lossy().to_string(),
                    last_modified: last_modified,
                }
            }             
        }           
    )
}

async fn parse_flac(
    pool: &PgPool, 
    path: &Path, 
    path_full: &Path,
    last_modified: PrimitiveDateTime,
    art_dir: &str
) -> Result<TrackInfo, BoxError> {
    // get tag
    let tag_optional = metaflac::Tag::read_from_path(path_full).ok();

    // get path
    let path_str = path.to_string_lossy().to_string();
    
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

                // get picture
                // exact same interface as id3 apparently for pictures
                let mut art_id = None;
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

                // in case the track has no embedded cover find it in dir
                if art_id.is_none() {
                    // get image file in parent dir
                    let picture = get_picture_in_dir(path_full)?;
                    if let Some(picture_dir) = picture {
                        art_id = Some(get_art_id(&read(picture_dir)?, pool, art_dir).await?);
                    };
                };

                match tag.vorbis_comments() {
                    Some(comment) => {
                        TrackInfo {
                            track_name: comment.title().unwrap_or(&vec![path_str]).join(", "),
                            artist_name: comment.artist().unwrap_or(&vec!["Unknown Artist".to_string()]).join(", "),
                            album_name: comment.album().unwrap_or(&vec!["Unknown Album".to_string()]).join(", "),
                            album_artist_name: comment.album_artist().unwrap_or(&vec!["Unknown Artist".to_string()]).join(", "),
                            track_number: comment.track().unwrap_or(0),
                            disc_number: comment.comments.get("DISCNUMBER").unwrap_or(&Vec::new()).get(0).unwrap_or(&"0".to_string()).to_string().parse::<u32>().unwrap_or(0),
                            length_seconds: track_length,
                            art_id: art_id,
                            path_str: path.to_string_lossy().to_string(),
                            last_modified: last_modified,
                        }
                    },
                    None => {
                        TrackInfo {
                            track_name: path_str,
                            artist_name: String::from("Unknown Artist"),
                            album_name: String::from("Unknown Album"),
                            album_artist_name: String::from("Unknown Artist"),
                            track_number: 0,
                            disc_number: 0,
                            length_seconds: track_length,
                            art_id: art_id,
                            path_str: path.to_string_lossy().to_string(),
                            last_modified: last_modified,
                        }   
                    }
                }
            },
            None => {
                TrackInfo {
                    track_name: path_str,
                    artist_name: String::from("Unknown Artist"),
                    album_name: String::from("Unknown Album"),
                    album_artist_name: String::from("Unknown Artist"),
                    track_number: 0,
                    disc_number: 0,
                    length_seconds: 0,
                    art_id: None,
                    path_str: path.to_string_lossy().to_string(),
                    last_modified: last_modified,
                }
            }
        }
    )
}

async fn parse_m4a(
    pool: &PgPool, 
    path: &Path, 
    path_full: &Path,
    last_modified: PrimitiveDateTime,
    art_dir: &str
) -> Result<TrackInfo, BoxError> {
    // get tag
    let tag_optional = mp4ameta::Tag::read_from_path(path_full).ok();

    // get path
    let path_str = path.to_string_lossy().to_string();

    Ok(
        match tag_optional {
            Some(tag) => {
                // get picture
                let mut art_id = None;
                if let Some(art) = tag.artwork() {
                    art_id = Some(get_art_id(art.data, pool, art_dir).await?);
                }

                // in case the track has no embedded cover find it in dir
                if art_id.is_none() {
                    // get image file in parent dir
                    let picture = get_picture_in_dir(path_full)?;
                    if let Some(picture_dir) = picture {
                        art_id = Some(get_art_id(&read(picture_dir)?, pool, art_dir).await?);
                    };
                };

                // get all artists and album artists
                let artists;
                if tag.artists().count() > 0 {
                    artists = tag.artists().join(", ");
                } else {
                    artists = "Unknown Artist".to_string();
                }

                let album_artists;
                if tag.album_artists().count() > 0 {
                    album_artists = tag.album_artists().join(", ");
                } else {
                    album_artists = "Unknown Artist".to_string();
                }

                TrackInfo {
                    track_name: tag.title().unwrap_or(&path_str).to_string(),
                    artist_name: artists,
                    album_name: tag.album().unwrap_or("Unknown Album").to_string(),
                    album_artist_name: album_artists,
                    track_number: tag.track_number().unwrap_or(0) as u32,
                    disc_number: tag.disc_number().unwrap_or(0) as u32,
                    length_seconds: tag.duration().unwrap_or(Duration::new(0,0)).as_secs(),
                    art_id: art_id,
                    path_str: path.to_string_lossy().to_string(),
                    last_modified: last_modified,
                }
            },
            None => {
                TrackInfo {
                    track_name: path_str,
                    artist_name: String::from("Unknown Artist"),
                    album_name: String::from("Unknown Album"),
                    album_artist_name: String::from("Unknown Artist"),
                    track_number: 0,
                    disc_number: 0,
                    length_seconds: 0,
                    art_id: None,
                    path_str: path.to_string_lossy().to_string(),
                    last_modified: last_modified,
                }
            }             
        }           
    )
}


// get an image file in the current directory
fn get_picture_in_dir(path: &Path) -> Result<Option<PathBuf>, BoxError> {
    // get parent
    let parent = path.parent().unwrap(); // since this is called in parse function, this is guaranteed to not be a directory

    // generate common cover names
    let common_names = [
        "cover",
        "folder",
        "front",
    ];
    let common_filenames: Vec<String> = common_names.iter().flat_map(|i| {
        return IMAGE_EXTENSIONS.iter().map(move |ext| format!{"{0}.{1}", i, ext});
    }).collect();

    // find first image with common file name
    let dir_iter = read_dir(parent)?
        .map(|dir| dir.ok()) // ignore unreadable files
        .filter(|dir| dir.is_some()) // ignore nones
        .map(|dir| dir.unwrap()); // should alwasy be unwrappable
    let dir_with_common_filenames: Vec<DirEntry> = dir_iter
        .filter(|d| common_filenames.contains(&d.file_name().to_string_lossy().to_lowercase())).collect();
    match dir_with_common_filenames.get(0) {
        Some(file_with_common_filename) => {
            return Ok(Some(file_with_common_filename.path().to_path_buf()));
        },
        None => {
            // else just find first image
            for dir in read_dir(parent)? {
                let path = dir?.path();
                if let Some(ext) = path.extension() {
                    if IMAGE_EXTENSIONS.iter().any(|i| i == &ext) {
                        return Ok(Some(path.to_path_buf()));
                    }
                }
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
            art_hash_bytes, new_art_name)
            .fetch_one(pool)
            .await?;
    };

    Ok(art_id)
}
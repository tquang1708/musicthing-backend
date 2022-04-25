use std::{
    path::Path,
    time::Duration,
};
use sqlx::types::time::PrimitiveDateTime;
use tower::BoxError;

use id3::TagLike;
use metaflac;
use mp3_duration;

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
    pub art: Option<Vec<u8>>,
    pub path_str: String,
    pub last_modified: PrimitiveDateTime,
}

pub fn parse_tag(path_str: &str) -> Result<TrackInfo, BoxError> {
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
            parse_mp3(path, path_str, last_modified)
        },
        "flac" => {
            parse_flac(path, path_str, last_modified)
        },
        _ => {
            Err(format!("File at {0} has unsupported extension {1}", path_str, extension))?
        },
    }
}

fn parse_mp3(path: &Path, path_str: &str, last_modified: PrimitiveDateTime) -> Result<TrackInfo, BoxError> {
    // get tag
    let tag_optional = id3::Tag::read_from_path(path).ok();

    // get length
    let track_length = mp3_duration::from_path(path).unwrap_or(Duration::new(0, 0)).as_secs();

    Ok(
        match tag_optional {
            Some(tag) => {
                // get picture
                let mut picture = None;
                let mut pictures_iter = tag.pictures();
                loop {
                    if let Some(picture_curr) = pictures_iter.next() {
                        if picture_curr.picture_type == id3::frame::PictureType::CoverFront {
                            // if cover front we can stop
                            picture = Some(picture_curr.data.clone());
                            break;
                        }
                    } else {
                        break;
                    }
                };

                TrackInfo {
                    track_name: tag.title().unwrap_or("Untitled").to_string(),
                    artist_name: tag.artist().unwrap_or("Unknown Artist").to_string(),
                    album_name: tag.album().unwrap_or("Unknown Album").to_string(),
                    album_artist_name: tag.album_artist().unwrap_or("Unknown Artist").to_string(),
                    track_number: tag.track().unwrap_or(0),
                    disc_number: tag.disc().unwrap_or(0),
                    length_seconds: track_length,
                    art: picture,
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
                    art: None,
                    path_str: path_str.to_string(),
                    last_modified: last_modified,
                }
            }             
        }           
    )
}

fn parse_flac(path: &Path, path_str: &str, last_modified: PrimitiveDateTime) -> Result<TrackInfo, BoxError> {
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
                let mut picture = None;
                let mut pictures_iter = tag.pictures();
                loop {
                    if let Some(picture_curr) = pictures_iter.next() {
                        if picture_curr.picture_type == metaflac::block::PictureType::CoverFront {
                            // if cover front we can stop
                            picture = Some(picture_curr.data.clone());
                            break;
                        }
                    } else {
                        break;
                    }
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
                            art: picture,
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
                            art: picture,
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
                    art: None,
                    path_str: path_str.to_string(),
                    last_modified: last_modified,
                }
            }
        }
    )
}
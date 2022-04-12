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
        _ => Err(format!("File at {0} has unsupported extension {1}", path_str, extension))?,
    }
}

fn parse_mp3(path: &Path, path_str: &str, last_modified: PrimitiveDateTime) -> Result<TrackInfo, BoxError> {
    let tag = id3::Tag::read_from_path(path)?;

    // get picture
    let mut pictures_iter = tag.pictures();
    let mut picture = None;
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
    }

    Ok(
        TrackInfo {
            track_name: tag.title().unwrap_or("").to_string(),
            artist_name: tag.artist().unwrap_or("").to_string(),
            album_name: tag.album().unwrap_or("").to_string(),
            album_artist_name: tag.album_artist().unwrap_or("").to_string(),
            track_number: tag.track().unwrap_or(0),
            disc_number: tag.disc().unwrap_or(0),
            length_seconds: mp3_duration::from_path(path).unwrap_or(Duration::new(0, 0)).as_secs(),
            art: picture,
            path_str: path_str.to_string(),
            last_modified: last_modified,
        }
    )
}

fn parse_flac(path: &Path, path_str: &str, last_modified: PrimitiveDateTime) -> Result<TrackInfo, BoxError> {
    let tag = metaflac::Tag::read_from_path(path)?;

    // get length
    let track_length;
    if let Some(streaminfo) = tag.get_streaminfo() {
        track_length = streaminfo.total_samples / streaminfo.sample_rate as u64;
    } else {
        track_length = 0;
    };

    // get picture
    // exact same interface as id3 apparently
    let mut pictures_iter = tag.pictures();
    let mut picture = None;
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
    }

    Ok(
        match tag.vorbis_comments() {
            Some(x) => {
                TrackInfo {
                    track_name: x.comments.get("TITLE").unwrap_or(&Vec::new()).join(", "),
                    artist_name: x.comments.get("ARTIST").unwrap_or(&Vec::new()).join(", "),
                    album_name: x.comments.get("ALBUM").unwrap_or(&Vec::new()).join(", "),
                    album_artist_name: x.comments.get("ALBUMARTIST").unwrap_or(&Vec::new()).join(", "),
                    track_number: x.comments.get("TRACKNUMBER").unwrap_or(&Vec::new()).get(0).unwrap_or(&"0".to_string()).to_string().parse::<u32>()?,
                    disc_number: x.comments.get("DISCNUMBER").unwrap_or(&Vec::new()).get(0).unwrap_or(&"0".to_string()).to_string().parse::<u32>()?,
                    length_seconds: track_length,
                    art: picture,
                    path_str: path_str.to_string(),
                    last_modified: last_modified,
                }
            },
            None => {
                TrackInfo {
                    track_name: String::from(""),
                    artist_name: String::from(""),
                    album_name: String::from(""),
                    album_artist_name: String::from(""),
                    track_number: 0,
                    disc_number: 0,
                    length_seconds: track_length,
                    art: picture,
                    path_str: path_str.to_string(),
                    last_modified: last_modified,
                }
            }
        }
    )
}
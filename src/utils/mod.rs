use std::{
    sync::Arc, 
    env,
    path::PathBuf,
    fs::File,
    io::BufReader,
    collections::HashMap,
};
use axum::http::StatusCode;
use tokio::sync::RwLock;
use tower::BoxError;
use dirs;
use shellexpand;
use serde::{Serialize, Deserialize};
use serde_json;

// config struct
#[derive(Clone, Deserialize, Debug)]
pub struct Config {
    pub database_connection_str: String,
    pub frontend_url: String,
    pub backend_socket_addr: String,
    pub use_tls: bool,
    pub max_db_connections: u32,
    pub db_connection_timeout_seconds: u64,
    pub concurrency_limit: usize,
    pub timeout_seconds: u64,
    pub music_directory: String,
    pub art_directory: String,
}

// parse then return config
pub fn parse_cfg() -> Result<Config, BoxError> {
    // looking up config
    let mut config: Config;
    match find_file("config.json")? {
        Some(path) => {
            // path found
            config = serde_json::from_reader(BufReader::new(File::open(path)?))?;

            // expand music_directory and art_directory
            config.music_directory = shellexpand::full(&config.music_directory)?.to_string();
            config.art_directory = shellexpand::full(&config.art_directory)?.to_string();
        },
        None => {
            // no path found - load default config
            config = Config {
                database_connection_str: "postgres://postgres:password@localhost/musicthing-metadb".to_string(),
                frontend_url: "http://0.0.0.0:3000".to_string(),
                backend_socket_addr: "0.0.0.0:8000".to_string(),
                use_tls: true,
                max_db_connections: 5,
                db_connection_timeout_seconds: 3,
                concurrency_limit: 1024,
                timeout_seconds: 60,
                music_directory: "../music".to_string(),
                art_directory: "./art".to_string(),
            };
            println!("No config.json found. Using default config.");
            println!("{:#?}", config);
        }
    }

    Ok(config)
}

pub fn find_file(filename: &str) -> Result<Option<PathBuf>, BoxError> {
    // look in config
    println!("Searching for {}", filename);
    match dirs::config_dir() {
        Some(config_path) => {
            let mut path = config_path;
            path.push("musicthing");
            path.push(format!("{}", filename));
            println!("Searching in {}...", path.to_str().ok_or("Path isn't a valid UTF-8 string")?);

            if path.exists() {
                println!("{} found.", filename);
                return Ok(Some(path));
            }
        },
        None => {
            println!("Config directory not found. Skipping...");
            println!("Review https://docs.rs/dirs/latest/dirs/fn.config_dir.html for details.");
        }
    }

    // we will reach here if .config doesn't exist
    // look in current directory
    println!("Searching in current directory...");
    let mut path = env::current_dir()?;
    path.push(format!("{}", filename));
    if path.exists() {
        // this code is duped from above so i do wonder whether there's a cleaner way to write this
        println!("{} found.", filename);
        return Ok(Some(path));
    }

    // we can't find it anywhere. Return none
    println!("Failed to find {}", filename);
    Ok(None)
}

// state struct with tokio's rwlock
pub type SharedCache = Arc<RwLock<Cache>>;

#[derive(Debug)]
pub struct Cache {
    pub album_cache: AlbumCache,
    pub album_id_cache: HashMap<String, ListAlbumID>,
}
impl Default for Cache {
    fn default() -> Cache {
        return Cache {
            album_cache: AlbumCache {
                list_album_cache_outdated: true,
                list_album_cache: None,
            },
            album_id_cache: HashMap::new(),
        };
    }
}

// could be none when there's no album
#[derive(Debug)]
pub struct AlbumCache {
    pub list_album_cache_outdated: bool,
    pub list_album_cache: Option<Vec<ListAlbum>>,
}

// list json storing struct for albums query
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ListAlbum {
    pub id: i32,
    pub name: String,
    pub artist_name: String,
    pub art_path: String,
}

// list json storing struct for indiv album query
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ListAlbumID {
    pub id: i32,
    pub name: String,
    pub album_artist_name: String,
    pub art_path: String,
    pub discs: Vec<ListDisc>
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ListDisc {
    pub number: i32,
    pub tracks: Vec<ListTrack>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ListTrack {
    pub number: i32,
    pub artist: String,
    pub name: String,
    pub path: String,
    pub length_seconds: i32,
}

// Utility function for mapping errors into 500 http response
pub fn internal_error(err: BoxError) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, format!("Internal error: {:?}", err))
}

// // slightly different function rwlock poison error
// pub fn internal_poison_error<T>(err: PoisonError<T>) -> (StatusCode, String) {
//     (StatusCode::INTERNAL_SERVER_ERROR, format!("SharedState's lock is poisoned: {}", err.to_string()))
// }
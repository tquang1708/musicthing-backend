use std::{
    sync::Arc, 
    env,
    path::PathBuf,
    fs::File,
    io::BufReader,
};
use axum::http::StatusCode;
use tokio::sync::RwLock;
use tower::BoxError;
use dirs;
use serde::{Serialize, Deserialize};
use serde_json;

// config struct
#[derive(Clone, Deserialize, Debug)]
pub struct Config {
    pub database_connection_str: String,
    pub frontend_url: String,
    pub backend_socket_addr: String,
    pub max_db_connections: u32,
    pub db_connection_timeout_seconds: u64,
    pub concurrency_limit: usize,
    pub timeout_seconds: u64,
    pub music_directory: String,
}

// parse then return config
pub fn parse_cfg() -> Result<Config, BoxError> {
    // looking up config
    let config;
    match find_cfg()? {
        Some(path) => {
            // path found
            config = serde_json::from_reader(BufReader::new(File::open(path)?))?;
        },
        None => {
            // no path found - load default config
            config = Config {
                database_connection_str: "postgres://postgres:password@localhost/musicthing-metadb".to_string(),
                frontend_url: "http://localhost:3000".to_string(),
                backend_socket_addr: "0.0.0.0:8000".to_string(),
                max_db_connections: 5,
                db_connection_timeout_seconds: 3,
                concurrency_limit: 1024,
                timeout_seconds: 60,
                music_directory: "../music-directory".to_string(),
            };
            println!("No config.json found. Using default config.");
            println!("{:#?}", config);
        }
    }

    Ok(config)
}

fn find_cfg() -> Result<Option<PathBuf>, BoxError> {
    // look in config
    println!("Searching for config.json.");
    match dirs::config_dir() {
        Some(config_path) => {
            let mut path = config_path;
            path.push("musicthing");
            path.push("config.json");
            println!("Searching in {}...", path.to_str().ok_or("Path isn't a valid UTF-8 string")?);

            if path.exists() {
                println!("Found config.json. Loading configs.");
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
    path.push("config.json");
    if path.exists() {
        // this code is duped from above so i do wonder whether there's a cleaner way to write this
        println!("Found config.json. Loading configs.");
        return Ok(Some(path));
    }

    // we can't find it anywhere. Return none
    Ok(None)
}

// state struct with tokio's rwlock
pub type SharedState = Arc<RwLock<State>>;

#[derive(Debug)]
pub struct State {
    pub list_cache_outdated: bool,
    pub list_cache: Option<ListRoot>,
}
impl Default for State {
    fn default() -> State {
        return State {
            list_cache_outdated: true,
            list_cache: None,
        };
    }
}

// list json storage struct
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ListRoot {
    pub albums: Vec<ListAlbum>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ListAlbum {
    pub name: String,
    pub album_artist_name: String,
    pub discs: Vec<ListDisc>,
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
}

// Utility function for mapping errors into 500 http response
pub fn internal_error(err: BoxError) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, format!("Internal error: {}", err.to_string()))
}

// // slightly different function rwlock poison error
// pub fn internal_poison_error<T>(err: PoisonError<T>) -> (StatusCode, String) {
//     (StatusCode::INTERNAL_SERVER_ERROR, format!("SharedState's lock is poisoned: {}", err.to_string()))
// }
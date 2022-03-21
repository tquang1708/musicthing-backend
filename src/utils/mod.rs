use std::{
    fs::File,
    io::BufReader,
    sync::{Arc, RwLock},
};
use axum::http::StatusCode;
use tower::BoxError;
use serde::{Serialize, Deserialize};
use serde_json;

// config struct
#[derive(Deserialize, Debug)]
pub struct Config {
    pub database_connection_str: String,
    pub frontend_url: String,
    pub backend_socket_addr: String,
    pub max_db_connections: u32,
    pub db_connection_timeout_seconds: u64,
    pub max_state_concurrency_limit: usize,
    pub state_timeout_seconds: u64,
    pub music_directory: String,
}

// parse then return config
pub fn parse_cfg() -> Result<Config, BoxError> {
    // hard-coding config location
    let config_reader = BufReader::new(File::open("./src/config.json")?);
    let config: Config = serde_json::from_reader(config_reader)?;
    Ok(config)
}

// state struct
pub type SharedState = Arc<RwLock<State>>;

#[derive(Debug)]
pub struct State {
    pub db_modified: bool,
    pub list_cache: Option<ListRoot>,
}
impl Default for State {
    fn default() -> State {
        return State {
            db_modified: true,
            list_cache: None,
        };
    }
}

// list json storage struct
#[derive(Debug, Serialize, Deserialize)]
pub struct ListRoot {
    pub albums: Vec<ListAlbum>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListAlbum {
    pub name: String,
    pub album_artist_name: String,
    pub discs: Vec<ListDisc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListDisc {
    pub number: i32,
    pub tracks: Vec<ListTrack>,
}

#[derive(Debug, Serialize, Deserialize)]
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
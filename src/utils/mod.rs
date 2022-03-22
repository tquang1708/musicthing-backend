use std::sync::Arc;
use axum::http::StatusCode;
use tokio::sync::RwLock;
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
    let json_file = include_bytes!("../config.json");
    let config: Config = serde_json::from_slice(json_file)?;
    Ok(config)
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
use axum::{
    http::StatusCode,
};
use std::{
    fs::File,
    io::BufReader,
    error::Error,
};
use serde::{Deserialize};
use serde_json;

// loading config
#[derive(Deserialize, Debug)]
pub struct Config {
    pub database_connection_str: String,
    pub max_db_connections: u32,
    pub db_connection_timeout_seconds: u64,
    pub music_directory: String,
}

// parse then return config
pub fn parse_cfg() -> Result<Config, Box<dyn Error>> {
    // hard-coding config location
    let config_reader = BufReader::new(File::open("./src/config.json")?);
    let config: Config = serde_json::from_reader(config_reader)?;
    Ok(config)
}

// Utility function for mapping errors into 500 http response
pub fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
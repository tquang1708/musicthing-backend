pub mod demo;
pub mod reload;
pub mod list;
pub mod play;

// structs for interfacing with the database
#[derive(sqlx::FromRow, Debug)]
pub struct Track {
    track_id: i32,
    track_name: String,
    path: String,
    checksum: Vec<u8>,
}
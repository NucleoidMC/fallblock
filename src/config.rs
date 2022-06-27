use std::{fs::File, path::PathBuf};

use serde::Deserialize;

use crate::protocol::{play::JoinGameData, status::ServerListPingResponse};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server_brand: String,
    pub join_game_data: JoinGameData,
    pub spawn_point: (f64, f64, f64),
    pub map_file: PathBuf,
    pub status: ServerListPingResponse,
    #[serde(default)]
    pub modern_forwarding_key: Option<String>,
}

pub fn load_config() -> Config {
    let mut file = File::open("config.json").expect("failed to open config file");
    serde_json::from_reader(&mut file).expect("failed to parse config")
}

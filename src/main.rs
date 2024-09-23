mod network;
mod commands;
mod storage;
mod cache;
mod config;
mod logging;
mod monitoring;
mod cluster;
mod transactions;
mod pubsub;

use crate::config::Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = match Config::from_file("config.toml") {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load config, using default: {}", e);
            Config::new()
        }
    };

    println!("Configuration loaded: {:?}", config);

    Ok(())
}
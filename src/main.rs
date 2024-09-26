use crate::config::config::Config;
use crate::network::server::Server;
use crate::storage::memory::MemoryStorage;
use std::sync::{Arc, Mutex};

mod network;
mod commands;
mod storage;
mod cache;
mod config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::new();
    let storage = Arc::new(Mutex::new(MemoryStorage::new()));

    {
        let mut storage = storage.lock().unwrap();
        if let Err(e) = storage.load_snapshot("redis_data.snapshot") {
            eprintln!("Failed to load snapshot: {}. Starting with empty storage.", e);
        } else {
            println!("Loaded data from snapshot.");
        }
    }

    let storage_clone = Arc::clone(&storage);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(300));
            let storage = storage_clone.lock().unwrap();
            if let Err(e) = storage.save_snapshot("redis_data.snapshot") {
                eprintln!("Failed to save snapshot: {}", e);
            } else {
                println!("Saved snapshot successfully.");
            }
        }
    });

    let server = Server::new(config);
    server.run()?;

    Ok(())
}
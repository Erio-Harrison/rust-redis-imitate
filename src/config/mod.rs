use std::fs;
use std::io;
use serde::de::Error;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub max_memory: usize,
}


impl Config{
    pub fn new() -> Self{
        Config{
            host:"127.0.0.1".to_string(),
            port:6379,
            max_connections:100,
            max_memory:1024*1024*1024,
        }
    }

    pub fn from_file(path: &str) -> Result<Self, io::Error>{
        let contents = fs::read_to_string(path)?;
        let config : Config = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn save_to_file(&self, path:&str) -> Result<(),io::Error>{
        let contents = toml::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        fs::write(path, contents)

    }
}

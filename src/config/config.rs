use serde::{Deserialize, Serialize};

#[derive(Debug,Clone, Deserialize, Serialize)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub max_memory: usize,
}

impl Config{
    pub fn new() -> Self{
        Config{
            host:"0.0.0.0".to_string(),
            port:6379,
            max_connections:1000,
            max_memory:1024*1024*1024,
        }
    }
}

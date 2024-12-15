//! # Configuration Module
//! 
//! Provides configuration settings for the Redis-like server, 
//! with serialization support through serde.

use serde::{Deserialize, Serialize};

/// Server configuration settings
///
/// Holds all configurable parameters for the Redis-like server instance.
/// Supports serialization and deserialization through serde.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
   /// Server host address
   /// Default: "0.0.0.0" (binds to all network interfaces)
   pub host: String,

   /// Server port number
   /// Default: 6379 (standard Redis port)
   pub port: u16,

   /// Maximum number of simultaneous client connections
   /// Default: 1000
   pub max_connections: usize,

   /// Maximum memory usage in bytes
   /// Default: 1GB (1024*1024*1024 bytes)
   pub max_memory: usize,
}

impl Config {
   /// Creates a new Config instance with default values
   ///
   /// # Default Values
   ///
   /// * host: "0.0.0.0" - Binds to all network interfaces
   /// * port: 6379 - Standard Redis port
   /// * max_connections: 1000 - Maximum concurrent connections
   /// * max_memory: 1GB - Maximum memory usage
   ///
   /// # Returns
   ///
   /// A new Config instance initialized with default values
   pub fn new() -> Self {
       Config {
           host: "0.0.0.0".to_string(),
           port: 6379,
           max_connections: 1000,
           max_memory: 1024 * 1024 * 1024,  // 1GB
       }
   }
}
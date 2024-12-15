//! # Server Module
//! 
//! Implements the main Redis-like server functionality, handling network listening,
//! connection management, and thread pool coordination for concurrent client handling.
use crate::config::config::Config;
use crate::network::connection::Connection;
use crate::commands::executor::CommandExecutor;
use crate::storage::memory::MemoryStorage;

use std::net::{TcpListener, TcpStream};
use std::io;
use threadpool::ThreadPool;
use std::sync::{Arc, Mutex};

pub struct Server {
    pub config: Arc<Config>,
    thread_pool: ThreadPool,
    storage: Arc<Mutex<MemoryStorage>>,
}

/// Core server structure managing all server components
/// 
/// Coordinates:
/// - Network listening and connection acceptance
/// - Thread pool for handling concurrent clients
/// - Shared memory storage
/// - Server configuration
impl Server {

   /// Creates a new server instance with the given configuration
    pub fn new(config: Config) -> Self {
        let config = Arc::new(config);
        let thread_pool = ThreadPool::new(config.max_connections);
        let storage = Arc::new(Mutex::new(MemoryStorage::new()));
        Server { config, thread_pool, storage }
    }

   /// Starts the server and begins accepting client connections
   /// # Server Lifecycle
   /// 1. Binds to configured host:port
   /// 2. Accepts incoming connections
   /// 3. Spawns worker thread for each client
   /// 4. Manages shared storage across all connections
    pub fn run(&self) -> io::Result<()> {
        let address = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&address)?;
        println!("Server is running on {}", address);
        
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let storage = Arc::clone(&self.storage);
                    self.thread_pool.execute(move || {
                        let executor = Arc::new(CommandExecutor::new(storage));
                        if let Err(e) = handle_client(stream,  executor) {
                            eprintln!("Error handling client: {}", e);
                        }
                    });
                }
                Err(e) => eprintln!("Connection failed: {}", e),
            }
        }

        Ok(())
    }
}

/// Handles an individual client connection
fn handle_client(stream: TcpStream, executor: Arc<CommandExecutor>) -> io::Result<()> {
    let mut connection = Connection::new(stream,  executor);
    connection.process()
}
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

impl Server {
    pub fn new(config: Config) -> Self {
        let config = Arc::new(config);
        let thread_pool = ThreadPool::new(config.max_connections);
        let storage = Arc::new(Mutex::new(MemoryStorage::new()));
        Server { config, thread_pool, storage }
    }

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

fn handle_client(stream: TcpStream, executor: Arc<CommandExecutor>) -> io::Result<()> {
    let mut connection = Connection::new(stream,  executor);
    connection.process()
}
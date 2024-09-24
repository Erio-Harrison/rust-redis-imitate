use crate::config::Config;
use crate::network::connection::Connection;

use std::net::{TcpListener, TcpStream};
use std::io;
use threadpool::ThreadPool;
use std::sync::Arc;

pub struct Server {
    config: Arc<Config>,
    thread_pool: ThreadPool,
}

impl Server {
    pub fn new(config: Config) -> Self {
        let config = Arc::new(config);
        let thread_pool = ThreadPool::new(config.max_connections);
        Server{config , thread_pool}
    }

    pub fn run(&self) -> io::Result<()>{
        let address = format!("{},{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&address)?;
        println!("Server is running on {}", address);
        
        for stream in listener.incoming(){
            match stream{
                Ok(stream) => {
                    let config = Arc::clone(&self.config);
                    self.thread_pool.execute(move || {
                        if let Err(e) = handle_client(stream, config){
                            eprintln!("Error handling client {}",e);
                        }
                    });
                }
                Err(_e) => eprintln!("Connection failed"),
            }
        }

        Ok(())
    }
}

fn handle_client(stream: TcpStream, config: Arc<Config>) -> io::Result<()> {
    let mut connection = Connection::new(stream, config);
    connection.process()
}
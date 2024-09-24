use crate::config::Config;
use std::net::TcpStream;
use std::io::{self, Read, Write};
use std::sync::Arc;

pub struct Connection {
    stream: TcpStream,
    config: Arc<Config>,
}

impl Connection{
    pub fn new(stream: TcpStream,config:Arc<Config>) -> Self{
        Connection{stream,config}
    }

    pub fn process(&mut self) -> io::Result<()>{
        let mut buffer = [0;512];
        loop{
            let bytes_read = self.stream.read(&mut buffer)?;
            if bytes_read == 0 {
                return Ok(());
            }

            let received = String::from_utf8_lossy(&buffer[..bytes_read]);
            println!("Received: {}",received);
            self.stream.write_all(&buffer[..bytes_read])?;
        }
    }
}
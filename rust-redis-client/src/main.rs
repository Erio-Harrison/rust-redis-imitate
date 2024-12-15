use std::net::TcpStream;
use std::io::{self, Read, Write};
use std::time::Duration;

/// Represents a simple Redis client implemented using TCP communication.
struct RedisClient {
    stream: TcpStream,
}

impl RedisClient {
    /// Creates a new Redis client by connecting to the specified server address.
    ///
    /// # Arguments
    /// - `addr`: The address of the Redis server in the format `IP:PORT`.
    ///
    /// # Returns
    /// - `Ok(Self)` if the connection is successfully established.
    /// - `Err(io::Error)` if there is an error during the connection setup.
    fn new(addr: &str) -> io::Result<Self> {
        let stream = TcpStream::connect(addr)?;
        stream.set_nonblocking(true)?;
        Ok(RedisClient { stream })
    }

    /// Sends a command to the Redis server and retrieves the response.
    ///
    /// # Arguments
    /// - `command`: The Redis command to execute, e.g., `PING`, `SET key value`, etc.
    ///
    /// # Returns
    /// - `Ok(String)` containing the server's response.
    /// - `Err(io::Error)` if there is an error during communication.
    fn send_command(&mut self, command: &str) -> io::Result<String> {
        self.stream.write_all(command.as_bytes())?;
        self.stream.write_all(b"\r\n")?;
        self.stream.flush()?;

        let mut response = String::new();
        let mut buf = [0; 1024];
        let mut retries = 0;
        loop {
            match self.stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    response.push_str(&String::from_utf8_lossy(&buf[..n]));
                    if response.ends_with("\r\n") {
                        break;
                    }
                },
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    if retries >= 50 {
                        return Err(io::Error::new(io::ErrorKind::TimedOut, "Operation timed out"));
                    }
                    std::thread::sleep(Duration::from_millis(100));
                    retries += 1;
                    continue;
                },
                Err(e) => return Err(e),
            }
        }

        Ok(response.trim().to_string())
    }
}

fn main() -> io::Result<()> {
    let mut client = RedisClient::new("170.64.237.20:6379")?;
    println!("Connected to Redis server. Type 'quit' to exit.");

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim();
        if input.eq_ignore_ascii_case("quit") {
            break;
        }

        match client.send_command(input) {
            Ok(response) => println!("{}", response),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    Ok(())
}
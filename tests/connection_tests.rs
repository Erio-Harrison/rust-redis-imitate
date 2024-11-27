use redis_clone::network::connection::Connection;
use redis_clone::commands::executor::CommandExecutor;
use redis_clone::storage::memory::MemoryStorage;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};
    use std::thread;

    // Helper function to create a mock connection
    fn setup_connection() -> (Connection, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        
        let client = TcpStream::connect(addr).unwrap();
        let (server, _) = listener.accept().unwrap();
        
        // Create MemoryStorage and wrap it in Arc<Mutex>
        let storage = Arc::new(Mutex::new(MemoryStorage::new()));
        let executor = Arc::new(CommandExecutor::new(storage));
        let connection = Connection::new(server, executor);
        
        (connection, client)
    }

    #[test]
    fn test_basic_command() {
        let (mut connection, mut client) = setup_connection();
        
        // Spawn a thread to handle the connection
        let handle = thread::spawn(move || {
            connection.process().unwrap();
        });

        // Send a SET command
        writeln!(client, "SET key value").unwrap();
        
        // Read response
        let mut response = String::new();
        let mut reader = BufReader::new(client);
        reader.read_line(&mut response).unwrap();
        
        assert_eq!(response.trim(), "OK");
        
        // Close connection
        drop(reader);
        handle.join().unwrap();
    }

    #[test]
    fn test_nested_transactions() {
        let (mut connection, client) = setup_connection();
        
        let handle = thread::spawn(move || {
            connection.process().unwrap();
        });

        let mut reader = BufReader::new(client);
        let mut response = String::new();

        // Start outer transaction
        writeln!(reader.get_ref(), "MULTI").unwrap();
        reader.read_line(&mut response).unwrap();
        assert_eq!(response.trim(), "OK");

        // Start inner transaction
        writeln!(reader.get_ref(), "MULTI").unwrap();
        response.clear();
        reader.read_line(&mut response).unwrap();
        assert_eq!(response.trim(), "OK");

        // Queue command in inner transaction
        writeln!(reader.get_ref(), "SET inner value").unwrap();
        response.clear();
        reader.read_line(&mut response).unwrap();
        assert_eq!(response.trim(), "QUEUED");

        // Execute inner transaction
        writeln!(reader.get_ref(), "EXEC").unwrap();
        response.clear();
        reader.read_line(&mut response).unwrap();
        assert!(response.trim().contains("OK"));

        // Execute outer transaction
        writeln!(reader.get_ref(), "EXEC").unwrap();
        response.clear();
        reader.read_line(&mut response).unwrap();
        assert!(response.trim().contains("OK"));

        // Close connection
        drop(reader);
        handle.join().unwrap();
    }

    #[test]
    fn test_transaction_discard() {
        let (mut connection, client) = setup_connection();
        
        let handle = thread::spawn(move || {
            connection.process().unwrap();
        });

        let mut reader = BufReader::new(client);
        let mut response = String::new();

        // Start transaction
        writeln!(reader.get_ref(), "MULTI").unwrap();
        reader.read_line(&mut response).unwrap();
        assert_eq!(response.trim(), "OK");

        // Queue command
        writeln!(reader.get_ref(), "SET key value").unwrap();
        response.clear();
        reader.read_line(&mut response).unwrap();
        assert_eq!(response.trim(), "QUEUED");

        // Discard transaction
        writeln!(reader.get_ref(), "DISCARD").unwrap();
        response.clear();
        reader.read_line(&mut response).unwrap();
        assert_eq!(response.trim(), "OK");

        // Verify key was not set
        writeln!(reader.get_ref(), "GET key").unwrap();
        response.clear();
        reader.read_line(&mut response).unwrap();
        assert_eq!(response.trim(), "(nil)");

        // Close connection
        drop(reader);
        handle.join().unwrap();
    }

    #[test]
    fn test_invalid_transaction_commands() {
        let (mut connection, client) = setup_connection();
        
        let handle = thread::spawn(move || {
            connection.process().unwrap();
        });

        let mut reader = BufReader::new(client);
        let mut response = String::new();

        // Try EXEC without MULTI
        writeln!(reader.get_ref(), "EXEC").unwrap();
        reader.read_line(&mut response).unwrap();
        assert_eq!(response.trim(), "ERR EXEC without MULTI");

        // Try DISCARD without MULTI
        writeln!(reader.get_ref(), "DISCARD").unwrap();
        response.clear();
        reader.read_line(&mut response).unwrap();
        assert_eq!(response.trim(), "ERR DISCARD without MULTI");

        // Close connection
        drop(reader);
        handle.join().unwrap();
    }
}
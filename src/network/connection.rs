//! # Connection Module
//! 
//! Handles individual client connections, providing command processing,
//! transaction management, and network communication for the Redis-like server.
use std::collections::VecDeque;
use crate::commands::parser::{Command, CommandParser};
use crate::commands::executor::CommandExecutor;
use std::net::TcpStream;
use std::io::{self, BufRead, BufReader, Write};
use std::sync::Arc;

/// Manages a single client connection and its transaction state
///
/// Handles the lifecycle of a client connection, including:
/// - Command reading and parsing
/// - Transaction management
/// - Response writing
/// - Connection state maintenance
pub struct Connection {
    stream: BufReader<TcpStream>,
    executor: Arc<CommandExecutor>,
    transaction_stack: VecDeque<Vec<Command>>,
}

impl Connection {

    /// Creates a new Connection instance
    ///
    /// # Arguments
    ///
    /// * `stream` - TCP stream for the client connection
    /// * `executor` - Shared command executor for processing commands
    ///
    /// # Returns
    ///
    /// A new Connection instance ready to process client commands
    pub fn new(stream: TcpStream, executor: Arc<CommandExecutor>) -> Self {
        Connection {
            stream: BufReader::new(stream),
            executor,
            transaction_stack: VecDeque::new(),
        }
    }

   /// Processes client commands in a loop until the connection is closed
   ///
   /// # Returns
   ///
   /// * `Ok(())` if the connection was closed normally
   /// * `Err(e)` if an I/O error occurred
   ///
   /// # Command Processing Flow
   ///
   /// 1. Reads command from client
   /// 2. Parses the command
   /// 3. Handles the command (including transaction management)
   /// 4. Writes response back to client
    pub fn process(&mut self) -> io::Result<()> {
        loop {
            let mut command = String::new();
            let bytes_read = self.stream.read_line(&mut command)?;
            if bytes_read == 0 {
                println!("Client disconnected");
                return Ok(());
            }
            println!("Received command: {}", command.trim());
            let parsed_command = CommandParser::parse(&command);
            let response = self.handle_command(parsed_command);
            
            println!("Sending response: {}", response);
            for line in response.lines(){
                self.stream.get_mut().write_all(line.as_bytes())?;
                self.stream.get_mut().write_all(b"\r\n")?;
            }
            self.stream.get_mut().flush()?;
        }
    }

   /// Handles a single command, managing transaction state as needed
   ///
   /// # Arguments
   ///
   /// * `command` - The parsed command to handle
   ///
   /// # Returns
   ///
   /// A string response to send back to the client
   ///
   /// # Transaction Handling
   ///
   /// * MULTI - Starts a new transaction
   /// * EXEC - Executes the current transaction
   /// * DISCARD - Discards the current transaction
   /// * Other commands - Queued if in transaction, executed immediately otherwise
    fn handle_command(&mut self, command: Command) -> String {
        match command {
            Command::Multi => {
                self.transaction_stack.push_back(Vec::new());
                self.executor.execute_command(command)
            }
            Command::Exec => {
                if self.transaction_stack.is_empty() {
                    "ERR EXEC without MULTI".to_string()
                } else {
                    let commands = self.transaction_stack.pop_back().unwrap();
                    let results = self.executor.execute_transaction(&commands);
                    if !self.transaction_stack.is_empty() {
                        // If still in the outer transaction, add the results as multiple commands
                        for result in results.iter() {
                            self.transaction_stack.back_mut().unwrap().push(
                                Command::Set(format!("RESULT:{}", uuid::Uuid::new_v4().to_string()), result.clone())
                            );
                        }
                    }
                    results.join("\n")
                }
            }
            Command::Discard => {
                if self.transaction_stack.is_empty() {
                    "ERR DISCARD without MULTI".to_string()
                } else {
                    self.transaction_stack.pop_back();
                    self.executor.execute_command(command)
                }
            }
            _ => {
                if !self.transaction_stack.is_empty() {
                    self.transaction_stack.back_mut().unwrap().push(command);
                    "QUEUED".to_string()
                } else {
                    self.executor.execute_command(command)
                }
            }
        }
    }
}
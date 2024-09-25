use crate::config::Config;
use std::collections::VecDeque;
use crate::commands::parser::{Command, CommandParser};
use crate::commands::executor::CommandExecutor;
use std::net::TcpStream;
use std::io::{self, BufRead, BufReader, Write};
use std::sync::Arc;


pub struct Connection {
    stream: BufReader<TcpStream>,
    config: Arc<Config>,
    executor: Arc<CommandExecutor>,
    transaction_stack: VecDeque<Vec<Command>>,
}

impl Connection {
    pub fn new(stream: TcpStream, config: Arc<Config>, executor: Arc<CommandExecutor>) -> Self {
        Connection {
            stream: BufReader::new(stream),
            config,
            executor,
            transaction_stack: VecDeque::new(),
        }
    }

    pub fn process(&mut self) -> io::Result<()> {
        loop {
            let mut command = String::new();
            let bytes_read = self.stream.read_line(&mut command)?;
            if bytes_read == 0 {
                return Ok(());
            }

            let parsed_command = CommandParser::parse(&command);
            let response = self.handle_command(parsed_command);

            self.stream.get_mut().write_all(response.as_bytes())?;
            self.stream.get_mut().write_all(b"\r\n")?;
        }
    }

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
//! # Command Executor Module
//! 
//! This module provides the execution layer for Redis-like commands,
//! handling command processing and storage interactions with thread-safe
//! mechanisms using Arc and Mutex.

use std::sync::Arc;
use std::sync::Mutex;
use crate::storage::memory::MemoryStorage;

use super::parser::Command;

/// A thread-safe command executor that processes Redis-like commands
/// 
/// Manages the execution of commands against a shared memory storage,
/// providing atomic operations and transaction support.
pub struct CommandExecutor {
    storage: Arc<Mutex<MemoryStorage>>
}

impl CommandExecutor {
    /// Creates a new CommandExecutor with the given shared storage
    ///
    /// # Arguments
    ///
    /// * `storage` - Thread-safe reference to the memory storage
    pub fn new(storage: Arc<Mutex<MemoryStorage>>) -> Self {
        CommandExecutor { storage }
    }

    /// Executes a single command and returns the result as a string
    ///
    /// # Arguments
    ///
    /// * `command` - The command to execute
    ///
    /// # Returns
    ///
    /// A string containing the command's result or error message
    ///
    /// # Command Results
    ///
    /// * SET - Returns "OK" on success
    /// * GET - Returns the value or "(nil)" if not found
    /// * DEL - Returns "1" if key was deleted, "0" if key didn't exist
    /// * INCR/DECR - Returns the new value after increment/decrement
    /// * LPUSH/RPUSH - Returns the new length of the list
    /// * LPOP/RPOP - Returns the popped value or "(nil)" if list is empty
    /// * LLEN - Returns the length of the list
    /// * MULTI - Returns "OK" when transaction starts
    /// * EXEC - Returns all transaction results followed by "OK"
    /// * DISCARD - Returns "OK" if transaction was rolled back successfully
    pub fn execute_command(&self, command: Command) -> String {
        let mut storage = self.storage.lock().unwrap();
        match command {
            Command::Set(key, value) => {
                storage.set(key, value);
                "OK".to_string()
            },
            Command::Get(key) => {
                match storage.get(&key) {
                    Some(value) => value.clone(),
                    None => "(nil)".to_string(),
                }
            },
            Command::Del(key) => {
                match storage.del(&key) {
                    true => "1".to_string(),
                    false => "0".to_string(),
                }
            },
            Command::Incr(key) => {
                storage.incr(&key).to_string()
            },
            Command::Decr(key) => {
                storage.decr(&key).to_string()
            },
            Command::LPush(key, value) => {
                storage.lpush(&key, value).to_string()
            },
            Command::RPush(key, value) => {
                storage.rpush(&key, value).to_string()
            },
            Command::LPop(key) => {
                match storage.lpop(&key) {
                    Some(value) => value,
                    None => "(nil)".to_string(),
                }
            },
            Command::RPop(key) => {
                match storage.rpop(&key) {
                    Some(value) => value,
                    None => "(nil)".to_string(),
                }
            },
            Command::LLen(key) => {
                storage.llen(&key).to_string()
            },
            Command::Multi =>{
                storage.start_transaction();
                "OK".to_string()
            },
            Command::Exec => {
                match storage.commit_transaction() {
                    Ok(results) => {
                        let mut response = String::new();
                        for result in results {
                            response.push_str(&format!("{}\n", result));
                        }
                        response.push_str("OK\n");
                        response
                    },
                    Err(e) => format!("ERR: {}\n", e),
                }
            },
            Command::Discard => {
                match storage.rollback_transaction() {
                    Ok(_) => "OK".to_string(),
                    Err(e) => format!("ERR: {}", e),
                }
            },
            Command::Unknown(cmd) => format!("ERR unknown command '{}'", cmd),
        }
    }
    
    /// Executes a batch of commands as part of a transaction
    ///
    /// # Arguments
    ///
    /// * `commands` - A slice of commands to execute in order
    ///
    /// # Returns
    ///
    /// A vector of strings containing the results of each command
    ///
    /// # Transaction Behavior
    ///
    /// * All commands in the transaction are executed atomically
    /// * If any command fails, the entire transaction is rolled back
    /// * Results are collected and returned in the order of execution
    pub fn execute_transaction(&self, commands: &[Command]) -> Vec<String> {
        let mut results = Vec::new();
        let mut storage = self.storage.lock().unwrap();
        
        for command in commands {
            let result = match command {
                Command::Set(key, value) => {
                    storage.set(key.to_string(), value.to_string());
                    "OK".to_string()
                },
                Command::Get(key) => {
                    match storage.get(&key) {
                        Some(value) => value.clone(),
                        None => "(nil)".to_string(),
                    }
                },
                Command::Del(key) => {
                    match storage.del(&key) {
                        true => "1".to_string(),
                        false => "0".to_string(),
                    }
                },
                Command::Incr(key) => {
                    storage.incr(&key).to_string()
                },
                Command::Decr(key) => {
                    storage.decr(&key).to_string()
                },
                Command::LPush(key, value) => {
                    storage.lpush(&key, value.to_string()).to_string()
                },
                Command::RPush(key, value) => {
                    storage.rpush(&key, value.to_string()).to_string()
                },
                Command::LPop(key) => {
                    match storage.lpop(&key) {
                        Some(value) => value,
                        None => "(nil)".to_string(),
                    }
                },
                Command::RPop(key) => {
                    match storage.rpop(&key) {
                        Some(value) => value,
                        None => "(nil)".to_string(),
                    }
                },
                Command::LLen(key) => {
                    storage.llen(&key).to_string()
                },
                Command::Multi =>{
                    storage.start_transaction();
                    "OK".to_string()
                },
                Command::Exec => {
                    match storage.commit_transaction() {
                        Ok(results) => {
                            let mut response = String::new();
                            for result in results {
                                response.push_str(&format!("{}\n", result));
                            }
                            response.push_str("OK\n");
                            response
                        },
                        Err(e) => format!("ERR: {}\n", e),
                    }
                },
                Command::Discard => {
                    match storage.rollback_transaction() {
                        Ok(_) => "OK".to_string(),
                        Err(e) => format!("ERR: {}", e),
                    }
                },
                Command::Unknown(cmd) => format!("ERR unknown command '{}'", cmd),
            
            };
            results.push(result);
        }
        
        results
    }
    
}
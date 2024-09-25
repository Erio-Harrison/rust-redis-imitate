use std::sync::Arc;
use std::sync::Mutex;
use crate::storage::memory::MemoryStorage;

use super::parser::Command;
pub struct CommandExecutor {
    storage: Arc<Mutex<MemoryStorage>>
}

impl CommandExecutor {
    pub fn new(storage: Arc<Mutex<MemoryStorage>>) -> Self {
        CommandExecutor { storage }
    }

    pub fn execute_command(&self, command: Command) -> String {
        let mut storage = self.storage.lock().unwrap();
        match command {
            Command::Set(key, value) => {
                storage.set(key, value);
                "OK".to_string()
            }
            Command::Get(key) => {
                match storage.get(&key) {
                    Some(value) => value.clone(),
                    None => "(nil)".to_string(),
                }
            }
            Command::Del(key) => {
                match storage.del(&key) {
                    true => "1".to_string(),
                    false => "0".to_string(),
                }
            }
            Command::Incr(key) => {
                storage.incr(&key).to_string()
            }
            Command::Decr(key) => {
                storage.decr(&key).to_string()
            }
            Command::LPush(key, value) => {
                storage.lpush(&key, value).to_string()
            }
            Command::RPush(key, value) => {
                storage.rpush(&key, value).to_string()
            }
            Command::LPop(key) => {
                match storage.lpop(&key) {
                    Some(value) => value,
                    None => "(nil)".to_string(),
                }
            }
            Command::RPop(key) => {
                match storage.rpop(&key) {
                    Some(value) => value,
                    None => "(nil)".to_string(),
                }
            }
            Command::LLen(key) => {
                storage.llen(&key).to_string()
            }
            Command::Multi =>{
                storage.start_transaction();
                "OK".to_string()
            },
            Command::Exec => {
                storage.commit_transaction();
                "OK".to_string()
            },
            Command::Discard =>{
                storage.rollback_transaction();
                "OK".to_string()
            }, 
            Command::Unknown(cmd) => format!("ERR unknown command '{}'", cmd),
        }
    }

    pub fn execute_transaction(&self, commands: &[Command]) -> Vec<String> {
        let mut results = Vec::new();
        let mut storage = self.storage.lock().unwrap();
        
        for command in commands {
            let result = match command {
                Command::Set(key, value) => {
                    storage.set(key.to_string(), value.to_string());
                    "OK".to_string()
                }
                Command::Get(key) => {
                    match storage.get(&key) {
                        Some(value) => value.clone(),
                        None => "(nil)".to_string(),
                    }
                }
                Command::Del(key) => {
                    match storage.del(&key) {
                        true => "1".to_string(),
                        false => "0".to_string(),
                    }
                }
                Command::Incr(key) => {
                    storage.incr(&key).to_string()
                }
                Command::Decr(key) => {
                    storage.decr(&key).to_string()
                }
                Command::LPush(key, value) => {
                    storage.lpush(&key, value.to_string()).to_string()
                }
                Command::RPush(key, value) => {
                    storage.rpush(&key, value.to_string()).to_string()
                }
                Command::LPop(key) => {
                    match storage.lpop(&key) {
                        Some(value) => value,
                        None => "(nil)".to_string(),
                    }
                }
                Command::RPop(key) => {
                    match storage.rpop(&key) {
                        Some(value) => value,
                        None => "(nil)".to_string(),
                    }
                }
                Command::LLen(key) => {
                    storage.llen(&key).to_string()
                }
                Command::Multi =>{
                    storage.start_transaction();
                    "OK".to_string()
                },
                Command::Exec => {
                    storage.commit_transaction();
                    "OK".to_string()
                },
                Command::Discard =>{
                    storage.rollback_transaction();
                    "OK".to_string()
                }, 
                Command::Unknown(cmd) => format!("ERR unknown command '{}'", cmd),
                _ => unreachable!(),
            
            };
            results.push(result);
        }
        
        results
    }
    
}
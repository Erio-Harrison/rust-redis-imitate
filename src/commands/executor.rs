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

    pub fn execute(&self, command: Command) -> String {
        match command {
            Command::Set(key, value) => {
                let mut storage = self.storage.lock().unwrap();
                storage.set(key, value);
                "OK".to_string()
            }
            Command::Get(key) => {
                let storage = self.storage.lock().unwrap();
                match storage.get(&key) {
                    Some(value) => value.clone(),
                    None => "(nil)".to_string(),
                }
            }
            Command::Del(key) => {
                let mut storage = self.storage.lock().unwrap();
                match storage.del(&key) {
                    true => "1".to_string(),
                    false => "0".to_string(),
                }
            }
            Command::Incr(key) => {
                let mut storage = self.storage.lock().unwrap();
                storage.incr(&key).to_string()
            }
            Command::Decr(key) => {
                let mut storage = self.storage.lock().unwrap();
                storage.decr(&key).to_string()
            }
            Command::LPush(key, value) => {
                let mut storage = self.storage.lock().unwrap();
                storage.lpush(&key, value).to_string()
            }
            Command::RPush(key, value) => {
                let mut storage = self.storage.lock().unwrap();
                storage.rpush(&key, value).to_string()
            }
            Command::LPop(key) => {
                let mut storage = self.storage.lock().unwrap();
                match storage.lpop(&key) {
                    Some(value) => value,
                    None => "(nil)".to_string(),
                }
            }
            Command::RPop(key) => {
                let mut storage = self.storage.lock().unwrap();
                match storage.rpop(&key) {
                    Some(value) => value,
                    None => "(nil)".to_string(),
                }
            }
            Command::LLen(key) => {
                let storage = self.storage.lock().unwrap();
                storage.llen(&key).to_string()
            }
            Command::Unknown(cmd) => format!("ERR unknown command '{}'", cmd),
        }
    }
}
//! # Memory Storage Module
//! 
//! Provides in-memory storage implementation with support for:
//! - String and List data types
//! - Transaction management with MULTI/EXEC/DISCARD
//! - Snapshots for persistence
//! - LRU caching
//! - Thread-safe concurrent access
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::fs::File;
use std::io::{self, BufWriter, BufReader, Write, BufRead};
use crate::cache::avlcache::AVLCache;
use std::time::Duration;

/// Represents a single transaction layer with changes to strings and lists
#[derive(Clone)]
struct TransactionLayer {
    strings: HashMap<String, Option<String>>,
    lists: HashMap<String, Option<VecDeque<String>>>,
}

/// Main storage engine implementing Redis-like functionality
///
/// Provides thread-safe storage with transaction support and caching.
/// All keys are case-insensitive and stored in lowercase.
pub struct MemoryStorage {
    strings: Arc<HashMap<String, String>>,
    lists: Arc<HashMap<String, VecDeque<String>>>,
    transaction_stack: Vec<TransactionLayer>,
    cache: AVLCache<String,String>,
}

impl MemoryStorage {
    /// Creates a new empty storage instance with default cache settings
    pub fn new() -> Self {
        MemoryStorage {
            strings: Arc::new(HashMap::new()),
            lists: Arc::new(HashMap::new()),
            transaction_stack: Vec::new(),
            cache: AVLCache::new(1000, Duration::from_secs(300)),
        }
    }

   /// Saves the current storage state to a file
   ///
   /// # Arguments
   ///
   /// * `path` - Path to save the snapshot file
    pub fn save_snapshot(&self, path: &str) -> io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        for (key, value) in self.strings.iter() {
            writeln!(writer, "STRING {} {}", key, value)?;
        }

        for (key, list) in self.lists.iter() {
            write!(writer, "LIST {} {}", key, list.len())?;
            for item in list {
                write!(writer, " {}", item)?;
            }
            writeln!(writer)?;
        }

        Ok(())
    }

   /// Loads storage state from a snapshot file
   ///
   /// # Arguments
   ///
   /// * `path` - Path to the snapshot file to load
    pub fn load_snapshot(&mut self, path: &str) -> io::Result<()> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut new_strings = HashMap::new();
        let mut new_lists = HashMap::new();

        for line in reader.lines() {
            let line = line?;
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "STRING" => {
                    if parts.len() >= 3 {
                        new_strings.insert(parts[1].to_string(), parts[2].to_string());
                    }
                }
                "LIST" => {
                    if parts.len() >= 3 {
                        let mut list = VecDeque::new();
                        for item in &parts[3..] {
                            list.push_back(item.to_string());
                        }
                        new_lists.insert(parts[1].to_string(), list);
                    }
                }
                _ => {}
            }
        }

        self.strings = Arc::new(new_strings);
        self.lists = Arc::new(new_lists);
        Ok(())
    }

   /// Starts a new transaction
   ///
   /// Creates a new transaction layer that will track changes until committed or rolled back.
    pub fn start_transaction(&mut self) {
        self.transaction_stack.push(TransactionLayer {
            strings: HashMap::new(),
            lists: HashMap::new(),
        });
    }

   /// Commits the current transaction
   ///
   /// # Returns
   ///
   /// * `Ok(Vec<String>)` - Results of operations in the transaction
   /// * `Err(String)` - Error message if no transaction is active
    pub fn commit_transaction(&mut self) -> Result<Vec<String>, String> {
        if self.transaction_stack.is_empty() {
            return Err("No active transaction to commit".to_string());
        }
    
        let committed_layer = self.transaction_stack.pop().unwrap();
        let mut results = Vec::new();
    
        if self.transaction_stack.is_empty() {
            // This is the outermost transaction, apply changes to main storage
            let mut new_strings = (*self.strings).clone();
            for (key, value_opt) in committed_layer.strings {
                match value_opt {
                    Some(value) => { 
                        new_strings.insert(key, value.clone()); 
                        results.push("OK".to_string());
                    }
                    None => { 
                        new_strings.remove(&key); 
                        results.push("OK".to_string());
                    }
                }
            }
            self.strings = Arc::new(new_strings);
    
            let mut new_lists = (*self.lists).clone();
            for (key, value_opt) in committed_layer.lists {
                match value_opt {
                    Some(value) => { 
                        new_lists.insert(key, value.clone()); 
                        results.push(value.len().to_string());
                    }
                    None => { 
                        new_lists.remove(&key); 
                        results.push("OK".to_string());
                    }
                }
            }
            self.lists = Arc::new(new_lists);
        } else {
            // This is a nested transaction, merge changes into the parent transaction
            let parent_layer = self.transaction_stack.last_mut().unwrap();
            for (key, value_opt) in committed_layer.strings {
                parent_layer.strings.insert(key, value_opt);
                results.push("QUEUED".to_string());
            }
            for (key, value_opt) in committed_layer.lists {
                parent_layer.lists.insert(key, value_opt);
                results.push("QUEUED".to_string());
            }
        }
        
        self.cache.clear();
        Ok(results)
    }

   /// Rolls back the current transaction
   ///
   /// # Returns
   ///
   /// * `Ok(())` - Transaction successfully rolled back
   /// * `Err(String)` - Error message if no transaction is active
    pub fn rollback_transaction(&mut self) -> Result<(), String> {
        if self.transaction_stack.is_empty() {
            return Err("No active transaction to rollback".to_string());
        }
        self.transaction_stack.pop();
        self.cache.clear();
        Ok(())
    }

    /// Sets a key-value pair in the storage
    ///
    /// If a transaction is active, the change is recorded in the current transaction layer.
    /// Otherwise, it's applied directly to the main storage. The value is also cached.
    ///
    /// # Arguments
    ///
    /// * `key` - The key (case-insensitive)
    /// * `value` - The value to store
    pub fn set(&mut self, key: String, value: String) {
        let key = key.to_lowercase();
        if let Some(layer) = self.transaction_stack.last_mut() {
            layer.strings.insert(key.clone(), Some(value.clone()));
        } else {
            Arc::make_mut(&mut self.strings).insert(key.clone(), value.clone());
        }
        self.cache.put(key, value);
    }

    /// Retrieves a value by its key
    ///
    /// Checks the cache first, then active transactions from newest to oldest,
    /// finally falling back to main storage. Found values are cached for future access.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up (case-insensitive)
    ///
    /// # Returns
    ///
    /// * `Some(String)` - The value if found
    /// * `None` - If the key doesn't exist
    pub fn get(&mut self, key: &str) -> Option<String> {
        let key = key.to_lowercase();
        
        if let Some(value) = self.cache.get(&key) {
            return Some(value);
        }
    
        let result = self.transaction_stack.iter().rev()
            .find_map(|layer| layer.strings.get(&key).cloned().flatten())
            .or_else(|| self.strings.get(&key).cloned());
    
        if let Some(value) = result.as_ref() {
            self.cache.put(key.clone(), value.clone());
        }
    
        result
    }

    /// Deletes a key-value pair from storage
    ///
    /// In a transaction, marks the key for deletion.
    /// Otherwise, removes it from main storage immediately.
    /// Also removes the key from cache if it existed.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to delete (case-insensitive)
    ///
    /// # Returns
    ///
    /// `true` if the key existed and was marked for deletion or removed
    pub fn del(&mut self, key: &str) -> bool {
        let key = key.to_lowercase();
        let result = if let Some(layer) = self.transaction_stack.last_mut() {
            layer.strings.insert(key.to_string(), None);
            layer.lists.insert(key.to_string(), None);
            true
        } else {
            Arc::make_mut(&mut self.strings).remove(&key).is_some() ||
            Arc::make_mut(&mut self.lists).remove(&key).is_some()
        };
        if result {
            self.cache.remove(&key);
        }

        result
    }

    /// Increments the numeric value stored at the given key
    ///
    /// If the key doesn't exist, it's initialized with "0" before incrementing.
    /// Non-numeric values are treated as 0.
    ///
    /// # Arguments
    ///
    /// * `key` - The key storing the numeric value (case-insensitive)
    ///
    /// # Returns
    ///
    /// The new value after incrementing
    pub fn incr(&mut self, key: &str) -> i64 {
        let key = key.to_lowercase();
        let value = self.get_or_insert_string(&key, "0".to_string());
        let mut num: i64 = value.parse().unwrap_or(0);
        num += 1;
        *value = num.to_string();
        num
    }

    /// Decrements the numeric value stored at the given key
    ///
    /// If the key doesn't exist, it's initialized with "0" before decrementing.
    /// Non-numeric values are treated as 0.
    ///
    /// # Arguments
    ///
    /// * `key` - The key storing the numeric value (case-insensitive)
    ///
    /// # Returns
    ///
    /// The new value after decrementing
    pub fn decr(&mut self, key: &str) -> i64 {
        let key = key.to_lowercase();
        let value = self.get_or_insert_string(&key, "0".to_string());
        let mut num: i64 = value.parse().unwrap_or(0);
        num -= 1;
        *value = num.to_string();
        num
    }
    
    /// Pushes a value to the front of a list
    ///
    /// Creates the list if it doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `key` - The list's key (case-insensitive)
    /// * `value` - The value to push
    ///
    /// # Returns
    ///
    /// The new length of the list
    pub fn lpush(&mut self, key: &str, value: String) -> usize {
        let key = key.to_lowercase();
        let list = self.get_or_insert_list(&key);
        list.push_front(value);
        list.len()
    }
    
    /// Pushes a value to the end of a list
    ///
    /// Creates the list if it doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `key` - The list's key (case-insensitive)
    /// * `value` - The value to push
    ///
    /// # Returns
    ///
    /// The new length of the list
    pub fn rpush(&mut self, key: &str, value: String) -> usize {
        let key = key.to_lowercase();
        let list = self.get_or_insert_list(&key);
        list.push_back(value);
        list.len()
    }

    /// Removes and returns the first element from a list
    ///
    /// # Arguments
    ///
    /// * `key` - The list's key (case-insensitive)
    ///
    /// # Returns
    ///
    /// * `Some(String)` - The removed value
    /// * `None` - If the list is empty or doesn't exist
    pub fn lpop(&mut self, key: &str) -> Option<String> {
        let key = key.to_lowercase();
        self.get_or_insert_list(&key).pop_front()
    }

    /// Removes and returns the last element from a list
    ///
    /// # Arguments
    ///
    /// * `key` - The list's key (case-insensitive)
    ///
    /// # Returns
    ///
    /// * `Some(String)` - The removed value
    /// * `None` - If the list is empty or doesn't exist
    pub fn rpop(&mut self, key: &str) -> Option<String> {
        let key = key.to_lowercase();
        self.get_or_insert_list(&key).pop_back()
    }

    /// Returns the length of a list
    ///
    /// If in a transaction, returns the length from the most recent transaction layer
    /// that has the list. Otherwise, returns the length from main storage.
    ///
    /// # Arguments
    ///
    /// * `key` - The list's key (case-insensitive)
    ///
    /// # Returns
    ///
    /// The length of the list, or 0 if it doesn't exist
    pub fn llen(&self, key: &str) -> usize {
        let key = key.to_lowercase();
        for layer in self.transaction_stack.iter().rev() {
            if let Some(Some(list)) = layer.lists.get(&key) {
                return list.len();
            }
        }
        self.lists.get(&key).map_or(0, |list| list.len())
    }

    /// Helper method to get or insert a string value
    ///
    /// Returns a mutable reference to the string value, creating it if necessary
    fn get_or_insert_string(&mut self, key: &str, default: String) -> &mut String {
        let key = key.to_lowercase();
        if let Some(layer) = self.transaction_stack.last_mut() {
            layer.strings.entry(key.to_string())
                .or_insert_with(|| self.strings.get(&key).cloned())
                .get_or_insert(default)
        } else {
            Arc::make_mut(&mut self.strings)
                .entry(key.to_string())
                .or_insert(default)
        }
    }

   /// Helper method to get or insert a list
   ///
   /// Returns a mutable reference to the list, creating it if necessary
    fn get_or_insert_list(&mut self, key: &str) -> &mut VecDeque<String> {
        let key = key.to_lowercase();
        if let Some(layer) = self.transaction_stack.last_mut() {
            layer.lists.entry(key.to_string())
                .or_insert_with(|| self.lists.get(&key).cloned())
                .get_or_insert_with(VecDeque::new)
        } else {
            Arc::make_mut(&mut self.lists)
                .entry(key.to_string())
                .or_insert_with(VecDeque::new)
        }
    }
}
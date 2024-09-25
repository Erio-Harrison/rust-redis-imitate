use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

#[derive(Clone)]
struct TransactionLayer {
    strings: HashMap<String, Option<String>>,
    lists: HashMap<String, Option<VecDeque<String>>>,
}

pub struct MemoryStorage {
    strings: Arc<HashMap<String, String>>,
    lists: Arc<HashMap<String, VecDeque<String>>>,
    transaction_stack: Vec<TransactionLayer>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        MemoryStorage {
            strings: Arc::new(HashMap::new()),
            lists: Arc::new(HashMap::new()),
            transaction_stack: Vec::new(),
        }
    }

    pub fn start_transaction(&mut self) {
        self.transaction_stack.push(TransactionLayer {
            strings: HashMap::new(),
            lists: HashMap::new(),
        });
    }

    pub fn commit_transaction(&mut self) -> Result<(), String> {
        if self.transaction_stack.is_empty() {
            return Err("No active transaction to commit".to_string());
        }

        let committed_layer = self.transaction_stack.pop().unwrap();

        if self.transaction_stack.is_empty() {
            // This is the outermost transaction, apply changes to main storage
            let mut new_strings = (*self.strings).clone();
            for (key, value_opt) in committed_layer.strings {
                match value_opt {
                    Some(value) => { new_strings.insert(key, value); }
                    None => { new_strings.remove(&key); }
                }
            }
            self.strings = Arc::new(new_strings);

            let mut new_lists = (*self.lists).clone();
            for (key, value_opt) in committed_layer.lists {
                match value_opt {
                    Some(value) => { new_lists.insert(key, value); }
                    None => { new_lists.remove(&key); }
                }
            }
            self.lists = Arc::new(new_lists);
        } else {
            // This is a nested transaction, merge changes into the parent transaction
            let parent_layer = self.transaction_stack.last_mut().unwrap();
            for (key, value_opt) in committed_layer.strings {
                parent_layer.strings.insert(key, value_opt);
            }
            for (key, value_opt) in committed_layer.lists {
                parent_layer.lists.insert(key, value_opt);
            }
        }

        Ok(())
    }

    pub fn rollback_transaction(&mut self) -> Result<(), String> {
        if self.transaction_stack.is_empty() {
            return Err("No active transaction to rollback".to_string());
        }
        self.transaction_stack.pop();
        Ok(())
    }

    pub fn set(&mut self, key: String, value: String) {
        if let Some(layer) = self.transaction_stack.last_mut() {
            layer.strings.insert(key, Some(value));
        } else {
            Arc::make_mut(&mut self.strings).insert(key, value);
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        for layer in self.transaction_stack.iter().rev() {
            if let Some(value_opt) = layer.strings.get(key) {
                return value_opt.clone();
            }
        }
        self.strings.get(key).cloned()
    }

    pub fn del(&mut self, key: &str) -> bool {
        if let Some(layer) = self.transaction_stack.last_mut() {
            layer.strings.insert(key.to_string(), None);
            layer.lists.insert(key.to_string(), None);
            true
        } else {
            Arc::make_mut(&mut self.strings).remove(key).is_some() ||
            Arc::make_mut(&mut self.lists).remove(key).is_some()
        }
    }

    pub fn incr(&mut self, key: &str) -> i64 {
        let value = self.get_or_insert_string(key, "0".to_string());
        let mut num: i64 = value.parse().unwrap_or(0);
        num += 1;
        *value = num.to_string();
        num
    }

    pub fn decr(&mut self, key: &str) -> i64 {
        let value = self.get_or_insert_string(key, "0".to_string());
        let mut num: i64 = value.parse().unwrap_or(0);
        num -= 1;
        *value = num.to_string();
        num
    }

    pub fn lpush(&mut self, key: &str, value: String) -> usize {
        let list = self.get_or_insert_list(key);
        list.push_front(value);
        list.len()
    }

    pub fn rpush(&mut self, key: &str, value: String) -> usize {
        let list = self.get_or_insert_list(key);
        list.push_back(value);
        list.len()
    }

    pub fn lpop(&mut self, key: &str) -> Option<String> {
        self.get_or_insert_list(key).pop_front()
    }

    pub fn rpop(&mut self, key: &str) -> Option<String> {
        self.get_or_insert_list(key).pop_back()
    }

    pub fn llen(&self, key: &str) -> usize {
        for layer in self.transaction_stack.iter().rev() {
            if let Some(Some(list)) = layer.lists.get(key) {
                return list.len();
            }
        }
        self.lists.get(key).map_or(0, |list| list.len())
    }

    fn get_or_insert_string(&mut self, key: &str, default: String) -> &mut String {
        if let Some(layer) = self.transaction_stack.last_mut() {
            layer.strings.entry(key.to_string())
                .or_insert_with(|| self.strings.get(key).cloned())
                .get_or_insert(default)
        } else {
            Arc::make_mut(&mut self.strings)
                .entry(key.to_string())
                .or_insert(default)
        }
    }

    fn get_or_insert_list(&mut self, key: &str) -> &mut VecDeque<String> {
        if let Some(layer) = self.transaction_stack.last_mut() {
            layer.lists.entry(key.to_string())
                .or_insert_with(|| self.lists.get(key).cloned())
                .get_or_insert_with(VecDeque::new)
        } else {
            Arc::make_mut(&mut self.lists)
                .entry(key.to_string())
                .or_insert_with(VecDeque::new)
        }
    }
}
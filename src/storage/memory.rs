use std::collections::{HashMap, VecDeque};

pub struct MemoryStorage {
    strings: HashMap<String, String>,
    lists: HashMap<String, VecDeque<String>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        MemoryStorage {
            strings: HashMap::new(),
            lists: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: String, value: String) {
        self.strings.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.strings.get(key)
    }

    pub fn del(&mut self, key: &str) -> bool {
        self.strings.remove(key).is_some() || self.lists.remove(key).is_some()
    }

    pub fn incr(&mut self, key: &str) -> i64 {
        let value = self.strings.entry(key.to_string()).or_insert("0".to_string());
        let mut num: i64 = value.parse().unwrap_or(0);
        num += 1;
        *value = num.to_string();
        num
    }

    pub fn decr(&mut self, key: &str) -> i64 {
        let value = self.strings.entry(key.to_string()).or_insert("0".to_string());
        let mut num: i64 = value.parse().unwrap_or(0);
        num -= 1;
        *value = num.to_string();
        num
    }

    pub fn lpush(&mut self, key: &str, value: String) -> usize {
        let list = self.lists.entry(key.to_string()).or_insert_with(VecDeque::new);
        list.push_front(value);
        list.len()
    }

    pub fn rpush(&mut self, key: &str, value: String) -> usize {
        let list = self.lists.entry(key.to_string()).or_insert_with(VecDeque::new);
        list.push_back(value);
        list.len()
    }

    pub fn lpop(&mut self, key: &str) -> Option<String> {
        self.lists.get_mut(key).and_then(|list| list.pop_front())
    }

    pub fn rpop(&mut self, key: &str) -> Option<String> {
        self.lists.get_mut(key).and_then(|list| list.pop_back())
    }

    pub fn llen(&self, key: &str) -> usize {
        self.lists.get(key).map_or(0, |list| list.len())
    }
}
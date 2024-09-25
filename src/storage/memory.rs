use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

pub struct MemoryStorage {
    strings: Arc<HashMap<String, String>>,
    lists: Arc<HashMap<String, VecDeque<String>>>,
    transaction_strings: Option<HashMap<String, Option<String>>>,
    transaction_lists: Option<HashMap<String, Option<VecDeque<String>>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        MemoryStorage {
            strings: Arc::new(HashMap::new()),
            lists: Arc::new(HashMap::new()),
            transaction_strings: None,
            transaction_lists: None,
        }
    }

    pub fn start_transaction(&mut self) {
        self.transaction_strings = Some(HashMap::new());
        self.transaction_lists = Some(HashMap::new());
    }

    pub fn commit_transaction(&mut self) {
        if let Some(trans_strings) = self.transaction_strings.take() {
            let mut new_strings = (*self.strings).clone();
            for (key, value_opt) in trans_strings {
                if let Some(value) = value_opt {
                    new_strings.insert(key, value);
                } else {
                    new_strings.remove(&key);
                }
            }
            self.strings = Arc::new(new_strings);
        }

        if let Some(trans_lists) = self.transaction_lists.take() {
            let mut new_lists = (*self.lists).clone();
            for (key, value_opt) in trans_lists {
                if let Some(value) = value_opt {
                    new_lists.insert(key, value);
                } else {
                    new_lists.remove(&key);
                }
            }
            self.lists = Arc::new(new_lists);
        }
    }

    pub fn rollback_transaction(&mut self) {
        self.transaction_strings = None;
        self.transaction_lists = None;
    }

    pub fn set(&mut self, key: String, value: String) {
        if let Some(ref mut trans) = self.transaction_strings {
            trans.insert(key, Some(value));
        } else {
            Arc::make_mut(&mut self.strings).insert(key, value);
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        if let Some(ref trans) = self.transaction_strings {
            if let Some(value_opt) = trans.get(key) {
                return value_opt.clone();
            }
        }
        self.strings.get(key).cloned()
    }

    pub fn del(&mut self, key: &str) -> bool {
        let mut deleted = false;
        if let Some(ref mut trans_strings) = self.transaction_strings {
            if trans_strings.insert(key.to_string(), None).is_some() {
                deleted = true;
            }
        } else if Arc::make_mut(&mut self.strings).remove(key).is_some() {
            deleted = true;
        }

        if let Some(ref mut trans_lists) = self.transaction_lists {
            if trans_lists.insert(key.to_string(), None).is_some() {
                deleted = true;
            }
        } else if Arc::make_mut(&mut self.lists).remove(key).is_some() {
            deleted = true;
        }

        deleted
    }

    pub fn incr(&mut self, key: &str) -> i64 {
        let value = if let Some(ref mut trans) = self.transaction_strings {
            trans.entry(key.to_string())
                .or_insert_with(|| self.strings.get(key).cloned())
                .get_or_insert_with(|| "0".to_string())
        } else {
            Arc::make_mut(&mut self.strings)
                .entry(key.to_string())
                .or_insert_with(|| "0".to_string())
        };

        let mut num: i64 = value.parse().unwrap_or(0);
        num += 1;
        *value = num.to_string();
        num
    }

    pub fn decr(&mut self, key: &str) -> i64 {
        let value = if let Some(ref mut trans) = self.transaction_strings {
            trans.entry(key.to_string())
                .or_insert_with(|| self.strings.get(key).cloned())
                .get_or_insert_with(|| "0".to_string())
        } else {
            Arc::make_mut(&mut self.strings)
                .entry(key.to_string())
                .or_insert_with(|| "0".to_string())
        };

        let mut num: i64 = value.parse().unwrap_or(0);
        num -= 1;
        *value = num.to_string();
        num
    }

    pub fn lpush(&mut self, key: &str, value: String) -> usize {
        let list = if let Some(ref mut trans) = self.transaction_lists {
            trans.entry(key.to_string())
                .or_insert_with(|| self.lists.get(key).cloned())
                .get_or_insert_with(VecDeque::new)
        } else {
            Arc::make_mut(&mut self.lists)
                .entry(key.to_string())
                .or_insert_with(VecDeque::new)
        };

        list.push_front(value);
        list.len()
    }

    pub fn rpush(&mut self, key: &str, value: String) -> usize {
        let list = if let Some(ref mut trans) = self.transaction_lists {
            trans.entry(key.to_string())
                .or_insert_with(|| self.lists.get(key).cloned())
                .get_or_insert_with(VecDeque::new)
        } else {
            Arc::make_mut(&mut self.lists)
                .entry(key.to_string())
                .or_insert_with(VecDeque::new)
        };

        list.push_back(value);
        list.len()
    }

    pub fn lpop(&mut self, key: &str) -> Option<String> {
        if let Some(ref mut trans) = self.transaction_lists {
            trans.entry(key.to_string())
                .or_insert_with(|| self.lists.get(key).cloned())
                .as_mut()
                .and_then(|list| list.pop_front())
        } else {
            Arc::make_mut(&mut self.lists)
                .get_mut(key)
                .and_then(|list| list.pop_front())
        }
    }

    pub fn rpop(&mut self, key: &str) -> Option<String> {
        if let Some(ref mut trans) = self.transaction_lists {
            trans.entry(key.to_string())
                .or_insert_with(|| self.lists.get(key).cloned())
                .as_mut()
                .and_then(|list| list.pop_back())
        } else {
            Arc::make_mut(&mut self.lists)
                .get_mut(key)
                .and_then(|list| list.pop_back())
        }
    }

    pub fn llen(&self, key: &str) -> usize {
        if let Some(ref trans) = self.transaction_lists {
            if let Some(Some(list)) = trans.get(key) {
                return list.len();
            }
        }
        self.lists.get(key).map_or(0, |list| list.len())
    }
}
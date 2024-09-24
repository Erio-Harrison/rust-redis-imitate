use std::collections::HashMap;

pub struct MemoryStorage {
    data: HashMap<String, String>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        MemoryStorage{
            data: HashMap::new(),
        }
    }

    pub fn set(&mut self, key:String, value: String){
        self.data.insert(key, value);
    }

    pub fn get(& self, key:&str) -> Option<&String>{
        self.data.get(key)
    }

    pub fn del(& mut self, key:&str) -> bool{
        self.data.remove(key).is_some()
    }
}
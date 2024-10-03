use redis_clone::storage::memory::MemoryStorage;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_operations() {
        let mut storage = MemoryStorage::new();
        
        storage.set("key1".to_string(), "value1".to_string());
        assert_eq!(storage.get("key1"), Some("value1".to_string()));
        
        storage.set("key1".to_string(), "new_value1".to_string());
        assert_eq!(storage.get("key1"), Some("new_value1".to_string()));
        
        assert_eq!(storage.get("KEY1"), Some("new_value1".to_string()));
        
        assert_eq!(storage.get("nonexistent"), None);
    }

    #[test]
    fn test_delete_operation() {
        let mut storage = MemoryStorage::new();
        
        storage.set("key1".to_string(), "value1".to_string());
        assert_eq!(storage.del("key1"), true);
        assert_eq!(storage.get("key1"), None);
        
        assert_eq!(storage.del("key1"), false);
        
        storage.set("KeyToDelete".to_string(), "value".to_string());
        assert_eq!(storage.del("keytodelete"), true);
        assert_eq!(storage.get("KeyToDelete"), None);
    }

    #[test]
    fn test_increment_decrement() {
        let mut storage = MemoryStorage::new();
        
        assert_eq!(storage.incr("counter"), 1);
        
        assert_eq!(storage.incr("counter"), 2);
        
        assert_eq!(storage.decr("counter"), 1);
        
        assert_eq!(storage.decr("counter"), 0);
        assert_eq!(storage.decr("counter"), -1);
        
        storage.set("non_numeric".to_string(), "abc".to_string());
        assert_eq!(storage.incr("non_numeric"), 1);
        assert_eq!(storage.decr("non_numeric"), 0);
    }

    #[test]
    fn test_list_operations() {
        let mut storage = MemoryStorage::new();
        
        assert_eq!(storage.lpush("mylist", "item1".to_string()), 1);
        assert_eq!(storage.rpush("mylist", "item2".to_string()), 2);
        assert_eq!(storage.lpush("mylist", "item0".to_string()), 3);
        
        assert_eq!(storage.llen("mylist"), 3);
        
        assert_eq!(storage.lpop("mylist"), Some("item0".to_string()));
        assert_eq!(storage.rpop("mylist"), Some("item2".to_string()));
        assert_eq!(storage.llen("mylist"), 1);
        
        assert_eq!(storage.lpop("mylist"), Some("item1".to_string()));
        assert_eq!(storage.lpop("mylist"), None);
        assert_eq!(storage.rpop("mylist"), None);
        
        assert_eq!(storage.llen("nonexistent"), 0);
        assert_eq!(storage.lpop("nonexistent"), None);
        assert_eq!(storage.rpop("nonexistent"), None);
    }

    #[test]
    fn test_transactions() {
        let mut storage = MemoryStorage::new();
        
        storage.start_transaction();
        
        storage.set("key1".to_string(), "value1".to_string());
        storage.lpush("list1", "item1".to_string());
        
        let results = storage.commit_transaction().unwrap();
        assert_eq!(results, vec!["OK".to_string(), "1".to_string()]);
        
        assert_eq!(storage.get("key1"), Some("value1".to_string()));
        assert_eq!(storage.llen("list1"), 1);
        
        storage.start_transaction();
        storage.set("key2".to_string(), "value2".to_string());
        storage.start_transaction();
        storage.set("key3".to_string(), "value3".to_string());
        let inner_results = storage.commit_transaction().unwrap();
        assert_eq!(inner_results, vec!["QUEUED".to_string()]);
        let outer_results = storage.commit_transaction().unwrap();
        assert_eq!(outer_results, vec!["OK".to_string(), "OK".to_string()]);
        
        storage.start_transaction();
        storage.set("key4".to_string(), "value4".to_string());
        storage.rollback_transaction().unwrap();
        assert_eq!(storage.get("key4"), None);
    }
}
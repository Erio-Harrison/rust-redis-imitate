use redis_clone::storage::memory::MemoryStorage;

#[test]
fn test_storage_integration() {
    let mut storage = MemoryStorage::new();
    storage.set("key1".to_string(), "value1".to_string());
    assert_eq!(storage.get("key1"), Some("value1".to_string()));

    storage.set("key2".to_string(),"value2".to_string());
    storage.set("key2".to_string(),"value2_new".to_string());
    assert_ne!(storage.get("key2"),Some("value2".to_string()));
    assert_eq!(storage.get("key2"),Some("value2_new".to_string()));
}

#[test]
fn test_storage_delete() {
    let mut storage = MemoryStorage::new();
    storage.set("key1".to_string(), "value1".to_string());
    assert_eq!(storage.del("key1"), true);
    assert_eq!(storage.get("key1"), None);
    assert_eq!(storage.del("key1"), false);
}
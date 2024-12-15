use redis_imitate::cache::avlcache::AVLCache;
use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put_and_get() {
        let mut cache = AVLCache::new(5, Duration::from_secs(60));
        cache.put("key1".to_string(), 1);
        cache.put("key2".to_string(), 2);
        cache.put("key3".to_string(), 3);

        assert_eq!(cache.get(&"key1".to_string()), Some(1));
        assert_eq!(cache.get(&"key2".to_string()), Some(2));
        assert_eq!(cache.get(&"key3".to_string()), Some(3));
        assert_eq!(cache.get(&"key4".to_string()), None);
    }

    #[test]
    fn test_capacity() {
        let mut cache = AVLCache::new(3, Duration::from_secs(60));
        cache.put("key1".to_string(), 1);
        cache.put("key2".to_string(), 2);
        cache.put("key3".to_string(), 3);
        cache.put("key4".to_string(), 4);

        assert_eq!(cache.get(&"key1".to_string()), None);
        assert_eq!(cache.get(&"key2".to_string()), Some(2));
        assert_eq!(cache.get(&"key3".to_string()), Some(3));
        assert_eq!(cache.get(&"key4".to_string()), Some(4));
    }

    #[test]
    fn test_remove() {
        let mut cache = AVLCache::new(5, Duration::from_secs(60));
        cache.put("key1".to_string(), 1);
        cache.put("key2".to_string(), 2);

        assert_eq!(cache.remove(&"key1".to_string()), Some(1));
        assert_eq!(cache.get(&"key1".to_string()), None);
        assert_eq!(cache.get(&"key2".to_string()), Some(2));
    }

    #[test]
    fn test_clear() {
        let mut cache = AVLCache::new(5, Duration::from_secs(60));
        cache.put("key1".to_string(), 1);
        cache.put("key2".to_string(), 2);

        cache.clear();
        assert_eq!(cache.get(&"key1".to_string()), None);
        assert_eq!(cache.get(&"key2".to_string()), None);
    }

    #[test]
    fn test_ttl() {
        let mut cache = AVLCache::new(5, Duration::from_millis(100));
        cache.put("key1".to_string(), 1);

        assert_eq!(cache.get(&"key1".to_string()), Some(1));

        std::thread::sleep(Duration::from_millis(150));

        assert_eq!(cache.get(&"key1".to_string()), None);
    }

    #[test]
    fn test_update_existing_key() {
        let mut cache = AVLCache::new(5, Duration::from_secs(60));
        cache.put("key1".to_string(), 1);
        cache.put("key1".to_string(), 2);

        assert_eq!(cache.get(&"key1".to_string()), Some(2));
    }

    #[test]
    fn test_large_capacity() {
        let mut cache = AVLCache::new(1_000_000, Duration::from_secs(60));
        for i in 0..1_000_000 {
            cache.put(format!("key{}", i), i);
        }
        assert_eq!(cache.get(&"key999999".to_string()), Some(999999));
        cache.put("new_key".to_string(), 1_000_000);
        assert_eq!(cache.get(&"key0".to_string()), None);
    }

    #[test]
    fn test_zero_ttl() {
        let mut cache = AVLCache::new(5, Duration::from_secs(0));
        cache.put("key1".to_string(), 1);
        assert_eq!(cache.get(&"key1".to_string()), None);
    }

    #[test]
    fn test_update_resets_ttl() {
        let mut cache = AVLCache::new(5, Duration::from_millis(200));
        cache.put("key1".to_string(), 1);

        std::thread::sleep(Duration::from_millis(150));
        cache.put("key1".to_string(), 2);

        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(cache.get(&"key1".to_string()), Some(2));
    }
}
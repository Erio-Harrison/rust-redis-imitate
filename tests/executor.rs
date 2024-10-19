use redis_clone::storage::memory::MemoryStorage;
use redis_clone::commands::executor::CommandExecutor;
use redis_clone::commands::parser::Command;
use std::sync::Arc;
use std::sync::Mutex;

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> CommandExecutor {
        let storage = Arc::new(Mutex::new(MemoryStorage::new()));
        CommandExecutor::new(storage)
    }

    #[test]
    fn test_set_and_get() {
        let executor = setup();
        
        assert_eq!(executor.execute_command(Command::Set("key1".to_string(), "value1".to_string())), "OK".to_string());
        assert_eq!(executor.execute_command(Command::Get("key1".to_string())), "value1".to_string());
        assert_eq!(executor.execute_command(Command::Get("nonexistent".to_string())), "(nil)".to_string());
    }

    #[test]
    fn test_del() {
        let executor = setup();
        
        executor.execute_command(Command::Set("key1".to_string(), "value1".to_string()));
        assert_eq!(executor.execute_command(Command::Del("key1".to_string())), "1".to_string());
        assert_eq!(executor.execute_command(Command::Get("key1".to_string())), "(nil)".to_string());
        assert_eq!(executor.execute_command(Command::Del("nonexistent".to_string())), "0".to_string());
    }

    #[test]
    fn test_incr_and_decr() {
        let executor = setup();
        
        assert_eq!(executor.execute_command(Command::Incr("counter".to_string())), "1".to_string());
        assert_eq!(executor.execute_command(Command::Incr("counter".to_string())), "2".to_string());
        assert_eq!(executor.execute_command(Command::Decr("counter".to_string())), "1".to_string());
        assert_eq!(executor.execute_command(Command::Decr("counter".to_string())), "0".to_string());
    }

    #[test]
    fn test_list_operations() {
        let executor = setup();
        
        assert_eq!(executor.execute_command(Command::LPush("list".to_string(), "item1".to_string())), "1".to_string());
        assert_eq!(executor.execute_command(Command::RPush("list".to_string(), "item2".to_string())), "2".to_string());
        assert_eq!(executor.execute_command(Command::LLen("list".to_string())), "2".to_string());
        assert_eq!(executor.execute_command(Command::LPop("list".to_string())), "item1".to_string());
        assert_eq!(executor.execute_command(Command::RPop("list".to_string())), "item2".to_string());
        assert_eq!(executor.execute_command(Command::LPop("list".to_string())), "(nil)".to_string());
    }

    #[test]
    fn test_unknown_command() {
        let executor = setup();
        
        assert_eq!(executor.execute_command(Command::Unknown("unknown".to_string())), "ERR unknown command 'unknown'".to_string());
    }

    #[test]
    fn test_transaction_discard() {
        let executor = setup();
        
        assert_eq!(executor.execute_command(Command::Multi), "OK".to_string());
        executor.execute_command(Command::Set("key1".to_string(), "value1".to_string()));
        assert_eq!(executor.execute_command(Command::Discard), "OK".to_string());
        assert_eq!(executor.execute_command(Command::Get("key1".to_string())), "(nil)".to_string());
    }
}
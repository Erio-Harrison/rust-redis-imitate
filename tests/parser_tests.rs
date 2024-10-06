use redis_clone::commands::parser::{Command,CommandParser};
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_command() {
        assert_eq!(
            CommandParser::parse("SET mykey myvalue"),
            Command::Set("mykey".to_string(), "myvalue".to_string())
        );
        assert_eq!(
            CommandParser::parse("set MYKEY MYVALUE"),
            Command::Set("mykey".to_string(), "MYVALUE".to_string())
        );
    }

    #[test]
    fn test_get_command() {
        assert_eq!(
            CommandParser::parse("GET mykey"),
            Command::Get("mykey".to_string())
        );
        assert_eq!(
            CommandParser::parse("get MYKEY"),
            Command::Get("mykey".to_string())
        );
    }

    #[test]
    fn test_del_command() {
        assert_eq!(
            CommandParser::parse("DEL mykey"),
            Command::Del("mykey".to_string())
        );
    }

    #[test]
    fn test_incr_command() {
        assert_eq!(
            CommandParser::parse("INCR counter"),
            Command::Incr("counter".to_string())
        );
    }

    #[test]
    fn test_decr_command() {
        assert_eq!(
            CommandParser::parse("DECR counter"),
            Command::Decr("counter".to_string())
        );
    }

    #[test]
    fn test_lpush_command() {
        assert_eq!(
            CommandParser::parse("LPUSH mylist value"),
            Command::LPush("mylist".to_string(), "value".to_string())
        );
    }

    #[test]
    fn test_rpush_command() {
        assert_eq!(
            CommandParser::parse("RPUSH mylist value"),
            Command::RPush("mylist".to_string(), "value".to_string())
        );
    }

    #[test]
    fn test_lpop_command() {
        assert_eq!(
            CommandParser::parse("LPOP mylist"),
            Command::LPop("mylist".to_string())
        );
    }

    #[test]
    fn test_rpop_command() {
        assert_eq!(
            CommandParser::parse("RPOP mylist"),
            Command::RPop("mylist".to_string())
        );
    }

    #[test]
    fn test_llen_command() {
        assert_eq!(
            CommandParser::parse("LLEN mylist"),
            Command::LLen("mylist".to_string())
        );
    }

    #[test]
    fn test_multi_command() {
        assert_eq!(CommandParser::parse("MULTI"), Command::Multi);
    }

    #[test]
    fn test_exec_command() {
        assert_eq!(CommandParser::parse("EXEC"), Command::Exec);
    }

    #[test]
    fn test_discard_command() {
        assert_eq!(CommandParser::parse("DISCARD"), Command::Discard);
    }

    #[test]
    fn test_unknown_command() {
        assert_eq!(
            CommandParser::parse("UNKNOWN command"),
            Command::Unknown("UNKNOWN command".to_string())
        );
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(
            CommandParser::parse(""),
            Command::Unknown("".to_string())
        );
    }

    #[test]
    fn test_case_insensitivity() {
        assert_eq!(
            CommandParser::parse("set mykey myvalue"),
            Command::Set("mykey".to_string(), "myvalue".to_string())
        );
        assert_eq!(
            CommandParser::parse("GET mykey"),
            Command::Get("mykey".to_string())
        );
    }

    #[test]
    fn test_extra_whitespace() {
        assert_eq!(
            CommandParser::parse("  SET    mykey    myvalue  "),
            Command::Set("mykey".to_string(), "myvalue".to_string())
        );
    }

    #[test]
    fn test_incorrect_argument_count() {
        assert_eq!(
            CommandParser::parse("SET key"),
            Command::Unknown("SET key".to_string())
        );
        assert_eq!(
            CommandParser::parse("GET"),
            Command::Unknown("GET".to_string())
        );
        assert_eq!(
            CommandParser::parse("INCR key value"),
            Command::Unknown("INCR key value".to_string())
        );
    }
}
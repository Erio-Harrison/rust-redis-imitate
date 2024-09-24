#[derive(Debug,PartialEq)]
pub enum Command {
    Set(String, String),
    Get(String),
    Del(String),
    Incr(String),
    Decr(String),
    LPush(String, String),
    RPush(String, String),
    LPop(String),
    RPop(String),
    LLen(String),
    Unknown(String),
}
pub struct CommandParser;

impl CommandParser {
    pub fn parse(input: &str) -> Command {
        let parts: Vec<&str> = input.trim().splitn(3, ' ').collect();
        match parts.as_slice() {
            ["SET", key, value] => Command::Set(key.to_string(), value.to_string()),
            ["GET", key] => Command::Get(key.to_string()),
            ["DEL", key] => Command::Del(key.to_string()),
            ["INCR", key] => Command::Incr(key.to_string()),
            ["DECR", key] => Command::Decr(key.to_string()),
            ["LPUSH", key, value] => Command::LPush(key.to_string(), value.to_string()),
            ["RPUSH", key, value] => Command::RPush(key.to_string(), value.to_string()),
            ["LPOP", key] => Command::LPop(key.to_string()),
            ["RPOP", key] => Command::RPop(key.to_string()),
            ["LLEN", key] => Command::LLen(key.to_string()),
            [command, ..] => Command::Unknown(command.to_string()),
            _ => Command::Unknown("".to_string()),
        }
    }
}
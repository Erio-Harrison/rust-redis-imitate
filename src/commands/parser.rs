#[derive(Debug,PartialEq,Clone)]
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
    Multi,
    Exec,
    Discard,
    Unknown(String),
}
pub struct CommandParser;

impl CommandParser {
    pub fn parse(input: &str) -> Command {
        let parts: Vec<&str> = input.trim().split_whitespace().collect();
        match parts.as_slice() {
            [command, rest @ ..] => match command.to_uppercase().as_str() {
                "SET" if rest.len() == 2 => Command::Set(rest[0].to_lowercase(), rest[1].to_string()),
                "GET" if rest.len() == 1 => Command::Get(rest[0].to_lowercase()),
                "DEL" if rest.len() == 1 => Command::Del(rest[0].to_lowercase()),
                "INCR" if rest.len() == 1 => Command::Incr(rest[0].to_lowercase()),
                "DECR" if rest.len() == 1 => Command::Decr(rest[0].to_lowercase()),
                "LPUSH" if rest.len() == 2 => Command::LPush(rest[0].to_lowercase(), rest[1].to_string()),
                "RPUSH" if rest.len() == 2 => Command::RPush(rest[0].to_lowercase(), rest[1].to_string()),
                "LPOP" if rest.len() == 1 => Command::LPop(rest[0].to_lowercase()),
                "RPOP" if rest.len() == 1 => Command::RPop(rest[0].to_lowercase()),
                "LLEN" if rest.len() == 1 => Command::LLen(rest[0].to_lowercase()),
                "MULTI" if rest.is_empty() => Command::Multi,
                "EXEC" if rest.is_empty() => Command::Exec,
                "DISCARD" if rest.is_empty() => Command::Discard,
                _ => Command::Unknown(input.to_string()),
            },
            _ => Command::Unknown("".to_string()),
        }
    }
}
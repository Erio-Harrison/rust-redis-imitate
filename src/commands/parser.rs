#[derive(Debug,PartialEq)]
pub enum Command{
    Set(String,String),
    Get(String),
    Del(String),
    Unknown(String),
}

pub struct CommandParser;

impl CommandParser{
    pub fn parse(input:String)->Command{
        let parts: Vec<&str> = input.trim().splitn(3, ' ').collect();
        match parts.as_slice() {
            ["SET", key, value] => Command::Set(key.to_string(), value.to_string()),
            ["GET", key] => Command::Get(key.to_string()),
            ["DEL", key] => Command::Del(key.to_string()),
            [command, ..] => Command::Unknown(command.to_string()),
            _ => Command::Unknown("".to_string()),
        }
    }
}
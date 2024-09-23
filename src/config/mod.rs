pub struct Config {
    pub host: String,
    pub port: u16,
    // 其他配置项...
}

impl Config {
    pub fn new() -> Self {
        Config {
            host: "127.0.0.1".to_string(),
            port: 6379,
        }
    }
}

use crate::config::Config;

pub struct Server {
    config: Config,
}

impl Server {
    pub fn new(config: Config) -> Self {
        Server { config }
    }

    pub fn run(&self) {
        println!("Server running on {}:{}", self.config.host, self.config.port);
    }
}
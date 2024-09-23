mod network;
mod commands;
mod storage;
mod cache;
mod config;
mod logging;
mod monitoring;
mod cluster;
mod transactions;
mod pubsub;

use crate::network::server::Server;
use crate::config::Config;

fn main() {
    let config = Config::new();
    let server = Server::new(config);
    server.run();
}
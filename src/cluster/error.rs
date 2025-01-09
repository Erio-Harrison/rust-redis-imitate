// src/cluster/error.rs
use thiserror::Error;
use std::io;

#[derive(Error, Debug)]
pub enum RaftError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    
    #[error("Invalid term: current {current}, received {received}")]
    InvalidTerm {
        current: u64,
        received: u64,
    },
    
    #[error("Node {0} not found")]
    NodeNotFound(String),
    
    #[error("Log entry at index {0} not found")]
    LogNotFound(u64),
    
    #[error("Log compaction in progress")]
    LogCompactionInProgress,
    
    #[error("Transport error: {0}")]
    Transport(String),
    
    #[error("State error: {0}")]
    State(String),
}

pub type RaftResult<T> = Result<T, RaftError>;
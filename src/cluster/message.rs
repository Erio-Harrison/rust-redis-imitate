// src/cluster/message.rs
use serde::{Serialize, Deserialize};
use std::time::SystemTime;
//use crate::cluster::error::RaftResult;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogEntry {
    pub term: u64,
    pub index: u64,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

impl LogEntry {
    pub fn new(term: u64, index: u64, data: Vec<u8>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        LogEntry {
            term,
            index,
            data,
            timestamp,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RaftMessage {
    // Leader选举相关
    RequestVote {
        term: u64,
        candidate_id: String,
        last_log_index: u64,
        last_log_term: u64,
    },
    
    RequestVoteResponse {
        term: u64,
        vote_granted: bool,
    },
    
    // 日志复制相关
    AppendEntries {
        term: u64,
        leader_id: String,
        prev_log_index: u64,
        prev_log_term: u64,
        entries: Vec<LogEntry>,
        leader_commit: u64,
    },
    
    AppendEntriesResponse {
        term: u64,
        success: bool,
        match_index: u64,
    },

    Heartbeat {
        term: u64,
        leader_id: String,
    },
}
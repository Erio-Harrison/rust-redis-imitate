use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use rand::Rng;
use super::error::{RaftError, RaftResult};

// Raft node role
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeRole {
    Follower,
    Candidate,
    Leader,
}

// Raft configuration
#[derive(Debug, Clone)]
pub struct RaftConfig {
    pub election_timeout_min: u64,     // Minimum election timeout (ms)
    pub election_timeout_max: u64,     // Maximum election timeout (ms)
    pub heartbeat_interval: u64,       // Heartbeat interval (ms)
}

impl Default for RaftConfig {
    fn default() -> Self {
        RaftConfig {
            election_timeout_min: 150,
            election_timeout_max: 300,
            heartbeat_interval: 50,
        }
    }
}

// Raft state machine
pub struct RaftState {
    // Basic information
    pub node_id: String,               // Node ID
    pub current_term: u64,             // Current term
    pub voted_for: Option<String>,     // Node ID that received the vote in the current term
    pub role: NodeRole,                // Current role
    
    // Election-related
    pub votes_received: u64,           // Number of votes received
    pub election_timeout: Duration,    // Current election timeout
    pub last_election_time: Instant,   // Last election time
    
    // Heartbeat-related
    pub last_heartbeat: Instant,       // Last heartbeat time
    pub heartbeat_interval: Duration,  // Heartbeat interval
    
    // Log-related
    pub last_log_index: u64,           // Index of the last log entry
    pub last_log_term: u64,            // Term of the last log entry
    pub commit_index: u64,             // Index of the highest log entry committed
    pub last_applied: u64,             // Index of the last log entry applied to the state machine
    
    // Configuration
    config: RaftConfig,
}

impl RaftState {
    pub fn new(node_id: String, config: Option<RaftConfig>) -> Self {
        let config = config.unwrap_or_default();
        let now = Instant::now();
        
        RaftState {
            node_id,
            current_term: 0,
            voted_for: None,
            role: NodeRole::Follower,
            
            votes_received: 0,
            election_timeout: Self::random_election_timeout(&config),
            last_election_time: now,
            
            last_heartbeat: now,
            heartbeat_interval: Duration::from_millis(config.heartbeat_interval),
            
            last_log_index: 0,
            last_log_term: 0,
            commit_index: 0,
            last_applied: 0,
            
            config,
        }
    }
    
    // Generate a random election timeout
    fn random_election_timeout(config: &RaftConfig) -> Duration {
        let mut rng = rand::thread_rng();
        let timeout = rng.gen_range(
            config.election_timeout_min..=config.election_timeout_max
        );
        Duration::from_millis(timeout)
    }
    
    // Reset the election timeout
    pub fn reset_election_timeout(&mut self) {
        self.election_timeout = Self::random_election_timeout(&self.config);
        self.last_election_time = Instant::now();
    }
    
    // Update the term
    pub fn update_term(&mut self, term: u64) -> RaftResult<()> {
        if term < self.current_term {
            return Err(RaftError::InvalidTerm {
                current: self.current_term,
                received: term,
            });
        }
        
        if term > self.current_term {
            self.current_term = term;
            self.voted_for = None;
            self.role = NodeRole::Follower;
        }
        
        Ok(())
    }
    
    // Check if an election should be started
    pub fn should_begin_election(&self) -> bool {
        self.role != NodeRole::Leader 
            && self.last_election_time.elapsed() >= self.election_timeout
    }
    
    // Start an election
    pub fn begin_election(&mut self) {
        self.role = NodeRole::Candidate;
        self.current_term += 1;
        self.voted_for = Some(self.node_id.clone());
        self.votes_received = 1;
        self.reset_election_timeout();
    }
    
    // Handle a vote request
    pub fn handle_vote_request(
        &mut self,
        candidate_id: &str,
        term: u64,
        last_log_index: u64,
        last_log_term: u64
    ) -> RaftResult<bool> {
        // If the request term is less than the current term, reject the vote
        if term < self.current_term {
            return Ok(false);
        }
        
        // Update the term
        self.update_term(term)?;
        
        // Check if the node has already voted or if the log is up-to-date
        let can_vote = self.voted_for.is_none() || self.voted_for.as_deref() == Some(candidate_id);
        let log_is_current = last_log_term > self.last_log_term || 
            (last_log_term == self.last_log_term && last_log_index >= self.last_log_index);
            
        if can_vote && log_is_current {
            self.voted_for = Some(candidate_id.to_string());
            self.reset_election_timeout();
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    // Receive a vote response
    pub fn receive_vote(&mut self, granted: bool) {
        if granted && self.role == NodeRole::Candidate {
            self.votes_received += 1;
        }
    }
    
    // Check if the election is won
    pub fn check_election_won(&self, cluster_size: usize) -> bool {
        self.role == NodeRole::Candidate && 
        self.votes_received > ((cluster_size / 2)).try_into().unwrap()
    }
    
    // Transition to Leader
    pub fn become_leader(&mut self) {
        if self.role == NodeRole::Candidate {
            self.role = NodeRole::Leader;
            self.last_heartbeat = Instant::now();
        }
    }
    
    // Check if a heartbeat should be sent
    pub fn should_send_heartbeat(&self) -> bool {
        self.role == NodeRole::Leader && 
        self.last_heartbeat.elapsed() >= self.heartbeat_interval
    }
    
    // Update the heartbeat time
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn setup_test_state() -> RaftState {
        RaftState::new(
            "node1".to_string(),
            Some(RaftConfig {
                election_timeout_min: 150,
                election_timeout_max: 300,
                heartbeat_interval: 50,
            })
        )
    }
    
    #[test]
    fn test_initial_state() {
        let state = setup_test_state();
        assert_eq!(state.role, NodeRole::Follower);
        assert_eq!(state.current_term, 0);
        assert_eq!(state.voted_for, None);
    }
    
    #[test]
    fn test_begin_election() {
        let mut state = setup_test_state();
        state.begin_election();
        
        assert_eq!(state.role, NodeRole::Candidate);
        assert_eq!(state.current_term, 1);
        assert_eq!(state.voted_for, Some("node1".to_string()));
        assert_eq!(state.votes_received, 1);
    }
    
    #[test]
    fn test_vote_request() {
        let mut state = setup_test_state();
        
        // First vote request
        let result = state.handle_vote_request("node2", 1, 0, 0).unwrap();
        assert!(result);
        assert_eq!(state.voted_for, Some("node2".to_string()));
        
        // Repeated vote request
        let result = state.handle_vote_request("node3", 1, 0, 0).unwrap();
        assert!(!result);
        assert_eq!(state.voted_for, Some("node2".to_string()));
    }
}
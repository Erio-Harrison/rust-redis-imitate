use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use std::collections::HashMap;
use std::time::SystemTime;

use crate::cluster::log_store::MockLogStore;
use crate::cluster::transport::MockTransport;

use super::error::{RaftError, RaftResult};
use super::message::{RaftMessage, LogEntry};
use super::state::{RaftState, NodeRole};
use super::consensus::RaftConsensus;
use super::transport::Transport;
use super::log_store::{LogStore, Snapshot};

// Command represents a client request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub operation: String,
    pub key: String,
    pub value: Option<Vec<u8>>,
    pub timestamp: u64,
}

impl Command {
    pub fn new(operation: String, key: String, value: Option<Vec<u8>>) -> Self {
        Command {
            operation,
            key,
            value,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

// Response represents the result of a command execution
#[derive(Debug, Clone)]
pub struct Response {
    pub success: bool,
    pub data: Option<Vec<u8>>,
    pub error: Option<String>,
}

// State machine interface
pub trait StateMachine: Send + Sync {
    fn apply(&mut self, command: &Command) -> RaftResult<Response>;
    fn snapshot(&mut self) -> RaftResult<Vec<u8>>;
    fn restore(&mut self, snapshot: Vec<u8>) -> RaftResult<()>;
}

pub struct RaftNode<T: Transport + 'static, L: LogStore + 'static, S: StateMachine + 'static> {
    node_id: String,
    consensus: Arc<RaftConsensus<T, L>>,
    state_machine: Arc<Mutex<S>>,
    applied_index: Arc<Mutex<u64>>,
    snapshot_threshold: u64,  // Number of logs before taking a snapshot
    last_snapshot_index: Arc<Mutex<u64>>,
}

impl<T: Transport + 'static, L: LogStore + 'static, S: StateMachine + 'static> RaftNode<T, L, S> {
    pub fn new(
        node_id: String,
        transport: Arc<T>,
        log_store: Arc<Mutex<L>>,
        state_machine: Arc<Mutex<S>>,
        cluster: HashMap<String, String>,
        snapshot_threshold: u64,
    ) -> Arc<Self> {
        let consensus = RaftConsensus::new(
            node_id.clone(),
            transport,
            log_store,
            cluster,
        );

        Arc::new(RaftNode {
            node_id,
            consensus,
            state_machine,
            applied_index: Arc::new(Mutex::new(0)),
            snapshot_threshold,
            last_snapshot_index: Arc::new(Mutex::new(0)),
        })
    }

    pub async fn start(node: Arc<Self>) -> RaftResult<()> {
        // Start consensus module
        RaftConsensus::start(Arc::clone(&node.consensus)).await?;
        
        // Start apply loop
        let apply_node = Arc::clone(&node);
        tokio::spawn(async move {
            if let Err(e) = apply_node.run_apply_loop().await {
                eprintln!("Error in apply loop: {}", e);
            }
        });
        
        // Start snapshot manager
        let snapshot_node = Arc::clone(&node);
        tokio::spawn(async move {
            if let Err(e) = snapshot_node.run_snapshot_manager().await {
                eprintln!("Error in snapshot manager: {}", e);
            }
        });
        
        Ok(())
    }

    // Process client request
    pub async fn process_command(&self, command: Command) -> RaftResult<Response> {
        let state = self.consensus.state.lock().await;
        
        // Only leader can process writes
        if state.role != NodeRole::Leader {
            return Err(RaftError::NotLeader);
        }
        
        // Create log entry
        let last_index = self.consensus.log_store.lock().await.last_index()?;
        let entry = LogEntry::new(
            state.current_term,
            last_index + 1,
            bincode::serialize(&command)
                .map_err(|e| RaftError::Serialization(e))?,
        );
        
        // Append to local log
        {
            let mut log_store = self.consensus.log_store.lock().await;
            log_store.append(vec![entry.clone()])?;
        }
        
        // Wait for replication
        let committed = self.wait_for_commit(entry.index).await?;
        if !committed {
            return Err(RaftError::ReplicationTimeout);
        }
        
        // Apply command
        let mut state_machine = self.state_machine.lock().await;
        state_machine.apply(&command)
    }

    // Wait for log entry to be committed
    async fn wait_for_commit(&self, index: u64) -> RaftResult<bool> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(5);
        
        while start.elapsed() < timeout {
            let committed_index = self.consensus.log_store.lock().await.committed_index()?;
            if committed_index >= index {
                return Ok(true);
            }
            sleep(Duration::from_millis(10)).await;
        }
        
        Ok(false)
    }

    // Apply committed log entries to state machine
    async fn run_apply_loop(self: Arc<Self>) -> RaftResult<()> {
        let node = Arc::clone(&self);
        
        tokio::spawn(async move {
            loop {
                if let Err(e) = node.apply_committed_entries().await {
                    eprintln!("Error applying committed entries: {}", e);
                }
                sleep(Duration::from_millis(10)).await;
            }
        });
        
        Ok(())
    }

    async fn apply_committed_entries(&self) -> RaftResult<()> {
        let committed_index = self.consensus.log_store.lock().await.committed_index()?;
        let mut applied_index = self.applied_index.lock().await;
        
        while *applied_index < committed_index {
            let next_index = *applied_index + 1;
            
            // Get log entry
            let entry = match self.consensus.log_store.lock().await.get(next_index)? {
                Some(entry) => entry,
                None => break,
            };
            
            // Deserialize command
            let command: Command = bincode::deserialize(&entry.data)
                .map_err(|e| RaftError::Serialization(e))?;
            
            // Apply to state machine
            let mut state_machine = self.state_machine.lock().await;
            state_machine.apply(&command)?;
            
            *applied_index = next_index;
        }
        
        Ok(())
    }

    // Manage snapshots
    async fn run_snapshot_manager(self: Arc<Self>) -> RaftResult<()> {
        let node = Arc::clone(&self);
        
        tokio::spawn(async move {
            loop {
                if let Err(e) = node.check_snapshot().await {
                    eprintln!("Error managing snapshots: {}", e);
                }
                sleep(Duration::from_secs(60)).await;
            }
        });
        
        Ok(())
    }

    async fn check_snapshot(&self) -> RaftResult<()> {
        let applied_index = *self.applied_index.lock().await;
        let last_snapshot_index = *self.last_snapshot_index.lock().await;
        
        if applied_index - last_snapshot_index >= self.snapshot_threshold {
            self.create_snapshot().await?;
        }
        
        Ok(())
    }

    async fn create_snapshot(&self) -> RaftResult<()> {
        let applied_index = *self.applied_index.lock().await;
        
        // Take state machine snapshot
        let state_machine_data = {
            let mut state_machine = self.state_machine.lock().await;
            state_machine.snapshot()?
        };
        
        // Create log snapshot
        {
            let mut log_store = self.consensus.log_store.lock().await;
            log_store.snapshot()?;
        }
        
        // Update snapshot index
        let mut last_snapshot_index = self.last_snapshot_index.lock().await;
        *last_snapshot_index = applied_index;
        
        Ok(())
    }

    pub async fn restore_from_snapshot(&self, snapshot: Snapshot) -> RaftResult<()> {
        // Restore state machine
        {
            let mut state_machine = self.state_machine.lock().await;
            state_machine.restore(snapshot.data.clone())?;
        }
        
        // Restore log store
        {
            let log_data = bincode::serialize(&snapshot)
                .map_err(|e| RaftError::Serialization(e))?;
                
            let mut log_store = self.consensus.log_store.lock().await;
            log_store.restore_snapshot(log_data)?;
        }
        
        // Update indices
        {
            let mut applied_index = self.applied_index.lock().await;
            let mut last_snapshot_index = self.last_snapshot_index.lock().await;
            *applied_index = snapshot.metadata.last_index;
            *last_snapshot_index = snapshot.metadata.last_index;
        }
        
        Ok(())
    }

    // Message handling
    pub async fn handle_message(&self, message: RaftMessage) -> RaftResult<()> {
        match message {
            RaftMessage::RequestVote { term, candidate_id, last_log_index, last_log_term } => {
                self.consensus.handle_vote_request(
                    candidate_id,
                    term,
                    last_log_index,
                    last_log_term
                ).await
            }
            
            RaftMessage::RequestVoteResponse { term, vote_granted } => {
                self.consensus.handle_vote_response(term, vote_granted).await
            }
            
            RaftMessage::AppendEntries { term, leader_id, prev_log_index, prev_log_term, entries, leader_commit } => {
                self.consensus.handle_append_entries(
                    term,
                    leader_id,
                    prev_log_index,
                    prev_log_term,
                    entries,
                    leader_commit
                ).await
            }
            
            RaftMessage::AppendEntriesResponse { term, success, match_index } => {
                self.consensus.handle_append_entries_response(
                    self.node_id.clone(),
                    term,
                    success,
                    match_index
                ).await
            }
            
            RaftMessage::Heartbeat { term, leader_id } => {
                let mut state = self.consensus.state.lock().await;
                state.update_term(term)?;
                if term >= state.current_term {
                    state.reset_election_timeout();
                }
                Ok(())
            }
        }
    }
}

// Mock state machine implementation for testing
#[derive(Default)]
struct MockStateMachine {
    data: HashMap<String, Vec<u8>>,
}

impl StateMachine for MockStateMachine {
    fn apply(&mut self, command: &Command) -> RaftResult<Response> {
        match command.operation.as_str() {
            "SET" => {
                if let Some(value) = &command.value {
                    self.data.insert(command.key.clone(), value.clone());
                    Ok(Response {
                        success: true,
                        data: None,
                        error: None,
                    })
                } else {
                    Ok(Response {
                        success: false,
                        data: None,
                        error: Some("No value provided for SET operation".to_string()),
                    })
                }
            }
            "GET" => {
                Ok(Response {
                    success: true,
                    data: self.data.get(&command.key).cloned(),
                    error: None,
                })
            }
            _ => Ok(Response {
                success: false,
                data: None,
                error: Some("Unknown operation".to_string()),
            }),
        }
    }

    fn snapshot(&mut self) -> RaftResult<Vec<u8>> {
        Ok(bincode::serialize(&self.data).unwrap())
    }

    fn restore(&mut self, snapshot: Vec<u8>) -> RaftResult<()> {
        self.data = bincode::deserialize(&snapshot).unwrap();
        Ok(())
    }
}

#[tokio::test]
async fn test_node_startup() {
    let node_id = "node1".to_string();
    let transport = Arc::new(MockTransport::new(node_id.clone()));
    let log_store = Arc::new(Mutex::new(MockLogStore::new()));
    let state_machine = Arc::new(Mutex::new(MockStateMachine::default()));
    
    let mut cluster = HashMap::new();
    cluster.insert(node_id.clone(), "addr1".to_string());
    
    let node = RaftNode::new(
        node_id.clone(),
        transport,
        log_store,
        state_machine,
        cluster,
        1000, // snapshot threshold
    );
    
    assert!(RaftNode::start(node).await.is_ok());
}

#[tokio::test]
async fn test_process_command() {
    let node_id = "node1".to_string();
    let transport = Arc::new(MockTransport::new(node_id.clone()));
    let log_store = Arc::new(Mutex::new(MockLogStore::new()));
    let state_machine = Arc::new(Mutex::new(MockStateMachine::default()));
    
    let mut cluster = HashMap::new();
    cluster.insert(node_id.clone(), "addr1".to_string());
    
    let node = RaftNode::new(
        node_id.clone(),
        transport.clone(),
        log_store.clone(),
        state_machine.clone(),
        cluster,
        1000,
    );
    
    // Make node the leader
    {
        let mut state = node.consensus.state.lock().await;
        state.become_leader();
    }
    
    // Test SET command
    let cmd = Command::new(
        "SET".to_string(),
        "key1".to_string(),
        Some(b"value1".to_vec()),
    );
    
    let result = node.process_command(cmd).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.success);
    
    // Verify log entry was created
    let log_store = node.consensus.log_store.lock().await;
    assert_eq!(log_store.last_index().unwrap(), 1);
    
    // Test GET command
    let cmd = Command::new(
        "GET".to_string(),
        "key1".to_string(),
        None,
    );
    
    let result = node.process_command(cmd).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.success);
    assert_eq!(response.data.unwrap(), b"value1");
}

#[tokio::test]
async fn test_snapshot_creation() {
    let node_id = "node1".to_string();
    let transport = Arc::new(MockTransport::new(node_id.clone()));
    let log_store = Arc::new(Mutex::new(MockLogStore::new()));
    let state_machine = Arc::new(Mutex::new(MockStateMachine::default()));
    
    let mut cluster = HashMap::new();
    cluster.insert(node_id.clone(), "addr1".to_string());
    
    let node = RaftNode::new(
        node_id.clone(),
        transport.clone(),
        log_store.clone(),
        state_machine.clone(),
        cluster,
        5, // Low snapshot threshold for testing
    );
    
    // Make node the leader
    {
        let mut state = node.consensus.state.lock().await;
        state.become_leader();
    }
    
    // Create enough entries to trigger snapshot
    for i in 0..6 {
        let cmd = Command::new(
            "SET".to_string(),
            format!("key{}", i),
            Some(format!("value{}", i).into_bytes()),
        );
        let result = node.process_command(cmd).await;
        assert!(result.is_ok());
    }
    
    // Wait for snapshot to be created
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Verify snapshot was created
    let log_store = node.consensus.log_store.lock().await;
    assert!(!log_store.snapshots.is_empty());
}

#[tokio::test]
async fn test_message_handling() {
    let node_id = "node1".to_string();
    let transport = Arc::new(MockTransport::new(node_id.clone()));
    let log_store = Arc::new(Mutex::new(MockLogStore::new()));
    let state_machine = Arc::new(Mutex::new(MockStateMachine::default()));
    
    let mut cluster = HashMap::new();
    cluster.insert(node_id.clone(), "addr1".to_string());
    cluster.insert("node2".to_string(), "addr2".to_string());
    
    let node = RaftNode::new(
        node_id.clone(),
        transport.clone(),
        log_store.clone(),
        state_machine.clone(),
        cluster,
        1000,
    );
    
    // Test vote request handling
    let msg = RaftMessage::RequestVote {
        term: 1,
        candidate_id: "node2".to_string(),
        last_log_index: 0,
        last_log_term: 0,
    };
    
    let result = node.handle_message(msg).await;
    assert!(result.is_ok());
    
    // Verify term was updated
    let state = node.consensus.state.lock().await;
    assert_eq!(state.current_term, 1);
}

#[tokio::test]
async fn test_error_conditions() {
    let node_id = "node1".to_string();
    let transport = Arc::new(MockTransport::new(node_id.clone()));
    let log_store = Arc::new(Mutex::new(MockLogStore::new()));
    let state_machine = Arc::new(Mutex::new(MockStateMachine::default()));
    
    let mut cluster = HashMap::new();
    cluster.insert(node_id.clone(), "addr1".to_string());
    
    let node = RaftNode::new(
        node_id.clone(),
        transport.clone(),
        log_store.clone(),
        state_machine.clone(),
        cluster,
        1000,
    );
    
    // Test processing command when not leader
    let cmd = Command::new(
        "SET".to_string(),
        "key1".to_string(),
        Some(b"value1".to_vec()),
    );
    
    let result = node.process_command(cmd).await;
    assert!(matches!(result, Err(RaftError::NotLeader)));
}
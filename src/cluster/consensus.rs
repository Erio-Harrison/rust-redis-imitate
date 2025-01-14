use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

use super::error::{RaftError, RaftResult};
use super::message::{RaftMessage, LogEntry};
use super::state::{RaftState, NodeRole};
use super::transport::Transport;
use super::log_store::LogStore;

pub struct RaftConsensus<T: Transport + 'static, L: LogStore + 'static> {
    pub state: Arc<Mutex<RaftState>>,
    pub transport: Arc<T>,
    pub log_store: Arc<Mutex<L>>,
    pub cluster: Arc<HashMap<String, String>>, // node_id -> address
    
    pub next_index: Arc<Mutex<HashMap<String, u64>>>,   
    pub match_index: Arc<Mutex<HashMap<String, u64>>>,  
}

impl<T: Transport + 'static, L: LogStore + 'static> RaftConsensus<T, L> {
    pub fn new(
        node_id: String,
        transport: Arc<T>,
        log_store: Arc<Mutex<L>>,
        cluster: HashMap<String, String>,
    ) -> Arc<Self> {
        let consensus = Arc::new(RaftConsensus {
            state: Arc::new(Mutex::new(RaftState::new(node_id, None))),
            transport,
            log_store,
            cluster: Arc::new(cluster),
            next_index: Arc::new(Mutex::new(HashMap::new())),
            match_index: Arc::new(Mutex::new(HashMap::new())),
        });

        consensus
    }

    pub async fn start(self: Arc<Self>) -> RaftResult<()> {
        self.initialize_leader_state().await?;
        
        Arc::clone(&self).run_election_timer().await?;
        
        Arc::clone(&self).run_heartbeat_timer().await?;
        
        Ok(())
    }

    async fn initialize_leader_state(&self) -> RaftResult<()> {
        let last_log_index = self.log_store.lock().await.last_index()?;
        
        let mut next_index = self.next_index.lock().await;
        let mut match_index = self.match_index.lock().await;
        
        for peer_id in self.cluster.keys() {
            next_index.insert(peer_id.clone(), last_log_index + 1);
            match_index.insert(peer_id.clone(), 0);
        }
        
        Ok(())
    }

    async fn handle_election_timeout(&self) -> RaftResult<()> {
        let should_begin;
        {
            let mut state = self.state.lock().await;
            should_begin = state.should_begin_election();
            if should_begin {
                state.begin_election();
            }
        }
        
        if should_begin {
            let last_log_index = self.log_store.lock().await.last_index()?;
            let last_log_term = self.log_store.lock().await.last_term()?;
            let (current_term, node_id) = {
                let state = self.state.lock().await;
                (state.current_term, state.node_id.clone())
            };
            
            let request = RaftMessage::RequestVote {
                term: current_term,
                candidate_id: node_id,
                last_log_index,
                last_log_term,
            };
            
            for peer_id in self.cluster.keys() {
                let transport = Arc::clone(&self.transport);
                let request = request.clone();
                let peer_id = peer_id.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = transport.send(&peer_id, request).await {
                        eprintln!("Failed to send vote request to {}: {}", peer_id, e);
                    }
                });
            }
        }
        
        Ok(())
    }

    pub async fn handle_vote_request(
        &self,
        candidate_id: String,
        term: u64,
        last_log_index: u64,
        last_log_term: u64
    ) -> RaftResult<()> {
        let (vote_granted, current_term) = {
            let mut state = self.state.lock().await;
            let vote_granted = state.handle_vote_request(
                &candidate_id,
                term,
                last_log_index,
                last_log_term
            )?;
            (vote_granted, state.current_term)
        };
        
        let response = RaftMessage::RequestVoteResponse {
            term: current_term,
            vote_granted,
        };
        
        self.transport.send(&candidate_id, response).await?;
        
        Ok(())
    }

    pub async fn handle_vote_response(
        &self,
        term: u64,
        vote_granted: bool
    ) -> RaftResult<()> {
        let need_initialize = {
            let mut state = self.state.lock().await;
            
            if term > state.current_term {
                state.update_term(term)?;
                false
            } else if state.role == NodeRole::Candidate && term == state.current_term {
                state.receive_vote(vote_granted);
                
                // 检查是否获得多数票
                if state.check_election_won(self.cluster.len() + 1) {
                    state.become_leader();
                    true
                } else {
                    false
                }
            } else {
                false
            }
        };

        if need_initialize {
            self.initialize_leader_state().await?;
            self.broadcast_heartbeat().await?;
        }
        
        Ok(())
    }

    pub async fn broadcast_heartbeat(&self) -> RaftResult<()> {
        let heartbeat = {
            let state = self.state.lock().await;
            
            if state.role != NodeRole::Leader {
                return Ok(());
            }
            
            RaftMessage::Heartbeat {
                term: state.current_term,
                leader_id: state.node_id.clone(),
            }
        };
        
        for peer_id in self.cluster.keys() {
            let transport = Arc::clone(&self.transport);
            let heartbeat = heartbeat.clone();
            let peer_id = peer_id.clone();
            
            tokio::spawn(async move {
                if let Err(e) = transport.send(&peer_id, heartbeat).await {
                    eprintln!("Failed to send heartbeat to {}: {}", peer_id, e);
                }
            });
        }
        
        Ok(())
    }

    async fn run_election_timer(self: Arc<Self>) -> RaftResult<()> {
        let consensus = Arc::clone(&self);
        
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_millis(100)).await;
                if let Err(e) = consensus.handle_election_timeout().await {
                    eprintln!("Error in election timer: {}", e);
                }
            }
        });
        
        Ok(())
    }

    async fn run_heartbeat_timer(self: Arc<Self>) -> RaftResult<()> {
        let consensus = Arc::clone(&self);
        
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_millis(50)).await;
                
                let should_send = {
                    let state = consensus.state.lock().await;
                    state.should_send_heartbeat()
                };
                
                if should_send {
                    if let Err(e) = consensus.replicate_logs().await {
                        eprintln!("Error in log replication: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }

    pub async fn replicate_logs(&self) -> RaftResult<()> {
        let (term, node_id) = {
            let state = self.state.lock().await;
            if state.role != NodeRole::Leader {
                return Ok(());
            }
            (state.current_term, state.node_id.clone())
        };

        for peer_id in self.cluster.keys() {
            let next_index = {
                let next_indices = self.next_index.lock().await;
                next_indices.get(peer_id).cloned().unwrap_or(1)
            };

            let prev_log_index = next_index - 1;
            let prev_log_term = if prev_log_index == 0 {
                0
            } else {
                match self.log_store.lock().await.get(prev_log_index)? {
                    Some(entry) => entry.term,
                    None => {
                        eprintln!("Previous log entry not found at index {}", prev_log_index);
                        continue;
                    }
                }
            };

            let last_log_index = self.log_store.lock().await.last_index()?;
            let entries = if next_index <= last_log_index {
                self.log_store.lock().await.get_range(next_index, last_log_index + 1)?
            } else {
                Vec::new()
            };

            let leader_commit = self.log_store.lock().await.committed_index()?;

            let request = RaftMessage::AppendEntries {
                term,
                leader_id: node_id.clone(),
                prev_log_index,
                prev_log_term,
                entries: entries.clone(),
                leader_commit,
            };

            let transport = Arc::clone(&self.transport);
            let peer_id = peer_id.clone();
            let next_index_ref = Arc::clone(&self.next_index);
            let match_index_ref = Arc::clone(&self.match_index);
            let consensus = Arc::new(RaftConsensus {
                state: Arc::clone(&self.state),
                transport: Arc::clone(&self.transport),
                log_store: Arc::clone(&self.log_store),
                cluster: Arc::clone(&self.cluster),
                next_index: Arc::clone(&self.next_index),
                match_index: Arc::clone(&self.match_index),
            });
            let entries_len = entries.len();

            tokio::spawn(async move {
                match transport.send(&peer_id, request).await {
                    Ok(_) => {
                        // Wait for response after successful sending...
                        // In actual implementation, the response should be received through some mechanism
                        // Here, the process is simplified and it is assumed that it is always successful
                        if entries_len > 0 {
                            let mut next_indices = next_index_ref.lock().await;
                            let mut match_indices = match_index_ref.lock().await;
                            
                            let new_next_index = next_index + entries_len as u64;
                            next_indices.insert(peer_id.clone(), new_next_index);
                            match_indices.insert(peer_id.clone(), new_next_index - 1);

                            consensus.update_commit_index().await.unwrap_or_else(|e| {
                                eprintln!("Failed to update commit index: {}", e);
                            });
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to send AppendEntries to {}: {}", peer_id, e);
                        let mut next_indices = next_index_ref.lock().await;
                        if let Some(index) = next_indices.get_mut(&peer_id) {
                            *index = (*index).saturating_sub(1);
                        }
                    }
                }
            });
        }

        Ok(())
    }

    pub async fn handle_append_entries(
        &self,
        term: u64,
        leader_id: String,
        prev_log_index: u64,
        prev_log_term: u64,
        entries: Vec<LogEntry>,
        leader_commit: u64
    ) -> RaftResult<()> {
        let mut success = false;
        let current_term;

        {
            let mut state = self.state.lock().await;
            
            if term < state.current_term {
                current_term = state.current_term;
            } else {
                state.update_term(term)?;
                state.role = NodeRole::Follower;
                current_term = term;

                let log_ok = if prev_log_index == 0 {
                    true
                } else {
                    match self.log_store.lock().await.get(prev_log_index)? {
                        Some(entry) => entry.term == prev_log_term,
                        None => false
                    }
                };

                if log_ok {
                    if prev_log_index + 1 <= self.log_store.lock().await.last_index()? {
                        self.log_store.lock().await.delete_from(prev_log_index + 1)?;
                    }

                    if !entries.is_empty() {
                        self.log_store.lock().await.append(entries.clone())?;
                    }

                    let current_commit = self.log_store.lock().await.committed_index()?;
                    if leader_commit > current_commit {
                        let last_new_index = self.log_store.lock().await.last_index()?;
                        let commit_index = std::cmp::min(leader_commit, last_new_index);
                        self.log_store.lock().await.commit(commit_index)?;
                    }

                    success = true;
                }
            }
        }

        let response = RaftMessage::AppendEntriesResponse {
            term: current_term,
            success,
            match_index: if success {
                prev_log_index + entries.len() as u64
            } else {
                0
            },
        };

        self.transport.send(&leader_id, response).await?;

        Ok(())
    }

    pub async fn handle_append_entries_response(
        &self,
        follower_id: String,
        term: u64,
        success: bool,
        match_index: u64
    ) -> RaftResult<()> {
        let mut state = self.state.lock().await;

        if term > state.current_term {
            state.update_term(term)?;
            return Ok(());
        }

        if state.role != NodeRole::Leader || term != state.current_term {
            return Ok(());
        }

        if success {
            {
                let mut next_indices = self.next_index.lock().await;
                let mut match_indices = self.match_index.lock().await;
                
                next_indices.insert(follower_id.clone(), match_index + 1);
                match_indices.insert(follower_id.clone(), match_index);
            }

            drop(state);
            self.update_commit_index().await?;
        } else {
            let mut next_indices = self.next_index.lock().await;
            if let Some(index) = next_indices.get_mut(&follower_id) {
                *index = (*index).saturating_sub(1);
            }
        }

        Ok(())
    }

    async fn update_commit_index(&self) -> RaftResult<()> {
        let (current_term, is_leader) = {
            let state = self.state.lock().await;
            (state.current_term, state.role == NodeRole::Leader)
        };

        if !is_leader {
            return Ok(());
        }

        let last_log_index = self.log_store.lock().await.last_index()?;
        let match_indices = self.match_index.lock().await;
        
        for index in (self.log_store.lock().await.committed_index()?..=last_log_index).rev() {
            let mut count = 1;
            
            let log_term = match self.log_store.lock().await.get(index)? {
                Some(entry) => entry.term,
                None => continue,
            };
            
            if log_term != current_term {
                continue;
            }

            for &match_idx in match_indices.values() {
                if match_idx >= index {
                    count += 1;
                }
            }

            if count > (self.cluster.len() + 1) / 2 {
                self.log_store.lock().await.commit(index)?;
                break;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use crate::cluster::{log_store::MockLogStore, message::{LogEntry, RaftMessage}, transport::MockTransport};
    //use async_trait::async_trait;

    async fn setup_consensus() -> (Arc<RaftConsensus<MockTransport, MockLogStore>>, Arc<MockTransport>) {
        let transport = Arc::new(MockTransport::new("node1".to_string()));
        let log_store = Arc::new(Mutex::new(MockLogStore::new()));
        
        let mut cluster = HashMap::new();
        cluster.insert("node2".to_string(), "addr2".to_string());
        cluster.insert("node3".to_string(), "addr3".to_string());

        let consensus = RaftConsensus::new(
            "node1".to_string(),
            Arc::clone(&transport),
            log_store,
            cluster
        );

        (consensus, transport)
    }

    #[tokio::test]
    async fn test_log_store_operations() {
        let mut store = MockLogStore::new();
        
        let entries = vec![
            LogEntry::new(1, 1, b"test1".to_vec()),
            LogEntry::new(1, 2, b"test2".to_vec()),
        ];
        let last_index = store.append(entries.clone()).unwrap();
        assert_eq!(last_index, 2);

        let entry = store.get(1).unwrap().unwrap();
        assert_eq!(entry.data, b"test1".to_vec());

        let range = store.get_range(1, 3).unwrap();
        assert_eq!(range.len(), 2);

        store.commit(2).unwrap();
        assert_eq!(store.committed_index().unwrap(), 2);

        store.delete_from(2).unwrap();
        assert_eq!(store.last_index().unwrap(), 1);

        store.snapshot().unwrap();
        assert_eq!(store.snapshots.len(), 1);
    }

    #[tokio::test]
    async fn test_transport_operations() {
        let mut transport = MockTransport::new("node1".to_string());
        
        transport.add_node("node2".to_string(), "addr2".to_string()).await.unwrap();
        
        let msg = RaftMessage::Heartbeat {
            term: 1,
            leader_id: "node1".to_string(),
        };
        transport.send("node2", msg.clone()).await.unwrap();

        let messages = transport.get_messages().await;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].0, "node2");

        transport.remove_node("node2").await.unwrap();
        let connections = transport.connections.lock().await;
        assert!(connections.is_empty());
    }
}
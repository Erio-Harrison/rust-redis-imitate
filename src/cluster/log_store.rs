// src/cluster/log_store.rs
//use std::sync::{Arc, RwLock};
use super::message::LogEntry;
use super::error::{RaftError, RaftResult};
use std::fs;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    pub last_index: u64,
    pub last_term: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub metadata: SnapshotMetadata,
    pub data: Vec<u8>,
}

pub struct MemLogStore {
    logs: Vec<LogEntry>,
    committed_index: u64,
    snapshot_dir: PathBuf,
    current_snapshot: Option<Snapshot>,
}

pub trait LogStore: Send + Sync {
    fn append(&mut self, entries: Vec<LogEntry>) -> RaftResult<u64>;
    fn get(&self, index: u64) -> RaftResult<Option<LogEntry>>;
    fn get_range(&self, start: u64, end: u64) -> RaftResult<Vec<LogEntry>>;
    fn delete_from(&mut self, index: u64) -> RaftResult<()>;
    fn last_index(&self) -> RaftResult<u64>;
    fn last_term(&self) -> RaftResult<u64>;
    
    fn commit(&mut self, index: u64) -> RaftResult<()>;
    fn committed_index(&self) -> RaftResult<u64>;
    
    fn snapshot(&mut self) -> RaftResult<()>;
    fn restore_snapshot(&mut self, data: Vec<u8>) -> RaftResult<()>;
}

impl MemLogStore {
    pub fn new(snapshot_dir: PathBuf) -> RaftResult<Self> {
        fs::create_dir_all(&snapshot_dir)?;
        
        Ok(MemLogStore {
            logs: Vec::new(),
            committed_index: 0,
            snapshot_dir,
            current_snapshot: None,
        })
    }

    fn snapshot_path(&self) -> PathBuf {
        self.snapshot_dir.join("snapshot.dat")
    }
}

impl LogStore for MemLogStore {
    fn append(&mut self, entries: Vec<LogEntry>) -> RaftResult<u64> {
        if entries.is_empty() {
            return Ok(self.last_index()?);
        }

        // Verify log continuity
        let first_index = entries[0].index;
        if first_index <= self.last_index()? {
            // Delete old conflicting logs
            self.delete_from(first_index)?;
        } else if first_index > self.last_index()? + 1 {
            // There is a gap in the log, and an error is returned
            return Err(RaftError::LogNotFound(self.last_index()? + 1));
        }

        // Append new log
        self.logs.extend(entries);
        Ok(self.last_index()?)
    }

    fn get(&self, index: u64) -> RaftResult<Option<LogEntry>> {
        if index == 0 || index > self.last_index()? {
            return Ok(None);
        }
        
        // Since the log index starts at 1, the array index should be reduced by 1.
        Ok(self.logs.get(index as usize - 1).cloned())
    }

    fn get_range(&self, start: u64, end: u64) -> RaftResult<Vec<LogEntry>> {
        if start == 0 || start > end || start > self.last_index()? {
            return Ok(Vec::new());
        }

        let end_idx = std::cmp::min(end, self.last_index()?);
        Ok(self.logs[(start - 1) as usize..end_idx as usize].to_vec())
    }

    fn delete_from(&mut self, index: u64) -> RaftResult<()> {
        if index == 0 || index > self.last_index()? {
            return Ok(());
        }

        // Truncate all logs starting from index
        self.logs.truncate((index - 1) as usize);
        
        // If the committed log is deleted, committed_index needs to be updated
        if self.committed_index > self.last_index()? {
            self.committed_index = self.last_index()?;
        }
        
        Ok(())
    }

    fn last_index(&self) -> RaftResult<u64> {
        Ok(self.logs.len() as u64)
    }

    fn last_term(&self) -> RaftResult<u64> {
        Ok(self.logs.last().map_or(0, |entry| entry.term))
    }

    fn commit(&mut self, index: u64) -> RaftResult<()> {
        if index == 0 {
            return Ok(());
        }

        if index > self.last_index()? {
            return Err(RaftError::LogNotFound(index));
        }

        // Cannot rollback committed_index
        if index < self.committed_index {
            return Err(RaftError::State(
                format!("Cannot commit index {} < committed_index {}", 
                    index, self.committed_index)
            ));
        }

        self.committed_index = index;
        Ok(())
    }

    fn committed_index(&self) -> RaftResult<u64> {
        Ok(self.committed_index)
    }

    fn snapshot(&mut self) -> RaftResult<()> {
        let snapshot_index = self.committed_index;
        if snapshot_index == 0 {
            return Ok(());
        }

        // Get the term of the last commit log
        let last_term = self.get(snapshot_index)?
            .ok_or_else(|| RaftError::LogNotFound(snapshot_index))?.term;

        // Create snapshot metadata
        let metadata = SnapshotMetadata {
            last_index: snapshot_index,
            last_term,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        // Serialize the committed log
        let snapshot_data = self.logs[..snapshot_index as usize].to_vec();
        let data = bincode::serialize(&snapshot_data)?;

        // Creating a snapshot
        let snapshot = Snapshot {
            metadata,
            data,
        };

        // Persistent snapshots
        let snapshot_bytes = bincode::serialize(&snapshot)?;
        let temp_path = self.snapshot_path().with_extension("tmp");
        fs::write(&temp_path, snapshot_bytes)?;
        fs::rename(temp_path, self.snapshot_path())?;

        // Update memory status
        self.current_snapshot = Some(snapshot);

        // Compress logs
        let remaining_logs = self.logs[snapshot_index as usize..].to_vec();
        self.logs = remaining_logs;

        Ok(())
    }

    fn restore_snapshot(&mut self, data: Vec<u8>) -> RaftResult<()> {
        // Parsing snapshots
        let snapshot: Snapshot = bincode::deserialize(&data)
            .map_err(|e| RaftError::State(format!("Failed to deserialize snapshot: {}", e)))?;

        // Verify the snapshot
        if let Some(current) = &self.current_snapshot {
            if snapshot.metadata.last_index < current.metadata.last_index {
                return Err(RaftError::State("Cannot restore older snapshot".into()));
            }
        }

        // Parsing log data
        let snapshot_logs: Vec<LogEntry> = bincode::deserialize(&snapshot.data)
            .map_err(|e| RaftError::State(format!("Failed to deserialize logs: {}", e)))?;

        // Update Status
        self.logs = snapshot_logs;
        self.committed_index = snapshot.metadata.last_index;
        self.current_snapshot = Some(snapshot);

        // Persistent snapshots
        let snapshot_bytes = bincode::serialize(&self.current_snapshot)?;
        fs::write(self.snapshot_path(), snapshot_bytes)?;

        Ok(())
    }
}


pub struct MockLogStore {
    pub logs: Vec<LogEntry>,
    pub committed_index: u64,
    pub snapshots: Vec<Vec<u8>>,
}

impl MockLogStore {
    pub fn new() -> Self {
        MockLogStore {
            logs: Vec::new(),
            committed_index: 0,
            snapshots: Vec::new(),
        }
    }
}

impl LogStore for MockLogStore {
    fn append(&mut self, entries: Vec<LogEntry>) -> RaftResult<u64> {
        self.logs.extend(entries);
        Ok(self.logs.len() as u64)
    }

    fn get(&self, index: u64) -> RaftResult<Option<LogEntry>> {
        if index == 0 || index > self.logs.len() as u64 {
            return Ok(None);
        }
        Ok(self.logs.get(index as usize - 1).cloned())
    }

    fn get_range(&self, start: u64, end: u64) -> RaftResult<Vec<LogEntry>> {
        if start == 0 || start > end || start > self.logs.len() as u64 {
            return Ok(Vec::new());
        }

        let end_idx = std::cmp::min(end as usize, self.logs.len());
        Ok(self.logs[(start - 1) as usize..end_idx].to_vec())
    }

    fn delete_from(&mut self, index: u64) -> RaftResult<()> {
        if index <= self.logs.len() as u64 {
            self.logs.truncate((index - 1) as usize);
            if self.committed_index > self.logs.len() as u64 {
                self.committed_index = self.logs.len() as u64;
            }
        }
        Ok(())
    }

    fn last_index(&self) -> RaftResult<u64> {
        Ok(self.logs.len() as u64)
    }

    fn last_term(&self) -> RaftResult<u64> {
        Ok(self.logs.last().map_or(0, |e| e.term))
    }

    fn commit(&mut self, index: u64) -> RaftResult<()> {
        if index > self.logs.len() as u64 {
            return Err(RaftError::LogNotFound(index));
        }
        self.committed_index = index;
        Ok(())
    }

    fn committed_index(&self) -> RaftResult<u64> {
        Ok(self.committed_index)
    }

    fn snapshot(&mut self) -> RaftResult<()> {
        let snapshot_data = bincode::serialize(&self.logs)?;
        self.snapshots.push(snapshot_data);
        Ok(())
    }

    fn restore_snapshot(&mut self, data: Vec<u8>) -> RaftResult<()> {
        self.logs = bincode::deserialize(&data)?;
        self.committed_index = self.logs.len() as u64;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    // Helper function to create a LogEntry with all required fields
    fn create_test_log_entry(index: u64, term: u64, data: &[u8], timestamp: u64) -> LogEntry {
        LogEntry {
            index,
            term,
            data: data.to_vec(),
            timestamp,
        }
    }

    // Set up a MemLogStore for testing
    fn setup_test_log_store() -> MemLogStore {
        let snapshot_dir = PathBuf::from("test_snapshots");
        if snapshot_dir.exists() {
            fs::remove_dir_all(&snapshot_dir).unwrap();
        }
        MemLogStore::new(snapshot_dir).unwrap()
    }

    #[test]
    fn test_append_and_get_logs() {
        let mut store = setup_test_log_store();

        let entries = vec![
            create_test_log_entry(1, 1, b"log1", 100),
            create_test_log_entry(2, 1, b"log2", 200),
        ];
        store.append(entries).unwrap();

        assert_eq!(store.last_index().unwrap(), 2);

        let entry1 = store.get(1).unwrap();
        assert!(entry1.is_some());
        assert_eq!(
            entry1.unwrap(),
            create_test_log_entry(1, 1, b"log1", 100)
        );

        let entry2 = store.get(2).unwrap();
        assert!(entry2.is_some());
        assert_eq!(
            entry2.unwrap(),
            create_test_log_entry(2, 1, b"log2", 200)
        );
    }

    #[test]
    fn test_get_range_of_logs() {
        let mut store = setup_test_log_store();

        let entries = vec![
            create_test_log_entry(1, 1, b"log1", 100),
            create_test_log_entry(2, 1, b"log2", 200),
            create_test_log_entry(3, 1, b"log3", 300),
        ];
        store.append(entries).unwrap();

        let range = store.get_range(2, 3).unwrap();
        assert_eq!(range.len(), 2);
        assert_eq!(range[0], create_test_log_entry(2, 1, b"log2", 200));
        assert_eq!(range[1], create_test_log_entry(3, 1, b"log3", 300));
    }
}

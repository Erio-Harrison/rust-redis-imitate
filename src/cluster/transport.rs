use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::Duration;
use super::message::RaftMessage;
use super::error::{RaftError, RaftResult};

pub trait Transport: Send + Sync {
    /// Send message to specified node
    fn send(&self, to: &str, msg: RaftMessage) -> impl std::future::Future<Output = RaftResult<()>> + Send;
    /// Start transport service to listen for messages
    fn start(&self) -> impl std::future::Future<Output = RaftResult<()>> + Send;
    /// Add a new node to the cluster
    fn add_node(&self, node_id: String, addr: String) -> impl std::future::Future<Output = RaftResult<()>> + Send;
    /// Remove a node from the cluster
    fn remove_node(&self, node_id: &str) -> impl std::future::Future<Output = RaftResult<()>> + Send;
}

pub struct RaftTransport {
    /// Current node ID
    node_id: String,
    /// Node connection pool
    connections: Arc<RwLock<HashMap<String, Arc<Mutex<TcpStream>>>>>,
    /// Message broadcast channel
    broadcast_tx: mpsc::Sender<(String, RaftMessage)>,
    /// Message receive callback
    msg_callback: Arc<dyn Fn(RaftMessage) -> RaftResult<()> + Send + Sync>,
}

impl RaftTransport {
    pub fn new(
        node_id: String, 
        msg_callback: Arc<dyn Fn(RaftMessage) -> RaftResult<()> + Send + Sync>
    ) -> Self {
        let (tx, mut rx) = mpsc::channel(1000);
        
        let transport = RaftTransport {
            node_id,
            connections: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx: tx,
            msg_callback,
        };

        let connections = Arc::clone(&transport.connections);
        tokio::spawn(async move {
            while let Some((to, msg)) = rx.recv().await {
                let stream_clone = {
                    connections.read()
                        .get(&to)
                        .map(|s| Arc::clone(s)) // Clone the Arc
                };

                if let Some(stream) = stream_clone {
                    match bincode::serialize(&msg) {
                        Ok(msg_data) => {
                            let mut stream = stream.lock().await; // Lock the Mutex
                            if let Err(e) = tokio::time::timeout(
                                Duration::from_secs(5),
                                stream.write_all(&msg_data)
                            ).await {
                                eprintln!("Failed to send message: {}", e);
                            }
                        }
                        Err(e) => eprintln!("Failed to serialize message: {}", e),
                    }
                }
            }
        });

        transport
    }
}

impl Transport for RaftTransport {
    async fn send(&self, to: &str, msg: RaftMessage) -> RaftResult<()> {
        self.broadcast_tx.send((to.to_string(), msg))
            .await
            .map_err(|e| RaftError::Transport(format!("Send failed: {}", e)))?;
        Ok(())
    }

    async fn add_node(&self, node_id: String, addr: String) -> RaftResult<()> {
        let stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| RaftError::Transport(format!("Connect failed: {}", e)))?;
        
        self.connections.write().insert(node_id, Arc::new(Mutex::new(stream)));
        Ok(())
    }

    async fn remove_node(&self, node_id: &str) -> RaftResult<()> {
        self.connections.write().remove(node_id);
        Ok(())
    }

    async fn start(&self) -> RaftResult<()> {
        let addr = "0.0.0.0:5000";
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| RaftError::Transport(format!("Bind failed: {}", e)))?;

        println!("Transport listening on {}", addr);

        let msg_callback = Arc::clone(&self.msg_callback);

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((mut stream, addr)) => {
                        println!("New connection from: {}", addr);
                        
                        let msg_callback = Arc::clone(&msg_callback);
                        
                        tokio::spawn(async move {
                            let mut buffer = Vec::new();
                            
                            loop {
                                let mut len_bytes = [0u8; 4];
                                match stream.read_exact(&mut len_bytes).await {
                                    Ok(_) => {
                                        let len = u32::from_be_bytes(len_bytes) as usize;
                                        buffer.resize(len, 0);
                                        match stream.read_exact(&mut buffer).await {
                                            Ok(_) => {
                                                match bincode::deserialize::<RaftMessage>(&buffer) {
                                                    Ok(msg) => {
                                                        if let Err(e) = (msg_callback)(msg) {
                                                            eprintln!("Failed to process message: {}", e);
                                                        }
                                                    }
                                                    Err(e) => eprintln!("Failed to deserialize message: {}", e),
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("Failed to read message: {}", e);
                                                break;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to read message length: {}", e);
                                        break;
                                    }
                                }
                            }
                        });
                    }
                    Err(e) => eprintln!("Failed to accept connection: {}", e),
                }
            }
        });

        Ok(())
    }
}

pub struct MockTransport {
    pub node_id: String,
    pub messages: Arc<Mutex<Vec<(String, RaftMessage)>>>,
    pub msg_callback: Option<Arc<dyn Fn(RaftMessage) -> RaftResult<()> + Send + Sync>>,
    pub connections: Arc<Mutex<HashMap<String, String>>>, // node_id -> addr
}

impl MockTransport {
    pub fn new(node_id: String) -> Self {
        MockTransport {
            node_id,
            messages: Arc::new(Mutex::new(Vec::new())),
            msg_callback: None,
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn set_callback(&mut self, callback: Arc<dyn Fn(RaftMessage) -> RaftResult<()> + Send + Sync>) {
        self.msg_callback = Some(callback);
    }

    pub async fn get_messages(&self) -> Vec<(String, RaftMessage)> {
        self.messages.lock().await.clone()
    }
}

//#[async_trait]
impl Transport for MockTransport {
    async fn send(&self, to: &str, msg: RaftMessage) -> RaftResult<()> {
        self.messages.lock().await.push((to.to_string(), msg.clone()));
    
        if let Some(callback) = &self.msg_callback {
            callback(msg)?;
        }
        
        Ok(())
    }

    async fn start(&self) -> RaftResult<()> {
        Ok(())
    }

    async fn add_node(&self, node_id: String, addr: String) -> RaftResult<()> {
        self.connections.lock().await.insert(node_id, addr);
        Ok(())
    }

    async fn remove_node(&self, node_id: &str) -> RaftResult<()> {
        self.connections.lock().await.remove(node_id);
        Ok(())
    }
}
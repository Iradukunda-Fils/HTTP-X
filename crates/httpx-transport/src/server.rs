use crate::dispatcher::CoreDispatcher;
use httpx_core::ControlSignal;
use std::net::SocketAddr;
use httpx_core::ServerConfig;
use socket2::{Socket, Domain, Type, Protocol};
use io_uring::IoUring;
use std::os::unix::io::AsRawFd;

pub struct HttpxServer {
    addr: SocketAddr,
    config: ServerConfig,
    predictive_mode: bool,
    trie: Option<httpx_dsa::LinearIntentTrie>,
    slab: Option<std::sync::Arc<httpx_dsa::SecureSlab>>,
}

impl HttpxServer {
    pub fn listen(addr: &str) -> Self {
        Self {
            addr: addr.parse().expect("Invalid address"),
            config: ServerConfig::default(),
            predictive_mode: false,
            trie: None,
            slab: None,
        }
    }

    pub fn from_builder(builder: httpx_core::ServerBuilder, addr: &str) -> Self {
        Self::listen(addr)
            .with_config(builder.config)
            .with_trie(builder.registry.take_trie())
    }

    pub fn with_trie(mut self, trie: httpx_dsa::LinearIntentTrie) -> Self {
        self.trie = Some(trie);
        self
    }

    pub fn with_config(mut self, config: ServerConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_intent_predicting(mut self) -> Self {
        self.predictive_mode = true;
        self
    }

    pub fn with_slab(mut self, slab: std::sync::Arc<httpx_dsa::SecureSlab>) -> Self {
        self.slab = Some(slab);
        self
    }

    /// Starts the HTTP-X Server Swarm with Mechanical Sympathy.
    pub async fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Initializing HTTP-X Sovereign Swarm on {}", self.addr);
        
        let (_global_tx, mut _global_rx) = tokio::sync::mpsc::channel::<ControlSignal>(1024);
        let mut primary_fd: Option<std::os::unix::io::RawFd> = None;

        // Initialize Learning Channel (Swarm -> Orchestrator)
        let (learn_tx, learn_rx) = tokio::sync::mpsc::unbounded_channel::<(Vec<u8>, bool)>();
        let mut worker_txs = Vec::new();

        let slab = self.slab.clone().unwrap_or_else(|| {
            std::sync::Arc::new(httpx_dsa::SecureSlab::new(self.config.slab_capacity))
        });

        let trie = self.trie.clone().unwrap_or_else(|| httpx_dsa::LinearIntentTrie::new(1024));

        for core_id in 0..self.config.threads {
            let addr = self.addr;
            let config = self.config.clone();
            let slab = slab.clone();
            let trie = trie.clone();
            let (control_tx, control_rx) = tokio::sync::mpsc::channel::<ControlSignal>(100);
            worker_txs.push(control_tx);
            
            let learn_tx = learn_tx.clone();

            // # Mechanical Sympathy: Shared SQPOLL
            // In Production Mode, create the ring here and pass it down.
            // Core 0 creates the WQ, others attach to it.
            let ring = if self.config.production_mode {
                let mut builder = IoUring::builder();
                builder.setup_sqpoll(2000);
                if let Some(fd) = primary_fd {
                    builder.setup_attach_wq(fd);
                }
                let ring = builder.build(2048).expect("Failed to create Production Ring");
                
                if primary_fd.is_none() {
                    primary_fd = Some(ring.as_raw_fd());
                }
                ring
            } else {
                IoUring::builder().build(128).expect("Failed to create Dev Ring")
            };
            
            std::thread::Builder::new()
                .name(format!("httpx-worker-{}", core_id))
                .spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();
                        
                    rt.block_on(async move {
                        // 1. Create a raw socket with SO_REUSEPORT
                        let socket = Socket::new(Domain::for_address(addr), Type::DGRAM, Some(Protocol::UDP)).unwrap();
                        socket.set_reuse_port(true).unwrap();
                        socket.set_nonblocking(true).unwrap();
                        socket.bind(&addr.into()).unwrap();
                        
                        let tokio_socket = tokio::net::UdpSocket::from_std(std::net::UdpSocket::from(socket)).unwrap();
                        let trie = trie.clone();
                        
                        let mut dispatcher = CoreDispatcher::new_from_ring(
                            core_id, 
                            tokio_socket, 
                            control_rx,
                            config,
                            trie,
                            ring,
                            learn_tx,
                        ).await.unwrap();

                        dispatcher.register_slab(&slab).unwrap();
                        
                        dispatcher.run_loop(&slab).await;
                    });
                })?;
        }

        // Start the ClusterOrchestrator on the next available core
        let orchestrator_core = self.config.threads; 
        let orchestrator = httpx_cluster::orchestrator::ClusterOrchestrator::new(
            orchestrator_core,
            learn_rx,
            worker_txs,
        );
        
        tokio::spawn(async move {
            orchestrator.run().await;
        });

        // Keep the swarm alive
        std::future::pending::<()>().await;
        Ok(())
    }
}

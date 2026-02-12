use std::net::SocketAddr;
use std::sync::Arc;
use httpx_core::ControlSignal;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use httpx_core::{ServerConfig, PredictiveEngine};
use crate::stream::GsoPacketizer;
use io_uring::{opcode, types, IoUring};
use std::os::unix::io::AsRawFd;

/// A NUMA-aware packet dispatcher bound to a specific CPU core.
pub struct CoreDispatcher {
    _core_id: usize,
    socket: Arc<UdpSocket>,
    engine: Arc<PredictiveEngine>,
    control_rx: mpsc::Receiver<ControlSignal>,
    ring: IoUring,
    #[allow(dead_code)]
    config: ServerConfig,
    packetizer: GsoPacketizer,
    learn_tx: mpsc::UnboundedSender<(Vec<u8>, bool)>,
}

impl CoreDispatcher {
    /// Initializes a dispatcher with an existing socket and control channel.
    // Legacy constructor wrapper for tests and simple usage.
    pub async fn new_with_socket(
        core_id: usize, 
        socket: UdpSocket, 
        control_rx: mpsc::Receiver<ControlSignal>,
        config: ServerConfig,
        trie: httpx_dsa::LinearIntentTrie,
        learn_tx: mpsc::UnboundedSender<(Vec<u8>, bool)>,
    ) -> Result<Self, std::io::Error> {
        // Default minimal (dev) configuration.
        let ring = IoUring::builder().build(128)?;
        Self::new_from_ring(core_id, socket, control_rx, config, trie, ring, learn_tx).await
    }

    /// Initializes a dispatcher with an existing ring (allows for shared WQ / SQPOLL).
    pub async fn new_from_ring(
        core_id: usize, 
        socket: UdpSocket, 
        control_rx: mpsc::Receiver<ControlSignal>,
        config: ServerConfig,
        trie: httpx_dsa::LinearIntentTrie,
        ring: IoUring,
        learn_tx: mpsc::UnboundedSender<(Vec<u8>, bool)>,
    ) -> Result<Self, std::io::Error> {
        let engine = Arc::new(PredictiveEngine::new(true));
        engine.swap_weights(trie);

        let packetizer = GsoPacketizer::new(config.slab_capacity);
        
        Ok(Self {
            _core_id: core_id,
            socket: Arc::new(socket),
            engine,
            control_rx,
            ring,
            config,
            packetizer,
            learn_tx,
        })
    }

    /// Registers the SecureSlab memory with io_uring for zero-copy Fixed I/O.
    pub fn register_slab(&self, slab: &httpx_dsa::SecureSlab) -> std::io::Result<()> {
        let mut iovecs = Vec::with_capacity(slab.slots());
        for i in 0..slab.slots() {
            iovecs.push(libc::iovec {
                iov_base: slab.get_slot(i) as *mut libc::c_void,
                iov_len: 4096, // Fixed page size
            });
        }
        
        unsafe {
            self.ring.submitter().register_buffers(&iovecs)
        }
    }

    /// The High-Performance Hot-Path.
    pub async fn run_loop(&mut self, slab: &httpx_dsa::SecureSlab) {
        let mut buf = [0u8; 4096]; 

        loop {
            // # Mechanical Sympathy: Reaping completions reduces memory pressure.
            self.reap_completions(slab);

            tokio::select! {
                Some(signal) = self.control_rx.recv() => {
                    self.handle_control(signal).await;
                }
                Ok((len, src)) = self.socket.recv_from(&mut buf) => {
                    self.on_packet(&buf[..len], src, slab).await;
                }
            }
        }
    }

    async fn handle_control(&self, signal: ControlSignal) {
        match signal {
            ControlSignal::Pivot(addr) => {
                tracing::warn!("Priority-Zero: Pivot detected for {}. Killing stale pushes.", addr);
                self.engine.cancel_for(&addr);
            }
            ControlSignal::KillAll => {
                tracing::error!("Priority-Zero: Global termination.");
            }
            ControlSignal::SwapTrie(new_trie) => {
                // Task 2: Shadow-Swap Handshake with RC Safety.
                self.engine.swap_weights((*new_trie).clone());
                tracing::info!("CoreDispatcher: Shadow-Swap Handshake Complete (Seq: {})", new_trie.sequence_number);
            }
        }
    }


    /// Reaps completions from the io_uring and recycles slab fragments.
    pub fn reap_completions(&mut self, slab: &httpx_dsa::SecureSlab) {
        let mut cq = self.ring.completion();
        while let Some(cqe) = cq.next() {
            let user_data = cqe.user_data();
            if user_data > 0 {
                // Decode combined handle: Payload (Low 32) | Template (High 32)
                let payload_handle = ((user_data & 0xFFFFFFFF) - 1) as usize;
                let template_data = (user_data >> 32) & 0xFFFFFFFF;
                
                slab.decrement_rc(payload_handle);
                
                if template_data > 0 {
                     let template_handle = (template_data - 1) as usize;
                     slab.decrement_rc(template_handle);
                }
            }
        }
    }

    /// Submits a GSO Super-Packet: Intent + Headers + Payload (Zero-Copy SendMsg).
    pub async fn submit_linked_burst(
        &mut self, 
        _target: SocketAddr, 
        payload_handle: u32, 
        template_handle: u32,
        expected_version: u32,
        slab: &httpx_dsa::SecureSlab
    ) -> std::io::Result<()> {
        let current_version = slab.get_version(payload_handle as usize);
        if current_version != expected_version {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Stale Payload"));
        }

        let fd = self.socket.as_raw_fd();
        
        // Prepare Vectored I/O (Intent, Header, Payload)
        // This eliminates the 3-SQE chain overhead.
        let msghdr_ptr = self.packetizer.prepare_burst(
            payload_handle as usize,
            b"INTENT_SYNC_FRAME".as_ptr(), b"INTENT_SYNC_FRAME".len(),
            slab.get_slot(template_handle as usize), 128,
            slab.get_slot(payload_handle as usize), 4096,
            0 // GSO segment size (future: config.mss)
        );

        // Encode Handles for RC Reaping
        let user_data = ((payload_handle as u64) + 1) | (((template_handle as u64) + 1) << 32);

        // SQE: SendMsg
        let op = opcode::SendMsg::new(
            types::Fd(fd),
            msghdr_ptr,
        ).build()
         .user_data(user_data);

        slab.increment_rc(payload_handle as usize);
        slab.increment_rc(template_handle as usize);

        unsafe {
            let mut sq = self.ring.submission();
            if sq.push(&op).is_err() {
                 // Backpressure: Return WouldBlock or drop
                 return Err(std::io::Error::new(std::io::ErrorKind::Other, "SQ Full"));
            }
        }

        let _ = self.ring.submit();
        Ok(())
    }

    /// Handles an incoming UDP packet and triggers a predictive push if a route matches.
    pub async fn on_packet(&mut self, data: &[u8], addr: SocketAddr, slab: &httpx_dsa::SecureSlab) {
        let session = httpx_core::session::Session::new(addr);
        
        // Task 2: Emit learning event before prediction
        let _ = self.learn_tx.send((data.to_vec(), true));

        if let Some((payload, version)) = self.engine.predict_for_path(&session, data) {
            let fd = self.socket.as_raw_fd();
            let sockaddr = socket2::SockAddr::from(addr);
            unsafe {
                let _ = libc::connect(fd, sockaddr.as_ptr(), sockaddr.len());
            }
            let _ = self.submit_linked_burst(addr, payload, 0, version, slab).await;
        }
    }
}

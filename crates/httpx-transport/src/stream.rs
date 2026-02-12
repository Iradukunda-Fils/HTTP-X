use std::io;
use std::os::unix::io::AsRawFd;
use tokio::net::UdpSocket;
use httpx_dsa::SecureSlab;

/// Handles zero-copy streaming of large payloads using GSO.
pub struct PayloadStreamer {
    socket: UdpSocket,
    _gso_size: u16,
}

impl PayloadStreamer {
    pub fn new(socket: UdpSocket, gso_size: u16) -> io::Result<Self> {
        let fd = socket.as_raw_fd();
        
        // # Mechanical Sympathy: UDP_SEGMENT (UDP GSO)
        // Enables batching multiple UDP segments into a single 64KB Super-Packet.
        // This reduces syscall frequency and PCIe bus transactions.
        unsafe {
            let val: libc::c_int = gso_size as libc::c_int;
            if libc::setsockopt(
                fd,
                libc::SOL_UDP,
                libc::UDP_SEGMENT,
                &val as *const libc::c_int as *const libc::c_void,
                std::mem::size_of::<libc::c_int>() as u32,
            ) != 0 {
                return Err(io::Error::last_os_error());
            }
        }

        Ok(Self {
            socket,
            _gso_size: gso_size,
        })
    }

    /// Stream a batch of fragments from the slab with a Freshness Guard.
    pub async fn stream_batch(
        &self, 
        slab: &SecureSlab, 
        handles: &[(u32, u32)], // (handle, expected_version)
        target: std::net::SocketAddr
    ) -> io::Result<usize> {
        let mut total = 0;
        let mut batch_buf = Vec::with_capacity(65535); // 64KB MAX GSO

        for &(handle, expected_version) in handles {
            // # Mechanical Sympathy Target: < 0.5ns check
            // Single CMP instruction to ensure semantic freshness.
            let physical_version = slab.get_version(handle as usize);
            if physical_version != expected_version {
                tracing::warn!("Freshness Violation: Stale push for handle {}. Expected {}, got {}.", handle, expected_version, physical_version);
                return Err(io::Error::new(io::ErrorKind::InvalidData, "Stale Payload"));
            }

            let buf = slab.get_slot(handle as usize);
            if batch_buf.len() + 4096 > 65535 {
                break;
            }
            
            unsafe {
                let slice = std::slice::from_raw_parts(buf, 4096);
                batch_buf.extend_from_slice(slice);
            }
            total += 1;
        }

        if total > 0 {
            self.socket.send_to(&batch_buf, target).await?;
        }

        Ok(total)
    }
}

/// Hardware-Offloaded Super-Packetizer for Zero-Copy io_uring Bursts.
pub struct GsoPacketizer {
    // Persistent iovec storage for in-flight operations.
    // Index by payload_handle.
    iovecs: Vec<[libc::iovec; 3]>,
    // Persistent CMSG storage (for UDP_SEGMENT).
    #[allow(dead_code)]
    cmsgs: Vec<[u8; 64]>,
    // Persistent msghdr storage (stable address for io_uring).
    msghdrs: Vec<libc::msghdr>,
    // Maximum slots supported by this packetizer
    #[allow(dead_code)]
    capacity: usize,
}

impl GsoPacketizer {
    pub fn new(capacity: usize) -> Self {
        // Initialize storage
        let mut iovecs = Vec::with_capacity(capacity);
        let mut cmsgs = Vec::with_capacity(capacity);
        let mut msghdrs = Vec::with_capacity(capacity);
        
        for _ in 0..capacity {
            // Default 3 iovecs per slot
            iovecs.push([
                libc::iovec { iov_base: std::ptr::null_mut(), iov_len: 0 },
                libc::iovec { iov_base: std::ptr::null_mut(), iov_len: 0 },
                libc::iovec { iov_base: std::ptr::null_mut(), iov_len: 0 },
            ]);
            cmsgs.push([0u8; 64]);
            msghdrs.push(unsafe { std::mem::zeroed() });
        }
        
        Self {
            iovecs,
            cmsgs,
            msghdrs,
            capacity,
        }
    }

    /// Prepares the iovecs and control messages for a GSO burst.
    /// Returns: (msghdr_ptr) for io_uring::SendMsg associated with the handle.
    pub fn prepare_burst(
        &mut self,
        handle: usize,
        intent_ptr: *const u8, intent_len: usize,
        header_ptr: *const u8, header_len: usize,
        payload_ptr: *const u8, payload_len: usize,
        _gso_size: u16, // Future: Use for UDP_SEGMENT
    ) -> *const libc::msghdr {
        let iovecs = &mut self.iovecs[handle];
        
        iovecs[0].iov_base = intent_ptr as *mut libc::c_void;
        iovecs[0].iov_len = intent_len;

        iovecs[1].iov_base = header_ptr as *mut libc::c_void;
        iovecs[1].iov_len = header_len;

        iovecs[2].iov_base = payload_ptr as *mut libc::c_void;
        iovecs[2].iov_len = payload_len;

        let msghdr = &mut self.msghdrs[handle];
        msghdr.msg_iov = iovecs.as_ptr() as *mut libc::iovec;
        msghdr.msg_iovlen = 3;
        
        // Todo: Implement CMSG construction for UDP_SEGMENT if kernel supports it via io_uring
        // Currently returning empty control buffer.
        msghdr.msg_control = std::ptr::null_mut();
        msghdr.msg_controllen = 0;
        msghdr.msg_name = std::ptr::null_mut();
        msghdr.msg_namelen = 0;

        msghdr as *const libc::msghdr
    }
}

use httpx_core::ServerBuilder;
use httpx_transport::HttpxServer;
use httpx_dsa::SecureSlab;
use httpx_codec::HeaderTemplate;
use std::sync::Arc;
use std::net::SocketAddr;
use std::time::Instant;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn rdtsc() -> u64 {
    unsafe { std::arch::x86_64::_rdtsc() }
}
#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn rdtsc() -> u64 { 0 }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // 1. Initialise Slab for Headers and Data
    // Requirement: Pre-allocated hardware-isolated slots
    let slab = Arc::new(SecureSlab::new(64));
    
    // 2. Prepare Header Template (Procrustean)
    // Constraint: Immutable block in SecureSlab
    let template_handle = 0;
    let base_headers = b"HTTP/1.1 200 OK\r\nDate: Wed, 21 Oct 2015 07:28:00 GMT\r\nContent-Length: 1024      \r\n\r\n";
    let _template = HeaderTemplate::new(&slab, template_handle, base_headers);
    
    // 3. Prepare "Hello World" Payload
    // Constraint: Statically resolved via u32 indices
    let payload_handle = 1;
    let payload = vec![0x41; 1024]; // 1KB of 'A'
    unsafe {
        std::ptr::copy_nonoverlapping(payload.as_ptr(), slab.get_slot(payload_handle as usize), 1024);
    }
    // High-frequency Freshness Chaos Certification (Phase 50/51)
    slab.set_version(payload_handle as usize, 100);

    // 4. Build Server via SAI (Sovereign Application Interface)
    let mut config = httpx_core::ServerConfig::default();
    config.threads = 1;
    config.slab_capacity = 64; 
    config.production_mode = true; // High-Performance Mode (HugeTLB + SQPOLL)

    let builder = ServerBuilder::new()
        .with_config(config)
        .route("/api/v1/hello", payload_handle, 100);
    
    let server = HttpxServer::from_builder(builder, "127.0.0.1:8081")
        .with_slab(slab.clone())
        .with_intent_predicting();

    println!("SAI Certified: Server shadowing on 127.0.0.1:8081");
    
    // 5. Spawn Server
    let _server_task = tokio::spawn(async move {
        println!("Server task starting...");
        let _ = server.start().await;
    });

    // 6. Run 15µs Challenge Benchmark
    println!("Client waiting for server to stand up...");
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    
    let client = std::net::UdpSocket::bind("127.0.0.1:0")?;
    client.set_nonblocking(true)?;
    let server_addr: SocketAddr = "127.0.0.1:8081".parse().unwrap();
    
    let request = b"/api/v1/hello";
    let iterations = 1000;
    let mut latencies_us = Vec::with_capacity(iterations);
    let mut latencies_cycles = Vec::with_capacity(iterations);

    println!("Starting Final 15µs Synchronous Challenge...");

    // # Mechanical Sympathy: Synchronous Loop for the benchmark
    for _ in 0..iterations {
        let start = Instant::now();
        
        client.send_to(request, server_addr)?;
        
        let mut bytes_recvd = 0;
        let mut buf = [0u8; 65535]; // Jumbo Frame Support
        
        let start_cycles = rdtsc();

        // Busy-poll for response (Super-Packet or Fragments)
        // Production Target: 1 Super-Packet containing Intent+Headers+Payload (~4.2KB)
        loop {
            match client.recv_from(&mut buf) {
                Ok((len, _)) => {
                    bytes_recvd += len;
                    if bytes_recvd >= 1024 {
                        break;
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Yield to prevent starvation on single-core setups
                    std::thread::yield_now();
                }
                Err(e) => return Err(e.into()),
            }
        }
        
        let end_cycles = rdtsc();
        let cycles = end_cycles - start_cycles;
        
        let duration = start.elapsed();
        latencies_us.push(duration.as_micros());
        latencies_cycles.push(cycles);
    }

    let avg_latency = if iterations > 0 { latencies_us.iter().sum::<u128>() / iterations as u128 } else { 0 };
    println!("Final SAI Benchmark Result (Sync): {}µs avg over {} iterations", avg_latency, iterations);
    println!("Cycles Per Packet (Last): {} cycles", latencies_cycles.last().unwrap_or(&0)); 
    
    if avg_latency < 15 && iterations > 0 {
        println!("SAI SUCCESS: End-to-End Latency < 15µs Target Achieved.");
    } else {
        println!("SAI VIOLATION: Latency {}µs exceeds 15µs target.", avg_latency);
    }

    Ok(())
}

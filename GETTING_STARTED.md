# GETTING_STARTED.md: The Developer Flight-Manual

Onboard the HTTP-X flight deck in under 5 minutes.

## 1. Physical Prerequisites

HTTP-X requires a modern Linux environment to utilize its mechanical sympathy features.

*   **Kernel**: 5.10+ (Core `io_uring` support).
*   **Memory**: Pre-allocate 2MB HugePages.
    ```bash
    echo 512 | sudo tee /proc/sys/vm/nr_hugepages
    ```
*   **Capabilities**: The binary requires `CAP_SYS_NICE` to spawn kernel-side `SQPOLL` threads.
    ```bash
    sudo setcap 'cap_sys_nice=eip' /path/to/target/release/examples/fast_api
    ```

## 2. The 3-Command Pulse

Validate your environment with the SAI benchmark suite:

```bash
# 1. Clone & Optimized Build
cargo build --release

# 2. Execute the SAI Performance Challenge
cargo run --release --example fast_api

# 3. Verify System Invariants
cargo test
```

## 3. Implementing your first Sovereign API

HTTP-X centers around the `ServerBuilder`. Routes are statically registered to pre-warmed memory handles.

```rust
use httpx_core::{ServerBuilder, ServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize with Production Hardening
    let config = ServerConfig {
        production_mode: true,
        threads: 4,
        ..Default::default()
    };

    let mut builder = ServerBuilder::new().with_config(config);

    // 2. Static Route Registration
    // Map /api/v1/hot to memory slot 0 
    builder.route("/api/v1/hot", 0, 1);

    // 3. Launch the Swarm
    let server = builder.build_server("0.0.0.0:8080");
    server.start().await?;

    Ok(())
}
```

## 4. Troubleshooting the Link-Chain

If the server fails to start, verify the following:
*   **Memlock Limits**: Ensure `ulimit -l` is set to "unlimited" or high enough for your `slab_capacity`.
*   **Port Binding**: HTTP-X uses `SO_REUSEPORT` for worker fanout; ensure no legacy UDP services are blocking the target port.

---

Next: [Dive into the Architecture](ARCHITECTURE.md)

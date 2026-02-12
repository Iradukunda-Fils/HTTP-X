# Developer Flight-Manual

This guide provides the necessary steps to deploy and extend the HTTP-X Sovereign Application Interface.

## 1. Environment Preparation

HTTP-X is designed for Linux systems with `io_uring` support (Kernel 5.10+).

### HugePage Allocation
Performance-critical memory requires pre-allocation of 2MB HugePages.
```bash
# Reserve 512 HugePages (1GB total)
echo 512 | sudo tee /proc/sys/vm/nr_hugepages
```
Verify allocation with `cat /proc/meminfo | grep Huge`.

### Binary Capabilities
Because `SQPOLL` utilizes high-priority kernel threads, the binary needs `CAP_SYS_NICE`.
```bash
sudo setcap 'cap_sys_nice=eip' ./target/release/examples/server_demo
```

## 2. The Build Lifecycle

HTTP-X uses a standard Cargo workflow but benefits significantly from Link-Time Optimization (LTO).

```bash
# Build for peak performance
cargo build --release

# Run internal functional check
cargo test

# Execute SAI Latency Benchmark
cargo run --release --example fast_api
```

## 3. The SAI Lifecycle: API Integration

### Step 1: Configuration
Initialize the `ServerConfig` for your hardware topology.
```rust
let config = ServerConfig {
    production_mode: true,
    threads: 8, // One per physical core
    slab_capacity: 4096,
    ..Default::default()
};
```

### Step 2: Route Burning
Register routes as "static intents." This "burns" them into the Trie pool, preventing runtime allocations.
```rust
let mut builder = ServerBuilder::new().with_config(config);
builder.route("/api/hft/v1", SLAB_INDEX, VERSION);
```

### Step 3: Launch
Start the server swarm. This initiates the `CoreDispatcher` on each thread and the `ClusterOrchestrator` on the control-plane core.
```rust
let server = builder.build_server("0.0.0.0:8080");
server.start().await?;
```

## 4. Troubleshooting

*   **Slab Failure**: If you see `mmap failed`, check if your HugePage allocation is sufficient or if `ulimit -l` (locked memory) is too low.
*   **Latency Spikes**: Check for CPU affinity conflicts. Ensure no other high-load processes are pinned to the worker cores.

---
*See [Deep-Dive Architecture](ARCHITECTURE.md) for technical internals.*

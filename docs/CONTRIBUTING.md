# The Sovereign Protocol

Contributing to HTTP-X requires a commitment to "Mechanical Sympathy." We prioritize hardware efficiency over developer convenience.

## Sovereign Standards

All Pull Requests are audited against these four non-negotiable standards:

### 1. Zero Hot-Path Allocations
No use of `Box`, `Vec`, `HashMap`, or other heap-allocated types within `CoreDispatcher::run_loop` or `PredictiveEngine::predict`. If you need dynamic memory, use a pre-allocated index in the `SecureSlab`.

### 2. Lock-Free Synchronization
Do not use `Mutex` or `RwLock` in the worker threads. Use atomic primitives with the correct memory ordering (`Acquire`/`Release`) or coordinate via the `ClusterOrchestrator` control plane.

### 3. Cache Line Isolation
Ensure all data structures accessed across thread boundaries are `#[repr(align(64))]`. Prevent false-sharing at all costs.

### 4. Zero-Copy Flow
If data is moved from the `SecureSlab` to the NIC, it must happen via a pointer/handle. `memcpy` is considered a performance failure.

## Performance-First Bug Reporting

We categorize issues by their impact on our core metrics:

*   **Latency Violation**: Any event that pushes First-Byte Latency above 10µs.
*   **Throughput Regression**: Any change that reduces the max ops/sec by more than 5%.
*   **Memory Violation**: Any leak or OOB access detected by the `leak_certification.rs` suite.

## The Technical Roadmap

Join the vanguard of systems engineering:
*   **eBPF Offload**: Moving the `GossipProtocol` discard logic into the kernel via eBPF.
*   **NUMA Locality Tuning**: Automatic slab placement on the core's local NUMA node.
*   **SIMD Trie Ops**: Utilizing AVX-512 or Neon for even faster path resolution.

## Where to Start?

New to the project? We recommend this onboarding path:

1.  **Understand the Slab**: Read `crates/httpx-dsa/src/slab.rs` to see how we bypass the standard allocator for hardware-aligned memory.
2.  **Trace a Packet**: Open `crates/httpx-transport/src/dispatcher.rs` and follow the `on_packet` flow from ring entry to predictive push.
3.  **Explore the Swarm**: See how weights converge across cores in `crates/httpx-cluster/src/orchestrator.rs`.
4.  **Run a Benchmark**: Execute `cargo run --release --example fast_api` and observe the latency logs in a production-mode environment.

---
*Building for the wire.*

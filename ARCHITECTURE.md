# ARCHITECTURE.md: Mechanical Sympathy Deep-Dive

This document provides a technical specification of the HTTP-X Sovereign Application Interface (SAI), designed to solve the "OS Bottleneck" for high-frequency applications.

## 1. The Memory Geometry (SecureSlab)

The `SecureSlab` is the foundation of zero-copy data transmission in HTTP-X.

### HugeTLB Contiguity
In **Production Mode**, the slab uses 2MB HugePages (`MAP_HUGETLB`). This reduces the Translation Lookaside Buffer (TLB) pressure, which is often a hidden bottleneck in high-throughput network applications.

### Hardware Guard Gates
Slots are separated by `PROT_NONE` guard pages. Any out-of-bounds (OOB) access during buffer preparation or kernel transmission triggers an immediate hardware-level `SIGSEGV`, preventing memory corruption before it reaches the wire.

## 2. The Transport Engine (io_uring)

HTTP-X bypasses the traditional socket API syscalls by using a dedicated `io_uring` plane.

### SQPOLL (Submission Polling)
When `production_mode` is active, worker threads share a kernel-side polling thread (`IORING_SETUP_SQPOLL`). This allows the `CoreDispatcher` to submit Super-Packets by simply writing to shared memory, with zero context switches into kernel-space.

### GSO Super-Packets
Instead of separate SQEs for Intent, Headers, and Payload, the `GsoPacketizer` assembles a single `sendmsg` SQE utilizing Generic Segmentation Offload (GSO). This reduces ring contention by 66%.

## 3. The Intelligence Layer (LinearIntentTrie)

Predictions are executed in $O(k)$ time, where $k$ is the path length, using a cache-line aligned Trie.

### Shadow-Swap & EBR
The `PredictiveEngine` uses **Epoch-Based Reclamation (EBR)** via `crossbeam-epoch`. This allows the master `ClusterOrchestrator` to swap the entire weight-set of the swarm atomically without locking the data-path workers. If the orchestrator discovers new traffic patterns, it merges them into a "Shadow Trie" and handshakes the update to the swarm.

## 4. The Reliability Plane

### Initial Intent Window (IIW)
Each session is granted "Credits" for speculative pushes. If the network experiences congestion (measured via RTT spikes), the `CongestionController` triggers an immediate backoff, revoking credits to prevent bufferbloat.

### Hysteresis Monitoring
The system monitors weight stability. If global convergence is erratic, HTTP-X enters "Sovereign Mode," where each core learns independently until the cluster stabilizes.

---

*For implementation details, see the inline documentation within `crates/httpx-transport`.*

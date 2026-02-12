# Mechanical Sympathy Deep-Dive

This document provides a technical specification of the HTTP-X Sovereign Application Interface (SAI), designed to solve the "OS Bottleneck" for high-frequency applications.

## 1. The Memory Geometry (SecureSlab)

The `SecureSlab` is the foundation of zero-copy data transmission in HTTP-X.

### HugeTLB Contiguity
In **Production Mode**, the slab uses 2MB HugePages (`MAP_HUGETLB`). This reduces the Translation Lookaside Buffer (TLB) pressure. By using larger page table entries, we minimize the depth of page table walks, which is a critical source of jitter during high-throughput packet bursts.

### Hardware Guard Gates
Slots in the `SecureSlab` are not just contiguous memory blocks; they are separated by `PROT_NONE` guard pages. 
*   **Safety Logic**: Any out-of-bounds (OOB) access by the application or a buffer overrun during header patching triggers an immediate hardware `SIGSEGV`.
*   **Security**: This prevents data leakage between concurrent sessions and ensures that even an exploit in the codec layer cannot read adjacent session data.

## 2. The Transport Engine (io_uring)

HTTP-X bypasses the legacy POSIX socket API by using a non-blocking `io_uring` submission/completion plane.

### SQPOLL (Submission Polling)
When `production_mode` is active, the transport engine utilizes `IORING_SETUP_SQPOLL`. A dedicated kernel thread polls the submission queue, meaning the `CoreDispatcher` performs NO syscalls to send data. It simply writes to the ring and the kernel "picks up" the packet from shared memory.

### GSO Zero-Copy Batching
The `GsoPacketizer` utilizes `UDP_SEGMENT` (GSO) and zero-copy fixed buffers. Instead of copying data into the kernel, `io_uring` reads directly from the `SecureSlab`. We assemble "Super-Packets" that are fragmented by the NIC hardware, resulting in a 3x reduction in ring entry consumption compared to standard linked chains.

## 3. The Intelligence Layer (LinearIntentTrie)

Predictions are executed in $O(k)$ time using a cache-line aligned bitwise search.

### Trie Bitmasking
Each node in the `LinearIntentTrie` is exactly 64 bytes (`#[repr(align(64))]`), matching the standard x86/ARM64 cache line. We use bit-level branching instead of character comparison, allowing the CPU to resolve path intent with minimal branch mispredictions.

### Shadow-Swap & Epoch Protection
To ensure the data path never stalls, we implement a **Shadow-Swap** mechanism:
1.  **Orchestration**: The `ClusterOrchestrator` accumulates weights on a "Shadow Trie."
2.  **Handshake**: When convergence is reached, it broadcasts a `SwapTrie` signal.
3.  **Atomic Swap**: Data plane workers perform a pointer swap using `crossbeam-epoch`.
4.  **Reclamation**: The old Trie is only dropped once all active worker threads move to a new epoch, ensuring no "use-after-free" on the hot path.

## 4. The Reliability Plane: IIW & Hysteresis

### Initial Intent Window (IIW)
Each session manages credits. A "Speculative Push" consumes 1 credit. Credits are regained only when the system confirms the packet reached the wire without triggering congestion backpressure.

### Hysteresis Monitoring
If weights flap too frequently, the **Sovereign Hysteresis Monitor** locks the weights until a stable pattern is detected. This prevents "learning oscillation" during network partitions.

---
*End of Technical Specification.*

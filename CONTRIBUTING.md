# CONTRIBUTING.md: The Sovereign Protocol

Welcome to the Sovereign Project. We build for the hardware, not the abstraction.

## The Sovereign Standards

Every PR is audited against the **Mechanical Sympathy Checklist**. If your code violates these rules, it will be rejected.

1.  **No Hot-Path Allocations**: Heap allocations (`Box`, `Vec`, `String`) are forbidden in the `CoreDispatcher::run_loop` or `on_packet` paths.
2.  **Zero-Copy Consistency**: Data must never be copied. Pass memory handles (Slab indices) or raw pointers within mapped regions.
3.  **Cache-Line Alignment**: Interactive data structures must be `#[repr(align(64))]` to prevent false sharing and ensure single-access fetches.
4.  **Atomic Linking**: Respect the 3-link SQE intent protocol for non-GSO hardware.

## Feedback & Participation

### Reporting "Latency Violations"
We do not use generic bug reports. If you encounter a scenario where First-Byte Latency exceeds 10µs on isolated hardware, file a **Latency Violation** report. Include:
*   `perf stat` output.
*   NIC model and Driver information.
*   NUMA topology (`lscpu` or `numactl -H`).

### High-Priority Challenges (The Roadmap)
We are actively seeking experts for the following:
*   **XDP/eBPF Ingress**: Moving the first filter layer into the NIC driver space.
*   **ARM64 Neon Opts**: Porting Trie bit-manipulation to 128-bit SIMD registers.
*   **Zero-Trust SAI**: Hardware-accelerated AEAD encryption within the `io_uring` ring.

---

*Join us in building the most sympathetic transport layer in history.*

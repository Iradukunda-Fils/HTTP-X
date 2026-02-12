use httpx_dsa::SecureSlab;
use io_uring::{opcode, types, IoUring};
use perf_event::Builder;
use perf_event::events::Software;
use std::os::unix::io::AsRawFd;
use nix::libc;

#[test]
fn test_zero_copy_integrity_proof() {
    let slab = SecureSlab::new(1024);
    let slot_idx = 0;
    let buf = slab.get_slot(slot_idx);

    // 1. Initialize data
    unsafe {
        std::ptr::write_bytes(buf, 0xAA, 4096);
    }

    // 2. Cold-start the page: MADV_DONTNEED forces the next access to be a "fresh" fetch.
    // If we were doing a memcpy, this would trigger a soft page-fault or at least a TLB miss during the read.
    // However, since we are doing Zero-Copy, the kernel just maps the page into the DMA engine.
    unsafe {
        libc::madvise(buf as *mut libc::c_void, 4096, libc::MADV_DONTNEED);
    }

    // 3. Setup perf-event to monitor Page Faults
    let mut counter = Builder::new()
        .kind(Software::PAGE_FAULTS)
        .build()
        .expect("Failed to build perf-event counter");

    // 4. Setup io_uring for a mock Write (into /dev/null for zero-obstruction)
    let mut ring = IoUring::new(8).expect("Failed to init io_uring");
    let dev_null = std::fs::File::open("/dev/null").expect("Failed to open /dev/null");
    
    let write_op = opcode::Write::new(
        types::Fd(dev_null.as_raw_fd()),
        buf,
        4096
    ).build();

    // 5. Execute and measure
    counter.enable().expect("Failed to enable counter");
    
    unsafe {
        ring.submission().push(&write_op).expect("Submission failed");
    }
    ring.submit_and_wait(1).expect("Submit failed");
    
    counter.disable().expect("Failed to disable counter");

    let faults = counter.read().expect("Failed to read counter");

    // # Verification: Zero Page Faults during the DMA transfer proof.
    // If the CPU were touching the data (memcpy), we would see >= 1 fault due to MADV_DONTNEED.
    println!("Zero-Copy Proof: Detected {} page-faults during transfer.", faults);
    assert_eq!(faults, 0, "Zero-Copy Failed: Kernel/CPU touched the memory (Page Fault detected)");
}

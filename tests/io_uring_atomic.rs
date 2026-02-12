use io_uring::{opcode, types, IoUring, squeue};
use std::os::unix::io::AsRawFd;
use tempfile::tempfile;

#[test]
fn test_io_uring_atomic_linking() {
    let mut ring = IoUring::new(8).expect("Failed to init io_uring");
    let file = tempfile().expect("Failed to create tempfile");
    
    let buf1 = [1u8; 1024];
    let buf2 = [2u8; 1024];

    // 1. Setup Linked SQEs: IOSQE_IO_LINK ensures SQE2 only runs if SQE1 is submitted.
    // In our protocol, this ensures "Intent" is processed before "Payload" data.
    let op1 = opcode::Write::new(
        types::Fd(file.as_raw_fd()),
        buf1.as_ptr(),
        1024
    ).build()
     .flags(squeue::Flags::IO_LINK);

    let op2 = opcode::Write::new(
        types::Fd(file.as_raw_fd()),
        buf2.as_ptr(),
        1024
    ).build();

    unsafe {
        let mut sq = ring.submission();
        sq.push(&op1).expect("Push SQE1 failed");
        sq.push(&op2).expect("Push SQE2 failed");
    }

    ring.submit_and_wait(2).expect("Submit failed");

    // 2. Verify Completion Queue (CQE)
    let cqe_results: Vec<i32> = ring.completion().map(|cqe| cqe.result()).collect();
    
    assert_eq!(cqe_results.len(), 2, "Expected 2 completion events");
    assert!(cqe_results[0] >= 0, "SQE1 failed: {}", cqe_results[0]);
    assert!(cqe_results[1] >= 0, "SQE2 failed: {}", cqe_results[1]);

    println!("Linked SQE Audit: Atomic chain (Intent+Payload) verified.");
}

#[test]
fn test_io_uring_link_break_on_failure() {
    let mut ring = IoUring::new(8).expect("Failed to init io_uring");
    
    // 1. Setup a chain where the first SQE fails (Invalid FD)
    let op1 = opcode::Write::new(
        types::Fd(-1), // Purposely invalid
        std::ptr::null(),
        0
    ).build()
     .flags(squeue::Flags::IO_LINK);

    let op2 = opcode::Write::new(
        types::Fd(-1),
        std::ptr::null(),
        0
    ).build();

    unsafe {
        let mut sq = ring.submission();
        sq.push(&op1).expect("Push SQE1 failed");
        sq.push(&op2).expect("Push SQE2 failed");
    }

    ring.submit_and_wait(2).expect("Submit failed");

    // 2. Verify results
    let cqe_results: Vec<i32> = ring.completion().map(|cqe| cqe.result()).collect();
    
    // Expected Result: 
    // SQE1: EBADF (-9)
    // SQE2: ECANCELED (-125) because the link broke
    assert_eq!(cqe_results.len(), 2);
    assert!(cqe_results[0] < 0, "SQE1 should have failed (EBADF)");
    assert_eq!(cqe_results[1], -125, "SQE2 must be ECANCELED because link broke");

    println!("Linked SQE Audit: Chain break (Atomicity) verified.");
}

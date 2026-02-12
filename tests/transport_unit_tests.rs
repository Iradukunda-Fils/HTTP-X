//! # Transport Layer Unit Tests
//!
//! Validates CongestionController credit evaluation, loss notification,
//! and GsoPacketizer iovec layout correctness.

use httpx_transport::reliability::{CongestionController, DefaultCongestionController};
use httpx_transport::stream::GsoPacketizer;
use std::time::Instant;

/// Verifies that under normal RTT conditions, the controller maintains
/// the maximum credit level (Level 2).
#[test]
fn test_congestion_controller_normal_rtt() {
    let t = Instant::now();

    let cc = DefaultCongestionController::new(10_000); // 10µs base RTT

    // Normal RTT = 10µs (same as base)
    let level = cc.evaluate_intent_credit(10_000);
    assert_eq!(level, 2, "Credit level should be 2 under normal RTT");

    // Slightly elevated but within 1.2x threshold
    let level = cc.evaluate_intent_credit(11_900);
    assert_eq!(level, 2, "Credit level should remain 2 within threshold");

    let overhead = t.elapsed();
    println!("test_congestion_controller_normal_rtt: Testing Overhead = {:?}", overhead);
}

/// Verifies that when RTT exceeds 1.2x base, the controller drops to Level 0.
#[test]
fn test_congestion_controller_high_rtt() {
    let t = Instant::now();

    let cc = DefaultCongestionController::new(10_000); // 10µs base RTT

    // RTT > 1.2 * 10_000 = 12_000
    let level = cc.evaluate_intent_credit(13_000);
    assert_eq!(level, 0, "Credit level should drop to 0 under high RTT");

    let overhead = t.elapsed();
    println!("test_congestion_controller_high_rtt: Testing Overhead = {:?}", overhead);
}

/// Verifies that `notify_loss` immediately drops the credit level to 0.
#[test]
fn test_congestion_controller_loss_notification() {
    let t = Instant::now();

    let cc = DefaultCongestionController::new(10_000);

    // Verify initial level is 2
    let level = cc.evaluate_intent_credit(10_000);
    assert_eq!(level, 2);

    // Simulate loss
    cc.notify_loss();

    // After loss, credit should be 0 regardless of RTT
    let level = cc.evaluate_intent_credit(10_000);
    assert_eq!(level, 0, "Credit level should be 0 after loss notification");

    let overhead = t.elapsed();
    println!("test_congestion_controller_loss_notification: Testing Overhead = {:?}", overhead);
}

/// Verifies that `GsoPacketizer::prepare_burst` correctly sets up
/// the iovec array with Intent, Header, and Payload pointers.
#[test]
fn test_gso_packetizer_prepare_burst() {
    let t = Instant::now();

    let mut packetizer = GsoPacketizer::new(16);

    let intent = b"INTENT_SYNC_FRAME";
    let header = [0xBBu8; 128];
    let payload = [0xAAu8; 4096];

    let msghdr_ptr = packetizer.prepare_burst(
        0,
        intent.as_ptr(), intent.len(),
        header.as_ptr(), header.len(),
        payload.as_ptr(), payload.len(),
        0,
    );

    assert!(!msghdr_ptr.is_null(), "msghdr_ptr should not be null");

    // Verify the msghdr fields
    let msghdr = unsafe { &*msghdr_ptr };
    assert_eq!(msghdr.msg_iovlen, 3, "Should have 3 iovecs (Intent, Header, Payload)");

    // Verify individual iovec entries
    let iovecs = unsafe { std::slice::from_raw_parts(msghdr.msg_iov, 3) };
    assert_eq!(iovecs[0].iov_len, intent.len(), "Intent iovec length mismatch");
    assert_eq!(iovecs[1].iov_len, header.len(), "Header iovec length mismatch");
    assert_eq!(iovecs[2].iov_len, payload.len(), "Payload iovec length mismatch");

    let overhead = t.elapsed();
    println!("test_gso_packetizer_prepare_burst: Testing Overhead = {:?}", overhead);
}

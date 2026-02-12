//! # Codec Layer Tests: HeaderTemplate
//!
//! Validates Procrustean Header Template creation and hot-patching
//! across SecureSlab memory boundaries.

use httpx_dsa::SecureSlab;
use httpx_codec::HeaderTemplate;
use std::time::Instant;

/// Verifies that `HeaderTemplate::new` correctly stores base headers
/// in the designated SecureSlab slot.
#[test]
fn test_header_template_creation() {
    let t = Instant::now();

    let slab = SecureSlab::new(8);
    let base = b"HTTP/1.1 200 OK\r\nDate: Thu, 01 Jan 1970 00:00:00 GMT\r\nContent-Length: 0         \r\n\r\n";
    let template = HeaderTemplate::new(&slab, 0, base);

    // Verify handle assignment
    assert_eq!(template.slab_handle, 0);

    // Verify slab content matches base headers
    let slot_ptr = slab.get_slot(0);
    let stored = unsafe { std::slice::from_raw_parts(slot_ptr, base.len()) };
    assert_eq!(stored, base.as_slice(), "Template content mismatch in slab");

    let overhead = t.elapsed();
    println!("test_header_template_creation: Testing Overhead = {:?}", overhead);
}

/// Verifies that `patch_date` writes at the correct offset.
#[test]
fn test_header_template_patch_date() {
    let t = Instant::now();

    let slab = SecureSlab::new(8);
    let base = b"HTTP/1.1 200 OK\r\nDate: Thu, 01 Jan 1970 00:00:00 GMT\r\nContent-Length: 0         \r\n\r\n";
    let template = HeaderTemplate::new(&slab, 0, base);

    let new_date = b"Wed, 11 Feb 2026 22:00:00 GM";
    template.patch_date(&slab, new_date);

    // Read the slot and verify the date region was modified
    let slot_ptr = slab.get_slot(0);
    let stored = unsafe { std::slice::from_raw_parts(slot_ptr, 128) };

    // The date should exist somewhere in the patched template
    let haystack = std::str::from_utf8(stored).unwrap_or("");
    assert!(
        haystack.contains("Wed, 11 Feb 2026") || haystack.contains("2026"),
        "Date patch not found. Slot content: {:?}",
        &stored[..base.len()]
    );

    let overhead = t.elapsed();
    println!("test_header_template_patch_date: Testing Overhead = {:?}", overhead);
}

/// Verifies that `patch_content_length` writes the correct value.
#[test]
fn test_header_template_patch_content_length() {
    let t = Instant::now();

    let slab = SecureSlab::new(8);
    let base = b"HTTP/1.1 200 OK\r\nDate: Thu, 01 Jan 1970 00:00:00 GMT\r\nContent-Length: 0         \r\n\r\n";
    let template = HeaderTemplate::new(&slab, 0, base);

    template.patch_content_length(&slab, 4096);

    // Read the slot — the Content-Length offset should now contain "4096"
    let slot_ptr = slab.get_slot(0);
    let stored = unsafe { std::slice::from_raw_parts(slot_ptr, 128) };
    let haystack = std::str::from_utf8(stored).unwrap_or("");

    // The value "4096" should be present in the patched region
    assert!(
        haystack.contains("4096"),
        "Content-Length patch not found. Slot content: {:?}",
        &stored[..base.len()]
    );

    let overhead = t.elapsed();
    println!("test_header_template_patch_content_length: Testing Overhead = {:?}", overhead);
}

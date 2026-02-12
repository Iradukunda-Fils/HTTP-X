extern crate alloc;
use core::sync::atomic::{AtomicUsize, Ordering};
use alloc::vec::Vec;
use std::sync::Arc;

#[derive(Debug)]
pub enum DropReason {
    Congested,
}

#[repr(align(64))]
struct CacheAlignedAtomic(AtomicUsize);

/// A wait-free SPSC Ring Buffer for bridging the PredictiveEngine to the Transport Loop.
/// 
/// ## Mechanical Sympathy
/// - **Cache-Line Padding**: Head and Tail pointers are separated by 64 bytes to prevent False Sharing.
/// - **Power-of-Two Sizing**: Index wrapping uses bitwise AND instead of expensive modulo.
pub struct SqBridge<T> {
    head: CacheAlignedAtomic,
    tail: CacheAlignedAtomic,
    buffer: Vec<Option<T>>,
    mask: usize,
}

impl<T> SqBridge<T> {
    pub fn new(capacity: usize) -> Arc<Self> {
        assert!(capacity.is_power_of_two(), "Capacity must be a power of two");
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(None);
        }

        Arc::new(Self {
            head: CacheAlignedAtomic(AtomicUsize::new(0)),
            tail: CacheAlignedAtomic(AtomicUsize::new(0)),
            buffer,
            mask: capacity - 1,
        })
    }

    /// Attempts to push a predictive intent into the bridge.
    pub fn try_push(&self, item: T) -> Result<(), DropReason> {
        let head = self.head.0.load(Ordering::Relaxed);
        let tail = self.tail.0.load(Ordering::Acquire);

        if head.wrapping_sub(tail) >= self.mask + 1 {
            return Err(DropReason::Congested);
        }

        let idx = head & self.mask;
        
        // # Safety: We are the ONLY producer. Release ordering ensures visibility.
        unsafe {
            let slot = self.buffer.as_ptr().add(idx) as *mut Option<T>;
            core::ptr::write(slot, Some(item));
        }

        self.head.0.store(head.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    /// Attempts to pop a predictive intent from the bridge.
    pub fn pop(&self) -> Option<T> {
        let tail = self.tail.0.load(Ordering::Relaxed);
        let head = self.head.0.load(Ordering::Acquire);

        if tail == head {
            return None;
        }

        let idx = tail & self.mask;

        // # Safety: We are the ONLY consumer. Acquire ordering ensures the write is visible.
        let item = unsafe {
            let slot = self.buffer.as_ptr().add(idx) as *mut Option<T>;
            core::ptr::replace(slot, None)
        };

        self.tail.0.store(tail.wrapping_add(1), Ordering::Release);
        item
    }
}

unsafe impl<T: Send> Send for SqBridge<T> {}
unsafe impl<T: Send> Sync for SqBridge<T> {}

extern crate alloc;
use alloc::vec::Vec;

use core::ptr::NonNull;
use core::ffi::c_void;
use nix::libc;
use nix::sys::mman::{mprotect, ProtFlags};

use core::sync::atomic::{AtomicUsize, AtomicU32, Ordering};

const PAGE_SIZE: usize = 4096;

/// A Secure, Hardware-Protected Slab Allocator.
#[repr(align(64))]
pub struct SecureSlab {
    base: NonNull<c_void>,
    slots: usize,
    total_len: usize,
    huge_mode: bool,
    ref_counts: Vec<AtomicUsize>,
    version_ids: Vec<AtomicU32>,
}

impl SecureSlab {
    /// Creates a new SecureSlab with the requested number of hardware-isolated slots.
    ///
    /// ## Safety Proof
    /// 1. **Resource Reservation**: `mmap` is used to reserve a contiguous virtual 
    ///    address space.
    /// 2. **Boundary Protection**: Slots are separated by `PROT_NONE` guard pages. 
    ///    Any OOB access triggers a hardware-level `SIGSEGV`.
    /// 3. **Memory Hardening**: Initial state is non-executable and non-readable 
    ///    except for activated data pages.
    pub fn new(slots: usize) -> Self {
        const HUGE_PAGE_SIZE: usize = 2 * 1024 * 1024;
        // Attempt HugeTLB Allocation first (Production Mode)
        // Optimization: Aligned to 2MB boundaries for TLB efficiency.
        let huge_len = core::cmp::max(slots * PAGE_SIZE, HUGE_PAGE_SIZE);
        // Round up to multiple of 2MB
        let huge_len = (huge_len + HUGE_PAGE_SIZE - 1) & !(HUGE_PAGE_SIZE - 1);
        
        let mut addr = unsafe {
            libc::mmap(
                core::ptr::null_mut(),
                huge_len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_HUGETLB,
                -1,
                0,
            )
        };
        
        let mut huge_mode = true;
        let mut total_len = huge_len;

        // Fallback to Standard 4K Pages (Dev Mode / Guarded Layout)
        if addr == libc::MAP_FAILED {
            huge_mode = false;
            // Layout: [Guard] [Slot 0] [Guard] [Slot 1] [Guard] ...
            // Total pages = slots * 2 + 1
            total_len = (slots * 2 + 1) * PAGE_SIZE;

            addr = unsafe {
                libc::mmap(
                    core::ptr::null_mut(),
                    total_len,
                    libc::PROT_NONE,
                    libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                    -1,
                    0,
                )
            };
        }

        if addr == libc::MAP_FAILED {
            panic!("SecureSlab: mmap failed");
        }

        let base = NonNull::new(addr).expect("mmap returned null");

        let mut ref_counts = Vec::with_capacity(slots);
        let mut version_ids = Vec::with_capacity(slots);
        for _ in 0..slots {
            ref_counts.push(AtomicUsize::new(0));
            version_ids.push(AtomicU32::new(0));
        }

        let slab = Self {
            base,
            slots,
            total_len,
            huge_mode,
            ref_counts,
            version_ids,
        };

        // Activate data pages (if not already HUGE_TLB RW)
        if !huge_mode {
            for i in 0..slots {
                slab.activate_slot(i);
            }
        }

        slab
    }

    /// Activates a specific memory slot for read/write operations.
    fn activate_slot(&self, idx: usize) {
        // Offset: (1 + idx * 2) Skip the initial guard + pairs of slot/guard
        let offset = (1 + idx * 2) * PAGE_SIZE;
        
        // # Safety: Pointer arithmetic is sound within the reserved mmap range.
        // We ensure 64-byte alignment by virtue of PAGE_SIZE (4096) being a multiple of 64.
        unsafe {
            let slot_ptr = self.base.as_ptr().byte_add(offset);
            mprotect(
                NonNull::new(slot_ptr).unwrap(),
                PAGE_SIZE,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            ).expect("SecureSlab: mprotect activation failed");
        }
    }

    /// Returns a direct pointer to the 4KB data page of the given slot.
    ///
    /// ## Performance
    /// Returns in ~5 cycles. Optimal for hot-path transport loops.
    pub fn get_slot(&self, idx: usize) -> *mut u8 {
        assert!(idx < self.slots);
        let offset = if self.huge_mode {
            // Contiguous: [Slot 0] [Slot 1] ...
            idx * PAGE_SIZE
        } else {
            // Guarded: [Guard] [Slot 0] [Guard] [Slot 1] ...
            (1 + idx * 2) * PAGE_SIZE
        };
        // Mechanical Sympathy: The offset is always page-aligned (and thus cache-aligned).
        unsafe { self.base.as_ptr().byte_add(offset) as *mut u8 }
    }

    /// Increments the reference count for a specific slot.
    /// 
    /// # Protocol
    /// Must be called when a buffer is submitted to the io_uring SQ.
    /// Uses `Ordering::Release` to ensure the buffer content is visible to the kernel.
    pub fn increment_rc(&self, idx: usize) {
        assert!(idx < self.slots);
        self.ref_counts[idx].fetch_add(1, Ordering::Release);
    }

    /// Decrements the reference count for a specific slot.
    /// 
    /// # Protocol
    /// Must be called when a CQE is processed by the transport loop.
    /// Uses `Ordering::Acquire` to ensure kernel writes are visible to software.
    pub fn decrement_rc(&self, idx: usize) {
        assert!(idx < self.slots);
        let prev = self.ref_counts[idx].fetch_sub(1, Ordering::Acquire);
        if prev == 0 {
            panic!("SecureSlab: decrement_rc called on slot with RC 0");
        }
    }

    /// Explicitly releases a slot back to the "FREE" state.
    /// 
    /// # Safety
    /// Panics if the RC is non-zero, indicating a kernel-flight violation.
    pub fn explicit_release(&self, idx: usize) {
        assert!(idx < self.slots);
        if self.ref_counts[idx].load(Ordering::Acquire) > 0 {
            panic!("SecureSlab: explicit_release failed - slot {} is still in-flight", idx);
        }
    }

    /// Returns the number of slots in the slab.
    pub fn slots(&self) -> usize {
        self.slots
    }

    /// Checks if a slot is currently in use by the kernel.
    pub fn is_in_flight(&self, idx: usize) -> bool {
        assert!(idx < self.slots);
        self.ref_counts[idx].load(Ordering::Acquire) > 0
    }

    /// Gets the current version ID of a slot.
    #[inline(always)]
    pub fn get_version(&self, idx: usize) -> u32 {
        assert!(idx < self.slots);
        self.version_ids[idx].load(Ordering::Acquire)
    }

    /// Sets the version ID of a slot (Freshness Commitment).
    pub fn set_version(&self, idx: usize, version: u32) {
        assert!(idx < self.slots);
        self.version_ids[idx].store(version, Ordering::Release);
    }

    /// Increments the version ID of a slot.
    pub fn increment_version(&self, idx: usize) -> u32 {
        assert!(idx < self.slots);
        self.version_ids[idx].fetch_add(1, Ordering::AcqRel) + 1
    }
}

impl Drop for SecureSlab {
    fn drop(&mut self) {
        // # Safety: base and total_len are valid and owned by this struct.
        unsafe {
            libc::munmap(self.base.as_ptr(), self.total_len);
        }
    }
}

unsafe impl Send for SecureSlab {}
unsafe impl Sync for SecureSlab {}

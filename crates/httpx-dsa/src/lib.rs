#![no_std]
extern crate alloc;

pub mod trie;
pub mod slab;
pub mod numa;

pub use trie::LinearIntentTrie;
pub use slab::SecureSlab;
pub use numa::NumaPinnedSlab;

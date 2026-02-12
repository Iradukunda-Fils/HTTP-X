use httpx_dsa::LinearIntentTrie;

/// The ResourceRegistry bridges application URIs to the Fast-Path engine.
/// 
/// ## Mechanical Sympathy: The Trie-Warmer
/// Registration "burns" URI segments into the LinearIntentTrie nodes. 
/// This eliminates the need for dynamic string matching and allocation
/// during the sub-8µs data-path hot-loop.
pub struct ResourceRegistry {
    trie: LinearIntentTrie,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self {
            trie: LinearIntentTrie::new(1024),
        }
    }

    /// Registers a route and pre-populates its bit-path in the trie.
    ///
    /// ## Constraint: No Dynamic Dispatch
    /// We use u32 handles for payloads and templates, preserving the
    /// zero-blocking static resolution model.
    pub fn route(&mut self, path: &str, payload_handle: u32, version_id: u32) {
        let bytes = path.as_bytes();
        
        // 1. Warm the trie: Ensure all segments exist in the radix structure.
        self.trie.warm(bytes);
        
        // 2. Associate payload: Bind the handle and version to the terminal node.
        self.trie.associate_payload(bytes, payload_handle, version_id);
    }

    /// Consumes the registry and returns the fully warmed trie.
    pub fn take_trie(self) -> LinearIntentTrie {
        self.trie
    }
}

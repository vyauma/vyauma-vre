use region::{protect, Protection};
use std::alloc::{alloc, dealloc, Layout};

/// Manages executable memory mapped for the JIT compiler.
#[derive(Debug)]
pub struct JitMemory {
    ptr: *mut u8,
    #[allow(dead_code)]
    size: usize,
    layout: Layout,
}

impl JitMemory {
    /// Allocates page-aligned memory, copies the given machine code bytes into it,
    /// and marks the memory region as Read, Write, and Execute (RWX).
    pub fn new(code: &[u8]) -> Self {
        let size = code.len();
        // Ensure the allocation size is at least 1 and aligned to the OS page size
        let layout = Layout::from_size_align(size.max(1), region::page::size()).unwrap();
        let ptr = unsafe { alloc(layout) };
        if ptr.is_null() {
            panic!("JIT memory allocation failed");
        }
        unsafe {
            std::ptr::copy_nonoverlapping(code.as_ptr(), ptr, size);
            // Protect memory to be Read/Write/Execute
            protect(ptr, size, Protection::READ_WRITE_EXECUTE).expect("Failed to protect JIT memory");
        }
        Self { ptr, size, layout }
    }

    /// Returns a raw pointer to the start of the executable memory.
    pub fn get_ptr(&self) -> *const u8 {
        self.ptr
    }
}

impl Drop for JitMemory {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.ptr, self.layout);
        }
    }
}

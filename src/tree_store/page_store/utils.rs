#[cfg(unix)]
pub(crate) fn get_page_size() -> usize {
    use libc::{sysconf, _SC_PAGESIZE};

    unsafe { sysconf(_SC_PAGESIZE) as usize }
}

#[cfg(not(unix))]
pub(crate) fn get_page_size() -> usize {
    4096
}

// Helper method to explicitly drop a reference to memory to ensure it is not used later, without
// triggering Clippy warnings
pub(crate) fn drop_ref<T>(_value: T) {}

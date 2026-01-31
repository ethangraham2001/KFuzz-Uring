use nix::{ioctl_read, libc, request_code_none};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::ptr;
use std::slice;
use std::sync::atomic::{AtomicU64, Ordering};

// Constants
const KCOV_MODE_DISABLED: libc::c_int = 0;
const KCOV_MODE_TRACE_PC: libc::c_int = 0;
const KCOV_FILE: &str = "/sys/kernel/debug/kcov";
pub const KCOV_COVER_SIZE: usize = 64 << 10;

ioctl_read!(kcov_init_trace, 'c', 1, libc::c_ulong);

#[derive(Debug)]
pub enum KCOVError {
    Initialization(String),
    Ioctl(String),
    Mmap(String),
}

impl fmt::Display for KCOVError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            KCOVError::Initialization(s) => write!(f, "Init error: {}", s),
            KCOVError::Ioctl(s) => write!(f, "Ioctl error: {}", s),
            KCOVError::Mmap(s) => write!(f, "Mmap error: {}", s),
        }
    }
}

impl std::error::Error for KCOVError {}

pub struct KCOVState {
    // Keep file explicitly to prevent it from closing
    #[allow(dead_code)]
    file: File,
    // Using AtomicU64 pointer for correct semantics
    cover: *mut AtomicU64,
    // Store size for unmapping
    size: usize,
}

// Ensure KCOVState is not sent to other threads unless safe (pointers are !Send)
// KCOV is usually thread-local, but if you need to move it:
unsafe impl Send for KCOVState {}

impl KCOVState {
    pub fn new() -> Result<Self, KCOVError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(KCOV_FILE)
            .map_err(|e| KCOVError::Initialization(format!("failed to open kcov: {e}")))?;

        // KCOV_INIT_TRACE expects the size as a value. We cast size to pointer
        // to satisfy nix's signature, knowing the kernel reads the 'arg'
        // register directly.
        unsafe {
            if let Err(e) = kcov_init_trace(file.as_raw_fd(), KCOV_COVER_SIZE as *mut libc::c_ulong)
            {
                return Err(KCOVError::Ioctl(format!("failed to init trace: {e}")));
            }
        }

        // 2. Mmap
        let mmap_len = KCOV_COVER_SIZE * std::mem::size_of::<libc::c_ulong>();
        let cover_ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                mmap_len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                file.as_raw_fd(),
                0,
            )
        };

        if cover_ptr == libc::MAP_FAILED {
            let err = std::io::Error::last_os_error();
            return Err(KCOVError::Mmap(format!("failed to mmap: {err}")));
        }

        Ok(Self {
            file,
            cover: cover_ptr as *mut AtomicU64,
            size: mmap_len,
        })
    }

    pub fn enable(&self) -> Result<(), KCOVError> {
        unsafe {
            (*self.cover).store(0, Ordering::Relaxed);
        }

        let request = request_code_none!('c', 100);
        unsafe {
            let res = libc::ioctl(self.file.as_raw_fd(), request, KCOV_MODE_TRACE_PC);
            if res != 0 {
                let err = std::io::Error::last_os_error();
                return Err(KCOVError::Ioctl(format!("failed to enable: {err}")));
            }
        }
        Ok(())
    }

    pub fn disable(&self) -> Result<(), KCOVError> {
        let request = request_code_none!('c', 101);
        unsafe {
            let res = libc::ioctl(self.file.as_raw_fd(), request, KCOV_MODE_DISABLED);
            if res != 0 {
                let err = std::io::Error::last_os_error();
                return Err(KCOVError::Ioctl(format!("failed to disable: {err}")));
            }
        }
        Ok(())
    }

    pub fn collect(&self) -> Vec<u64> {
        unsafe {
            let n_records = (*self.cover).load(Ordering::Relaxed) as usize;
            if n_records > KCOV_COVER_SIZE {
                return Vec::new();
            }

            let ptr_start = (self.cover as *const u64).add(1);
            let slice = slice::from_raw_parts(ptr_start, n_records);

            slice.to_vec()
        }
    }
}

impl Drop for KCOVState {
    fn drop(&mut self) {
        unsafe {
            if !self.cover.is_null() {
                libc::munmap(self.cover as *mut libc::c_void, self.size);
            }
        }
    }
}

use std::io::{self, Write};
use std::os::fd::AsRawFd;

use crate::kcov::{self, KCOVState};
use crate::kfuzz_target::KFuzzTarget;

pub trait KFuzzExecutor {
    fn run<'a>(&'a mut self, target: &mut KFuzzTarget, input: &[u8]) -> &'a [u64];
    fn new() -> Self;
}

macro_rules! _IO {
    ($type:expr, $nr:expr) => {
        (0 << 30) | (($type as u32) << 8) | (($nr as u32) << 0) | (0 << 16)
    };
}

const KFUZZ_IOC_MAGIC: u8 = b'K';
const KFUZZ_IOC_RUN: u32 = _IO!(KFUZZ_IOC_MAGIC, 1);

pub struct KFuzzSimpleExecutor {
    kcov_state: KCOVState,
    coverage_buffer: Vec<u64>,
}

impl KFuzzExecutor for KFuzzSimpleExecutor {
    fn new() -> Self {
        Self {
            kcov_state: KCOVState::new().expect("unable to initialize KCOVState"),
            coverage_buffer: Vec::with_capacity(kcov::KCOV_COVER_SIZE),
        }
    }

    fn run<'a>(&'a mut self, target: &mut KFuzzTarget, input: &[u8]) -> &'a [u64] {
        self.kcov_state
            .enable()
            .map_err(|e| {
                println!("KCOV failure code: {:?}", e);
                e
            })
            .expect("failed to enable");

        // Step 1: write data into `fuzz` file for this test suite.
        target
            .info
            .file
            .write_all(input)
            .expect("unable to write to file");

        // Step 2: ioctl the file so that we can detect errors.
        let res = unsafe {
            libc::ioctl(
                target.info.file.as_raw_fd(),
                KFUZZ_IOC_RUN as _,
                target.info.id,
            )
        };
        if res < 0 {
            panic!("unable to run ioctl: {}", io::Error::last_os_error());
        }

        self.kcov_state.disable().expect("failed to disable kcov");

        self.coverage_buffer.clear();
        let raw_data = self.kcov_state.collect();
        self.coverage_buffer.extend_from_slice(&raw_data);
        &self.coverage_buffer
    }
}

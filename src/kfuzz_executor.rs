use std::os::unix::fs::FileExt;

use crate::kcov::{self, KCOVState};
use crate::kfuzz_target::KFuzzTarget;

pub trait KFuzzExecutor {
    fn run<'a>(&'a mut self, target: &mut KFuzzTarget, input: &[u8]) -> &'a [u64];
    fn new() -> Self;
}

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
        target
            .file
            .write_at(input, 0)
            .expect("unable to write to file");
        self.kcov_state.disable().expect("failed to disable kcov");

        self.coverage_buffer.clear();
        let raw_data = self.kcov_state.collect();
        self.coverage_buffer.extend_from_slice(&raw_data);
        &self.coverage_buffer
    }
}

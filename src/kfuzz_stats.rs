use std::{
    sync::{
        Arc,
        atomic::{AtomicI64, AtomicUsize},
    },
    time::Instant,
};

pub struct KFuzzStats {
    pub start_time: Instant,
    pub total_execs: AtomicI64,
    pub total_coverage: AtomicUsize,
    pub total_corpus_size: AtomicUsize,
}

impl KFuzzStats {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            start_time: Instant::now(),
            total_execs: AtomicI64::new(0),
            total_coverage: AtomicUsize::new(0),
            total_corpus_size: AtomicUsize::new(0),
        })
    }
}

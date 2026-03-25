use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use crate::kfuzz_executor::KFuzzExecutor;
use crate::kfuzz_stats::KFuzzStats;
use crate::kfuzz_target::{CorpusEntry, KFuzzTarget, compute_energy};

pub struct KFuzzManager<E: KFuzzExecutor> {
    targets: Vec<KFuzzTarget>,
    executor: E,
    coverage_map: CoverageMap,
    pub stats: Arc<KFuzzStats>,
}

impl<E: KFuzzExecutor> KFuzzManager<E> {
    pub fn new(targets: Vec<KFuzzTarget>) -> Self {
        Self {
            targets: targets,
            executor: E::new(),
            coverage_map: CoverageMap::new(),
            stats: KFuzzStats::new(),
        }
    }

    pub fn fuzz(&mut self) {
        self.display_startup_info();
        start_monitor(self.stats.clone());

        loop {
            let mut total_corpus_size = 0;
            for target in &mut self.targets {
                let input = target.corpus.next_input();
                let trace = self.executor.run(target, &input);

                self.stats.total_execs.fetch_add(1, Ordering::Relaxed);

                let new_features = self.coverage_map.add_trace(trace);
                if new_features > 0 {
                    let new_energy = compute_energy(input.len() as u64, new_features as u64);
                    let new_entry = CorpusEntry::new(input, new_energy);
                    target.corpus.add(new_entry);

                    self.stats
                        .total_coverage
                        .store(self.coverage_map.size(), Ordering::Relaxed);
                }
                total_corpus_size += target.corpus.size();
            }

            self.stats
                .total_corpus_size
                .store(total_corpus_size, Ordering::Relaxed);
        }
    }

    fn display_startup_info(&self) {
        println!("targets:");
        for target in self.targets.iter() {
            println!("\t{}", target.info.name);
        }
    }
}

pub fn start_monitor(stats: Arc<KFuzzStats>) {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(1));

            let elapsed = stats.start_time.elapsed().as_secs_f64();
            let execs = stats.total_execs.load(Ordering::Relaxed);
            let cov = stats.total_coverage.load(Ordering::Relaxed);
            let corpus = stats.total_corpus_size.load(Ordering::Relaxed);

            let execs_per_sec = if elapsed > 0.0 {
                execs as f64 / elapsed
            } else {
                0.0
            };

            println!(
                "[+] Time: {:5.1}s | Execs: {:8} | Speed: {:7.1}/s | Cov: {:5} | Corpus: {:5}",
                elapsed, execs, execs_per_sec, cov, corpus
            );
            std::io::stdout().flush().unwrap();
        }
    });
}

struct CoverageMap {
    global_features: HashSet<u64>,
    coverage_file: File,
}

impl CoverageMap {
    fn new() -> Self {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("coverage.out")
            .expect("failed to open coverage file");
        Self {
            global_features: HashSet::new(),
            coverage_file: file,
        }
    }

    pub fn size(&self) -> usize {
        self.global_features.len()
    }

    pub fn add_trace(&mut self, trace: &[u64]) -> usize {
        let mut new_pcs: Vec<u64> = Vec::new();
        for &pc in trace {
            if self.global_features.insert(pc) {
                new_pcs.push(pc);
            }
        }

        if !new_pcs.is_empty() {
            let mut buffer = String::new();
            for pc in &new_pcs {
                use std::fmt::Write;
                writeln!(&mut buffer, "{:#x}", pc).unwrap();
            }
            self.coverage_file
                .write_all(buffer.as_bytes())
                .expect("failed to write to coverage file");
            self.coverage_file
                .flush()
                .expect("failed to flush coverage file");
        }
        return new_pcs.len();
    }
}

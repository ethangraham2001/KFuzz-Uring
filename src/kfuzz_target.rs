use crate::blob_mutator::{Mutator, NaiveMutator};
use rand::{Rng, SeedableRng, rngs::SmallRng};
use std::fs::{self, File, OpenOptions};

pub struct KFuzzTarget {
    pub name: String,
    pub file: File,
    pub corpus: Corpus<NaiveMutator>,
    pub total_coverage: u64,
}

const KFUZZTEST_BASE_PATH: &str = "/sys/kernel/debug/kfuzztest";

pub fn find_kfuzztest_targets() -> Result<Vec<String>, std::io::Error> {
    let subdirs = fs::read_dir(KFUZZTEST_BASE_PATH)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    Ok(subdirs)
}

impl KFuzzTarget {
    pub fn new(name: &str) -> Result<Self, std::io::Error> {
        let path = std::path::Path::new(KFUZZTEST_BASE_PATH)
            .join(name)
            .join("input_simple");
        let file = OpenOptions::new().read(false).write(true).open(path)?;
        Ok(KFuzzTarget {
            name: String::from(name),
            file: file,
            corpus: Corpus::new(),
            total_coverage: 0,
        })
    }
}

#[derive(Clone)]
pub struct CorpusEntry {
    pub data: Vec<u8>,
    pub energy: u64,
}

const DEFAULT_ENERGY: u64 = 10;

impl CorpusEntry {
    pub fn default() -> Self {
        Self {
            data: vec![b'A'; 5],
            energy: DEFAULT_ENERGY,
        }
    }

    pub fn new(input: Vec<u8>, energy: u64) -> Self {
        Self {
            data: input,
            energy: energy,
        }
    }
}

pub fn compute_energy(_: u64, new_features: u64) -> u64 {
    const ENERGY_MULTIPLIER: u64 = 10;
    // For now, rank on the uniqueness of the input. Later on, worth considering
    // scaling this if the input is small.
    DEFAULT_ENERGY + new_features * ENERGY_MULTIPLIER
}

pub struct Corpus<M: Mutator> {
    entries: Vec<CorpusEntry>,
    mutator: M,
    rng: SmallRng,
}

impl<M: Mutator> Corpus<M> {
    pub fn new() -> Self {
        return Corpus {
            entries: vec![CorpusEntry::default()],
            mutator: M::new(),
            rng: SmallRng::from_entropy(),
        };
    }

    pub fn add(&mut self, entry: CorpusEntry) {
        self.entries.push(entry)
    }

    pub fn size(&self) -> usize {
        self.entries.len()
    }

    fn pick(&mut self) -> CorpusEntry {
        use rand::distributions::WeightedIndex;
        use rand::prelude::Distribution;
        let weights = self.entries.iter().map(|e| e.energy);
        let dist = WeightedIndex::new(weights).expect("Corpus weight error");
        let idx = dist.sample(&mut self.rng);
        self.entries[idx].clone()
    }

    pub fn next_input(&mut self) -> Vec<u8> {
        if self.entries.len() > 1 && self.rng.gen_bool(0.25) {
            self.splice_mutate()
        } else {
            self.mutate_once()
        }
    }

    fn mutate_once(&mut self) -> Vec<u8> {
        let mut entry = self.pick();
        self.mutator.mutate(&mut entry.data);
        entry.data
    }

    fn splice_mutate(&mut self) -> Vec<u8> {
        let parent1 = self.pick().data;
        let parent2 = self.pick().data;

        let cut1 = self.rng.gen_range(0..parent1.len());
        let cut2 = self.rng.gen_range(0..parent2.len());
        let mut new_data = Vec::with_capacity(cut1 + (parent2.len() - cut2));
        new_data.extend_from_slice(&parent1[..cut1]);
        new_data.extend_from_slice(&parent2[cut2..]);
        new_data
    }
}

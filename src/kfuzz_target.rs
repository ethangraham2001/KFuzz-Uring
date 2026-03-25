use crate::blob_mutator::{Mutator, NaiveMutator};
use rand::{Rng, SeedableRng, rngs::SmallRng};
use std::fs::{self, File, OpenOptions};
use std::io;

#[derive(Debug)]
pub struct KUnitInfo {
    pub file: File,
    pub suite: String,
    pub name: String,
    pub id: u32,
}

pub struct KFuzzTarget {
    pub info: KUnitInfo,
    pub corpus: Corpus<NaiveMutator>,
    pub total_coverage: u64,
}

const KUNIT_BASE_PATH: &str = "/sys/kernel/debug/kunit";

pub fn find_fuzz_targets() -> io::Result<Vec<KUnitInfo>> {
    let subdirs = fs::read_dir(KUNIT_BASE_PATH)?;
    let mut targets = Vec::new();

    for entry in subdirs {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let fuzz_path = path.join("fuzz");
        let content = match fs::read_to_string(&fuzz_path) {
            Ok(c) => c,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        };

        let suite = entry.file_name().to_string_lossy().to_string();

        for line in content.lines() {
            let line = line.trim();

            let Some((id_str, name_str)) = line.split_once(':') else {
                continue;
            };

            let Ok(id) = id_str.trim().parse::<u32>() else {
                continue;
            };

            let name = name_str.trim().to_string();
            let file = OpenOptions::new().write(true).open(&fuzz_path)?;
            targets.push(KUnitInfo {
                file,
                suite: suite.clone(),
                name,
                id,
            });
        }
    }
    Ok(targets)
}

impl KFuzzTarget {
    pub fn new(info: KUnitInfo, rng_seed: u64) -> Result<Self, std::io::Error> {
        Ok(KFuzzTarget {
            info: info,
            corpus: Corpus::new(rng_seed),
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
    pub fn new(rng_seed: u64) -> Self {
        return Corpus {
            entries: vec![CorpusEntry::default()],
            mutator: M::new_from_seed(rng_seed),
            rng: SmallRng::seed_from_u64(rng_seed),
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

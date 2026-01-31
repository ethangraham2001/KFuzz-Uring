use rand::distributions::Standard;
use rand::prelude::Distribution;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

pub trait Mutator {
    fn mutate(&mut self, data: &mut Vec<u8>);
    fn new() -> Self;
}

#[derive(Debug, Clone, Copy)]
enum MutationType {
    BitFlip,
    ByteOverwrite,
    InsertBlock,
    EraseBlock,
}

impl Distribution<MutationType> for Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> MutationType {
        match rng.gen_range(0..=3) {
            0 => MutationType::BitFlip,
            1 => MutationType::ByteOverwrite,
            2 => MutationType::InsertBlock,
            _ => MutationType::EraseBlock,
        }
    }
}

pub struct NaiveMutator {
    rng: SmallRng,
}

impl Mutator for NaiveMutator {
    fn mutate(&mut self, buf: &mut Vec<u8>) {
        // Stack 1 to 5 mutations.
        let mutator_count = self.rng.gen_range(0..=5);
        for _ in 0..mutator_count {
            self.handle_mutation(buf);
        }
    }

    fn new() -> Self {
        NaiveMutator {
            rng: SmallRng::from_entropy(),
        }
    }
}

impl NaiveMutator {
    fn handle_mutation(&mut self, buf: &mut Vec<u8>) {
        let t: MutationType = self.rng.r#gen();
        match t {
            MutationType::BitFlip => self.handle_bit_flip(buf),
            MutationType::ByteOverwrite => self.handle_byte_overwrite(buf),
            MutationType::InsertBlock => self.handle_insert_block(buf),
            MutationType::EraseBlock => self.handle_erase_block(buf),
        }
    }
    fn handle_bit_flip(&mut self, buf: &mut Vec<u8>) {
        if buf.is_empty() {
            return;
        }
        let idx: usize = self.rng.gen_range(0..buf.len());
        let mask: u8 = 1 << self.rng.gen_range(0..8);
        buf[idx] ^= mask;
    }

    fn handle_byte_overwrite(&mut self, buf: &mut Vec<u8>) {
        if buf.is_empty() {
            return;
        }
        let index = self.rng.gen_range(0..buf.len());
        buf[index] = self.rng.r#gen();
    }

    fn handle_erase_block(&mut self, buf: &mut Vec<u8>) {
        if buf.is_empty() {
            return;
        }
        let len = self.rng.gen_range(1..=buf.len().min(16));
        let start = self.rng.gen_range(0..=buf.len() - len);
        buf.drain(start..start + len);
    }

    fn handle_insert_block(&mut self, buf: &mut Vec<u8>) {
        let pos = self.rng.gen_range(0..=buf.len());
        let len = self.rng.gen_range(1..17);
        let random_iter = (0..len).map(|_| self.rng.r#gen::<u8>());
        buf.splice(pos..pos, random_iter);
    }
}

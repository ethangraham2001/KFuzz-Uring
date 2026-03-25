pub mod blob_mutator;
pub mod kcov;
pub mod kfuzz_executor;
pub mod kfuzz_manager;
pub mod kfuzz_stats;
pub mod kfuzz_target;

use rand::Rng;

use crate::kfuzz_executor::KFuzzSimpleExecutor;
use crate::kfuzz_manager::KFuzzManager;
use crate::kfuzz_target::{KFuzzTarget, find_fuzz_targets};

fn main() {
    let seed: u64 = rand::thread_rng().r#gen();
    println!("rng seed = {}", seed);

    let targets = find_fuzz_targets().unwrap();
    let my_target = targets
        .into_iter()
        // Assuming pkcs7 contains an injected bug, it serves as a good PoC
        // for the fuzzer's effectiveness.
        .find(|x| x.name == "fuzz_pkcs7_parse_message")
        .unwrap();

    let pkcs7_targ = KFuzzTarget::new(my_target, seed).unwrap();

    let mut manager = KFuzzManager::<KFuzzSimpleExecutor>::new(vec![pkcs7_targ]);
    manager.fuzz();
}

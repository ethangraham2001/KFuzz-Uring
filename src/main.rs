pub mod blob_mutator;
pub mod kcov;
pub mod kfuzz_executor;
pub mod kfuzz_manager;
pub mod kfuzz_stats;
pub mod kfuzz_target;

use crate::kfuzz_executor::KFuzzSimpleExecutor;
use crate::kfuzz_manager::KFuzzManager;

fn main() {
    let targets = vec![
        kfuzz_target::KFuzzTarget::new("test_pkcs7_parse_message").expect("could not load target"),
        kfuzz_target::KFuzzTarget::new("test_rsa_parse_pub_key").expect("could not load target"),
        kfuzz_target::KFuzzTarget::new("test_rsa_parse_priv_key").expect("could not load target"),
    ];
    let mut manager = KFuzzManager::<KFuzzSimpleExecutor>::new_from_targets(targets);
    manager.fuzz();
}

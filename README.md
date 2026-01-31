# KFuzzUring

KFuzzUring is a fuzzer for [KFuzzTest](https://lore.kernel.org/all/20260112192827.25989-1-ethan.w.s.graham@gmail.com/#t)
targets written in Rust.

## Why?

syzkaller is highly effective at fuzzing system calls, but contains complex
features that are overkill for blob-based fuzzing:

- Orchestration of multiple VMs
- Generating long sequences of system calls
- Structural mutation

KFuzzTest targets are invoked by simply calling `write()` on the debugfs file
associated to a fuzz target, e.g.,
`/sys/kernel/debug/kfuzztest/test-name/input_simple`. In removing most of the
complexity, we hope to increase the performance on such targets.

Empirically, with a recent experiment on an injected bug in
`pkcs7_parse_message()`, KFuzzUring was able to trigger a KASAN report in under
a second, where syzkaller would commonly take around 30 seconds.

On my laptop, I am seeing around 130k inputs/sec, which is blazingly fast for
a kernel fuzzer as far as I can tell. 

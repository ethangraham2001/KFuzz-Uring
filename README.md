# KUnit Fuzzing (original description below)

I'm playing around with implementing fuzzing harnesses in KUnit - something that
was proposed back in the thread following the [original RFC](https://lore.kernel.org/all/CABVgOS=7wrxywmgn8YRW4o_sUN=wOxa4k7NbTObAxA5okmr+CQ@mail.gmail.com/).

It seems to work decently well - albeit slower than the macro-based fuzz tests
probably because KUnit executes the tests (fuzzing harnesses) in separate
threads.

## Experiment, and reproducing it

I like to build the binary on my local machine and then copy it into a QEMU VM.
It seems like the best way to do this with Cargo (avoiding libc compatibilities
and the like) is like so:

```sh
$ cargo build --release --target x86_64-unknown-linux-musl
```

Then, simply copy the binary over to the VM, which should look something like
this if you use a VM configured similarly to how it is suggested in the
[syzkaller instructions](https://github.com/google/syzkaller/blob/master/docs/linux/setup_ubuntu-host_qemu-vm_x86-64-kernel.md).

```sh
$ scp -i $IMAGE/bullseye.id_rsa -P 10021 -o "StrictHostKeyChecking no" ./target/x86_64-unknown-linux-musl/release/KFuzz-Uring root@localhost:~/kfuzzuring
```

Then just run the binary which is currently (regretfully) hardcoded to fuzz
`pkcs7_parse_message`. It should find the bug pretty fast.

```sh
$ ./kfuzzuring
rng seed = 16670631328944092080
targets:
	fuzz_pkcs7_parse_message
[  870.089498] ==================================================================
[  870.090424] BUG: KASAN: slab-out-of-bounds in asn1_ber_decoder+0x148c/0x1560
[  870.091363] Read of size 1 at addr ffff888105004dc1 by task kunit_try_catch/36376
[  870.092350]
[  870.092585] CPU: 0 UID: 0 PID: 36376 Comm: kunit_try_catch Tainted: G      D          N  7.0.0-rc5-g5bd5ed742368 #88 PREEMPT(full)
[  870.092593] Tainted: [D]=DIE, [N]=TEST
[  870.092594] Hardware name: QEMU Standard PC (i440FX + PIIX, 1996), BIOS Arch Linux 1.17.0-2-2 04/01/2014
[  870.092598] Call Trace:
[  870.092600]  <TASK>
[  870.092602]  dump_stack_lvl+0x70/0xa0
[  870.092611]  ? asn1_ber_decoder+0x148c/0x1560
[  870.092614]  print_report+0x170/0x4f3
[  870.092620]  ? __pfx__raw_spin_lock_irqsave+0x10/0x10
# ...
```

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

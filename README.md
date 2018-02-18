[![crates.io](https://img.shields.io/crates/v/cortex-m-rtfm.svg)](https://crates.io/crates/cortex-m-rtfm)
[![crates.io](https://img.shields.io/crates/d/cortex-m-rtfm.svg)](https://crates.io/crates/cortex-m-rtfm)

# `cortex-m-rtfm`

> Real Time For the Masses (RTFM) framework for ARM Cortex-M microcontrollers

# [Documentation](https://docs.rs/cortex-m-rtfm)

# License

# Klee based analysis.

## Complilation

Use a Rust toolchain with a llwm4 backend.


> xargo build --example empty --features klee_mode --release --target x86_64-unknown-linux-gnu

The `--features klee_mode` implies the following:

- the set of tasks is generated in ./klee/tasks.txt
- the example is built without HW dependencies 
  claim does NOT affect basepri register

The `--target x86_64-unknown-linux-gnu` implies the following:

``` text
rustflags = [
  "--emit=llvm-bc,llvm-ir",
  "-C", "linker=true"
]
```

So both `.bc` and `.ll` are generated for the application at hande (`empty.rs`). (Only `.bc` is indeed needed/used for KLEE analysis, but the `.ll` is neat as being human readable for code inspection).

The files will contain all symbols besides the *weakly* defined default handlers (which should not be analysed by KLEE in any case).

TODO: there is still a warning on module level assembly.

The `--relesase` flag implies:

``` text
[profile.release]
lto = true
debug = true
panic = "abort"
```

`lto = true` ensures pre-llvm linknig of dependencies for the generated `.bc` and `.ll` files.

`panic = "abort"` is required to override the default panic (unwrap) behavior of the x86 (host) target.



## KLEE Analysis

A docker daemon should be started/enabled.

> systemctl start docker.service # if not already started/enabled

In the current folder:

> docker run --rm --user $(id -u):$(id -g) -v $PWD/target/x86_64-unknown-linux-gnu/release/examples:/mnt -w /mnt -it afoht/llvm-klee-4 /bin/bash

This starts a shell in the docker `llvm-klee-4`, with a shared mount to the examples directory, where your `empty-xxxx.bc` is located.

From there you can run various commands like:

> klee empty-xxxxx.bc

See KLEE for detailed information.

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)

- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

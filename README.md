# soil-rust

Rust bootstrap compiler and interpreter for [Soil](https://github.com/ajmwagar/soil).

This repository exists only to bootstrap the Soil-written compiler. It is not the
runtime, not Loam, and not the final compiler.

It contains:

- the S-expression reader
- the interpreter
- the numeric core
- the Rust Wasm backend
- `soil-c`, which emits Wasm from numeric-core Soil
- `soil-run`, which runs Soil files on the bootstrap interpreter
- the language prelude

## Build

```sh
cargo build -p soil-lang --bin soil-c --bin soil-run
```

## Run the Soil compiler

```sh
# from a checkout that has ajmwagar/soil next to ajmwagar/soil-rust
cargo run -q -p soil-lang --bin soil-run -- \
  --prelude crates/soil-lang/stdlib/prelude.soil \
  --call main ../soil/compiler/core.soil /tmp/compile-runtime.soil
```

The saved bootstrap seed from the monorepo transition:

```text
sha256 3f58bee9e33aecf79991a2e2bdfa1f81926d625aec9825d56427dacb857f53b5
source 96333190cc5e6105cf382406e0b3b79f63879c37
```

The hash is recorded in `bootstrap/seed.txt`; binaries are not committed.

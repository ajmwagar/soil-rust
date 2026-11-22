# soil-wasm

Temporary WASI p2 runner for the Soil bootstrap interpreter.

Wasmtime links WASI. This runner maps dynamic Soil `host-call` names to
`wasi:cli` and `wasi:io` imports. The compiler will eventually emit those
imports directly.

```sh
cargo build -p soil-wasm --target wasm32-wasip2 --release

wasmtime run --dir . target/wasm32-wasip2/release/soil-wasm.wasm \
  --prelude core/prelude.soil \
  --call main \
  std/bindings/wasi-io.generated.soil \
  std/bindings/wasi-cli.generated.soil \
  core/repl.soil \
  tools/repl/main.soil
```

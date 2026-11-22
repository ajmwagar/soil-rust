// SPDX-License-Identifier: MIT OR Apache-2.0
use std::path::PathBuf;

use anyhow::{Result, bail};
use soil_lang::sexp::Value;

fn main() -> Result<()> {
    let args = std::env::args().skip(1);
    let mut lisp = false;
    let mut selfhost = false;
    let mut soil_root = PathBuf::from(".");
    let mut prelude = None;
    let mut input = None;
    let mut output = None;
    let mut args = args;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--lisp" => lisp = true,
            "--selfhost" => selfhost = true,
            "--soil-root" => {
                soil_root = PathBuf::from(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--soil-root needs a path"))?,
                );
            }
            "--prelude" => {
                prelude = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--prelude needs a path"))?,
                ));
            }
            "-h" | "--help" => {
                println!("usage: soil-c [--lisp|--selfhost] [--soil-root DIR] [--prelude PATH] INPUT.soil [OUTPUT.wasm]");
                return Ok(());
            }
            other => {
                if input.is_none() {
                    input = Some(PathBuf::from(other));
                } else if output.is_none() {
                    output = Some(PathBuf::from(other));
                } else {
                    bail!("usage: soil-c [--lisp] INPUT.soil [OUTPUT.wasm]");
                }
            }
        }
    }
    let Some(input) = input else {
        bail!("usage: soil-c [--lisp] INPUT.soil [OUTPUT.wasm]");
    };
    let output = output.unwrap_or_else(|| PathBuf::from(format!("{}.wasm", input.display())));
    let source = std::fs::read_to_string(&input)?;
    let wasm = if selfhost {
        compile_selfhost(&source, &soil_root, prelude.as_deref())?
    } else if lisp {
        soil_lang::lisp::compile_source(&source)?.wasm
    } else {
        let program = soil_lang::parse(&source)?;
        soil_lang::compile(&program)?
    };
    std::fs::write(&output, wasm)?;
    println!("{}", output.display());
    Ok(())
}

fn compile_selfhost(
    source: &str,
    soil_root: &std::path::Path,
    prelude: Option<&std::path::Path>,
) -> Result<Vec<u8>> {
    let prelude = prelude
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| soil_root.join("core/prelude.soil"));
    let compiler = soil_root.join("compiler/core.soil");
    let entry = soil_root.join("compiler/main.soil");

    let mut runtime = soil_lang::Runtime::new(Box::new(soil_lang::FakeHost::new()));
    runtime.load_file(&prelude)?;
    runtime.load_file(&compiler)?;
    runtime.load_file(&entry)?;
    runtime.set_args(vec![source.to_string()]);
    let value = runtime.call("main", &[])?;
    value_bytes(&value)
}

fn value_bytes(value: &Value) -> Result<Vec<u8>> {
    let items = value
        .as_list()
        .ok_or_else(|| anyhow::anyhow!("selfhost compiler did not return a byte list"))?;
    items
        .iter()
        .map(|item| match item {
            Value::Int(value) if (0..=255).contains(value) => Ok(*value as u8),
            _ => bail!("selfhost compiler returned a non-byte value"),
        })
        .collect()
}

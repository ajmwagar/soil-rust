// SPDX-License-Identifier: MIT OR Apache-2.0
//! Run Soil files on the bootstrap interpreter.
//!
//! ```text
//! soil-run --prelude stdlib/prelude.soil --call main compiler/core.soil program.soil
//! ```

use std::path::PathBuf;

use anyhow::{Result, bail};
use soil_lang::{FakeHost, Runtime};

fn main() -> Result<()> {
    let mut prelude = std::env::var_os("SOIL_PRELUDE").map(PathBuf::from);
    let mut call = Some("main".to_string());
    let mut files = Vec::new();
    let mut program_args = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--prelude" => {
                prelude = Some(PathBuf::from(
                    args.next().ok_or_else(|| anyhow::anyhow!("--prelude needs a path"))?,
                ));
            }
            "--call" => {
                call = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--call needs a name"))?,
                );
            }
            "--no-call" => call = None,
            "--arg" => {
                program_args.push(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--arg needs a value"))?,
                );
            }
            "--" => {
                program_args.extend(args.by_ref());
                break;
            }
            "-h" | "--help" => {
                println!(
                    "usage: soil-run [--prelude PATH] [--call NAME] [--arg VALUE]... FILE... [-- ARG...]"
                );
                return Ok(());
            }
            other if other.starts_with('-') => bail!("unknown option {other}"),
            file => files.push(PathBuf::from(file)),
        }
    }
    let prelude = prelude.unwrap_or_else(|| PathBuf::from("stdlib/prelude.soil"));
    let mut runtime = Runtime::new(Box::new(FakeHost::new()));
    runtime.set_args(program_args);
    runtime.load_file(&prelude)?;
    for file in &files {
        runtime.load_file(file)?;
    }
    if let Some(name) = call {
        println!("{}", runtime.call(&name, &[])?);
    }
    Ok(())
}

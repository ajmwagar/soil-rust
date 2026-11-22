// SPDX-License-Identifier: MIT OR Apache-2.0
//! Soil bootstrap interpreter as a `wasm32-wasip2` command.
//!
//! Wasmtime links the `wasi:cli` and `wasi:io` imports. This binary maps the
//! dynamic Soil `host-call` names onto those imports. It is the temporary
//! runner until the Soil compiler emits direct WIT imports.

use std::path::PathBuf;

use anyhow::{Result, bail};
use soil_lang::sexp::Value;
use soil_lang::{Host, Runtime};

use wasip2::io::poll::Pollable;
use wasip2::io::streams::{InputStream, OutputStream, StreamError};

struct WasmHost {
    stdin: Option<InputStream>,
    stdout: Option<OutputStream>,
    stderr: Option<OutputStream>,
    pollables: Vec<Pollable>,
}

impl WasmHost {
    fn new() -> Self {
        Self {
            stdin: None,
            stdout: None,
            stderr: None,
            pollables: Vec::new(),
        }
    }

    fn stdin(&mut self) -> &InputStream {
        self.stdin.get_or_insert_with(wasip2::cli::stdin::get_stdin)
    }

    fn stdout(&mut self) -> &OutputStream {
        self.stdout
            .get_or_insert_with(wasip2::cli::stdout::get_stdout)
    }

    fn stderr(&mut self) -> &OutputStream {
        self.stderr
            .get_or_insert_with(wasip2::cli::stderr::get_stderr)
    }
}

fn ok(value: Value) -> Value {
    Value::List(vec![Value::Symbol("ok".into()), value])
}

fn err(message: impl ToString) -> Value {
    Value::List(vec![
        Value::Symbol("err".into()),
        Value::String(message.to_string()),
    ])
}

fn stream_error(error: StreamError) -> Value {
    err(format!("{error:?}"))
}

fn bytes(value: &Value) -> Result<Vec<u8>, String> {
    let Some(items) = value.as_list() else {
        return Err("bytes: expected a list".into());
    };
    items
        .iter()
        .map(|item| match item {
            Value::Int(byte) if (0..=255).contains(byte) => Ok(*byte as u8),
            _ => Err("bytes: expected integers in 0..=255".into()),
        })
        .collect()
}

fn byte_list(bytes: impl IntoIterator<Item = u8>) -> Value {
    Value::List(bytes.into_iter().map(|byte| Value::Int(i64::from(byte))).collect())
}

impl Host for WasmHost {
    fn granted(&self, interface: &str, function: &str) -> bool {
        matches!(
            (interface, function),
            ("wasi:cli/stdin", "get-stdin")
                | ("wasi:cli/stdout", "get-stdout")
                | ("wasi:cli/stderr", "get-stderr")
                | ("wasi:io/streams", "input-stream.blocking-read")
                | ("wasi:io/streams", "output-stream.blocking-write-and-flush")
                | ("wasi:io/streams", "output-stream.blocking-flush")
                | ("wasi:filesystem", "read")
                | ("wasi:filesystem", "write")
                | ("wasi:filesystem", "list")
                | ("wasi:clocks/monotonic-clock", "now")
                | ("wasi:clocks/monotonic-clock", "resolution")
                | ("wasi:clocks/wall-clock", "now")
                | ("wasi:clocks/wall-clock", "resolution")
                | ("wasi:clocks/monotonic-clock", "subscribe-duration")
                | ("wasi:io/poll", "poll")
        )
    }

    fn call(
        &mut self,
        interface: &str,
        function: &str,
        args: &[Value],
    ) -> std::result::Result<Value, String> {
        match (interface, function) {
            ("wasi:cli/stdin", "get-stdin") => {
                self.stdin();
                Ok(Value::Int(1))
            }
            ("wasi:cli/stdout", "get-stdout") => {
                self.stdout();
                Ok(Value::Int(2))
            }
            ("wasi:cli/stderr", "get-stderr") => {
                self.stderr();
                Ok(Value::Int(3))
            }
            ("wasi:io/streams", "input-stream.blocking-read") => {
                let handle = args.first().and_then(Value::as_f64).unwrap_or(0.0);
                let len = args.get(1).and_then(Value::as_f64).unwrap_or(0.0) as u64;
                if handle != 1.0 {
                    return Ok(err("unknown input stream"));
                }
                Ok(match self.stdin().blocking_read(len) {
                    Ok(data) => ok(byte_list(data)),
                    Err(error) => stream_error(error),
                })
            }
            ("wasi:io/streams", "output-stream.blocking-write-and-flush") => {
                let handle = args.first().and_then(Value::as_f64).unwrap_or(0.0);
                let Some(contents) = args.get(1) else {
                    return Ok(err("write needs bytes"));
                };
                let data = bytes(contents)?;
                let result = match handle {
                    2.0 => self.stdout().blocking_write_and_flush(&data),
                    3.0 => self.stderr().blocking_write_and_flush(&data),
                    _ => return Ok(err("unknown output stream")),
                };
                Ok(match result {
                    Ok(()) => ok(Value::Bool(true)),
                    Err(error) => stream_error(error),
                })
            }
            ("wasi:clocks/monotonic-clock", "now") => {
                Ok(Value::Int(wasip2::clocks::monotonic_clock::now() as i64))
            }
            ("wasi:clocks/monotonic-clock", "resolution") => Ok(Value::Int(
                wasip2::clocks::monotonic_clock::resolution() as i64,
            )),
            ("wasi:clocks/wall-clock", "now") => {
                let clock = wasip2::clocks::wall_clock::now();
                Ok(Value::List(vec![
                    Value::List(vec![
                        Value::Keyword("seconds".into()),
                        Value::Int(clock.seconds as i64),
                    ]),
                    Value::List(vec![
                        Value::Keyword("nanoseconds".into()),
                        Value::Int(i64::from(clock.nanoseconds)),
                    ]),
                ]))
            }
            ("wasi:clocks/wall-clock", "resolution") => {
                let clock = wasip2::clocks::wall_clock::resolution();
                Ok(Value::List(vec![
                    Value::List(vec![
                        Value::Keyword("seconds".into()),
                        Value::Int(clock.seconds as i64),
                    ]),
                    Value::List(vec![
                        Value::Keyword("nanoseconds".into()),
                        Value::Int(i64::from(clock.nanoseconds)),
                    ]),
                ]))
            }
            ("wasi:clocks/monotonic-clock", "subscribe-duration") => {
                let duration = args.first().and_then(Value::as_f64).unwrap_or(0.0) as u64;
                let pollable = wasip2::clocks::monotonic_clock::subscribe_duration(duration);
                self.pollables.push(pollable);
                Ok(Value::Int(self.pollables.len() as i64))
            }
            ("wasi:io/poll", "poll") => {
                let Some(handles) = args.first().and_then(Value::as_list) else {
                    return Ok(err("poll needs a list of pollables"));
                };
                let mut refs = Vec::with_capacity(handles.len());
                for handle in handles {
                    let Some(handle) = handle.as_f64() else {
                        return Ok(err("poll handles must be integers"));
                    };
                    let Some(pollable) = self.pollables.get(handle as usize - 1) else {
                        return Ok(err("unknown pollable handle"));
                    };
                    refs.push(pollable);
                }
                Ok(Value::List(
                    wasip2::io::poll::poll(&refs)
                        .into_iter()
                        .map(|index| Value::Int(i64::from(index)))
                        .collect(),
                ))
            }
            ("wasi:filesystem", "list") => {
                let path = args.first().and_then(Value::as_str).unwrap_or_default();
                let dir = std::path::Path::new(path);
                let entries = match std::fs::read_dir(dir) {
                    Ok(entries) => entries,
                    Err(error) => return Ok(err(format!("list {path}: {error}"))),
                };
                let mut values = Vec::new();
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().into_owned();
                    let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
                    values.push(Value::List(vec![
                        Value::List(vec![Value::Keyword("name".into()), Value::String(name)]),
                        Value::List(vec![
                            Value::Keyword("path".into()),
                            Value::String(path.to_string_lossy().into_owned()),
                        ]),
                        Value::List(vec![
                            Value::Keyword("dir".into()),
                            Value::Bool(is_dir),
                        ]),
                    ]));
                }
                Ok(ok(Value::List(values)))
            }
            ("wasi:filesystem", "read") => {
                let path = args.first().and_then(Value::as_str).unwrap_or_default();
                Ok(match std::fs::read(path) {
                    Ok(data) => ok(Value::String(String::from_utf8_lossy(&data).into_owned())),
                    Err(error) => err(format!("read {path}: {error}")),
                })
            }
            ("wasi:filesystem", "write") => {
                let path = args.first().and_then(Value::as_str).unwrap_or_default();
                let Some(contents) = args.get(1) else {
                    return Ok(err("write needs bytes"));
                };
                let data = bytes(contents)?;
                Ok(match std::fs::write(path, data) {
                    Ok(()) => ok(Value::Bool(true)),
                    Err(error) => err(format!("write {path}: {error}")),
                })
            }
            ("wasi:io/streams", "output-stream.blocking-flush") => {
                let handle = args.first().and_then(Value::as_f64).unwrap_or(0.0);
                let result = match handle {
                    2.0 => self.stdout().blocking_flush(),
                    3.0 => self.stderr().blocking_flush(),
                    _ => return Ok(err("unknown output stream")),
                };
                Ok(match result {
                    Ok(()) => ok(Value::Bool(true)),
                    Err(error) => stream_error(error),
                })
            }
            _ => Err(format!("no runner binding for {interface}/{function}")),
        }
    }
}

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
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--prelude needs a path"))?,
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
                    "usage: soil-wasm [--prelude PATH] [--call NAME] [--arg VALUE]... FILE... [-- ARG...]"
                );
                return Ok(());
            }
            other if other.starts_with('-') => bail!("unknown option {other}"),
            file => files.push(PathBuf::from(file)),
        }
    }
    let prelude = prelude.unwrap_or_else(|| PathBuf::from("core/prelude.soil"));
    let mut runtime = Runtime::new(Box::new(WasmHost::new()));
    runtime.set_args(program_args);
    runtime.load_file(&prelude)?;
    for file in &files {
        runtime.load_file(file)?;
    }
    if let Some(name) = call {
        let value = runtime.call(&name, &[])?;
        if !matches!(value, Value::Nil) {
            println!("{value}");
        }
    }
    Ok(())
}

// SPDX-License-Identifier: MIT OR Apache-2.0
//! Rust bootstrap implementation of the Soil reader, interpreter, numeric core,
//! and Wasm compiler.
//!
//! This crate exists to bootstrap the Soil-written compiler in
//! `ajmwagar/soil`. It intentionally has no host or capability dependencies.

pub mod core;
pub mod eval;
pub mod lisp;
pub mod sexp;
pub mod wasm;

pub use core::{Expr, Function, Program, parse};
pub use eval::{
    FakeHost, Host, HostChain, Runtime, json_to_value as from_json, value_to_json as to_json,
};
pub use lisp::{Compiled as CompiledLisp, compile_source as compile_lisp};
pub use sexp::{Comment, Document, Form, Span, parse_document, print_document};
pub use wasm::compile;

//! Exports that must be *rejected* under `panic = "unwind"` (and accepted
//! under `panic = "abort"`): the per-argument unwind-safety assertion in
//! the generated export shim (`__rt::ensure_arg_unwind_safe`).
//!
//! This crate is excluded from the workspace; the
//! `test-wasm-bindgen-unwind-compile-fail` justfile recipe checks both
//! polarities. It cannot be a trybuild test because the failure only
//! exists under `-Cpanic=unwind` with a rebuilt std, which trybuild's
//! host compiler doesn't provide.

use std::cell::Cell;
use std::marker::PhantomData;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct NotUnwindSafe {
    _marker: PhantomData<&'static mut ()>,
}

#[wasm_bindgen]
pub struct NotRefUnwindSafe {
    _cell: Cell<u32>,
}

// Owned argument of a non-`UnwindSafe` type.
#[wasm_bindgen]
pub fn take_owned(_x: NotUnwindSafe) {}

// Borrow of a non-`RefUnwindSafe` pointee.
#[wasm_bindgen]
pub fn take_ref(_x: &NotRefUnwindSafe) {}

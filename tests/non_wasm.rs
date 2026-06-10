extern crate wasm_bindgen;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct A {
    x: u32,
}

#[wasm_bindgen]
impl A {
    pub fn new() -> A {
        A { x: 3 }
    }

    pub fn foo(&self) {
        assert_eq!(self.x, 3);
    }
}

#[wasm_bindgen]
pub fn foo(x: bool) {
    A::new().foo();

    if x {
        bar("test");
        baz(JsValue::from(3));
    }
}

#[wasm_bindgen]
extern "C" {
    fn some_import();
    #[wasm_bindgen(thread_local_v2)]
    static A: JsValue;
}

#[wasm_bindgen]
pub fn bar(_: &str) -> JsValue {
    some_import();
    A.with(JsValue::clone)
}

#[wasm_bindgen]
pub fn baz(_: JsValue) {}

#[test]
fn test_foo() {
    foo(false);
    A::new().foo();
}

// Canary for the trait-based export dispatch: the decode → project shape
// the macro emits must work for an alias-hidden `&str` (type-level only:
// `WasmSlice` carries 32-bit pointers, so a slice abi can't round-trip on
// a 64-bit host) and round-trip owned arguments with host-safe ABI values.
// (`&mut [T]` guards aren't exercised here because their drop runs a
// wasm-only intrinsic; the wasm test suite covers them.)
#[test]
fn arg_abi_canary() {
    use wasm_bindgen::convert::{ArgAbi, ArgGuard, CallScoped, IntoWasmAbi};

    type Str<'a> = &'a str;

    fn str_shape(abi: <Str<'_> as ArgAbi<CallScoped>>::Abi) -> usize {
        let mut g = unsafe { <Str<'_> as ArgAbi<CallScoped>>::arg_from_abi(abi) };
        let s = ArgGuard::project(&mut g);
        s.len()
    }
    let _ = str_shape;

    // Runtime decode-project round-trip, mirroring the generated shape.
    let mut a0 = unsafe { <u32 as ArgAbi<CallScoped>>::arg_from_abi(2u32.into_abi()) };
    let mut a1 = unsafe { <f64 as ArgAbi<CallScoped>>::arg_from_abi(1.5f64.into_abi()) };
    let a0 = ArgGuard::project(&mut a0);
    let a1 = ArgGuard::project(&mut a1);
    assert_eq!(a0 as f64 + a1, 3.5);
}

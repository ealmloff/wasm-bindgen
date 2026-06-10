use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    type A;

    fn f() -> &'static u32;

    #[wasm_bindgen(method)]
    fn f1();
    #[wasm_bindgen(method)]
    fn f2(x: u32);
    #[wasm_bindgen(method)]
    fn f3(x: &&u32);
    #[wasm_bindgen(method)]
    fn f4(x: &foo::Bar);
    #[wasm_bindgen(method)]
    fn f4(x: &::Bar);
    #[wasm_bindgen(method)]
    fn f4(x: &Bar<T>);
    #[wasm_bindgen(method)]
    fn f4(x: &dyn Fn(T));

    #[wasm_bindgen(constructor)]
    fn f();
    #[wasm_bindgen(constructor)]
    fn f() -> ::Bar;
    #[wasm_bindgen(constructor)]
    fn f() -> &Bar;

    // Invalid `catch` returns are now rejected by the type system
    // (`CatchFromWasmAbi`) or by codegen, not the parser; they live in
    // invalid-catch*.rs because the parser-phase errors in this file
    // suppress later-phase errors.
}

fn main() {}

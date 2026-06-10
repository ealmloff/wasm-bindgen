// A `catch` import returning a reference hits the import
// return-reference ban in codegen — a macro-expansion-time error, which
// would suppress the typeck-phase errors asserted in invalid-catch.rs,
// so it needs its own compilation unit.

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch)]
    fn f() -> &u32;
}

fn main() {}

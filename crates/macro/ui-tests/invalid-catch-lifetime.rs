// A `catch` import returning `Result<'a>` errors during name resolution
// (undeclared lifetime), a phase earlier than the typeck errors asserted
// in invalid-catch.rs, so it needs its own compilation unit.

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch)]
    fn f() -> Result<'a>;
}

fn main() {}

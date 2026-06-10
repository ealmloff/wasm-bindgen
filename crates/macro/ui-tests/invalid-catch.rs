// `catch` imports whose return type is not a supported `Result` are
// rejected by the type system (`CatchFromWasmAbi`), not by the macro
// parsing a literal `Result<...>` — so type aliases of `Result` work.
// These cases error during typeck; same-phase only, since earlier-phase
// errors would suppress them (see invalid-imports.rs for the macro-time
// case and invalid-catch-lifetime.rs for the resolution-time case).

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch)]
    fn f() -> u32;
    #[wasm_bindgen(catch)]
    fn g() -> Result;
}

fn main() {}

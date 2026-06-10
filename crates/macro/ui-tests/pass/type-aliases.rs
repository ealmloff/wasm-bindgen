//! Type aliases must behave exactly like the types they alias.
//!
//! This is a trybuild *pass* test: every signature here must compile.
//! `ArgAbi<Scope>` and `CatchFromWasmAbi` dispatch through trait
//! resolution rather than macro-time syntax matching, so aliases behave
//! identically to their expansions.

use wasm_bindgen::prelude::*;

type StrRef<'a> = &'a str;
type Bytes<'a> = &'a [u8];
type BytesMut<'a> = &'a mut [u8];
type JsRef<'a> = &'a JsValue;
type Text = String;
type OptText = Option<String>;
type Fallible = Result<JsValue, JsValue>;

// === Aliases hiding a reference in export argument position ===

#[wasm_bindgen]
pub fn alias_str_len(s: StrRef) -> usize {
    s.len()
}

#[wasm_bindgen]
pub fn alias_bytes_sum(b: Bytes) -> u32 {
    b.iter().map(|&b| u32::from(b)).sum()
}

#[wasm_bindgen]
pub fn alias_bytes_fill(b: BytesMut, v: u8) {
    b.fill(v);
}

#[wasm_bindgen]
pub fn alias_js_is_null(v: JsRef) -> bool {
    v.is_null()
}

// Async exports use the `Anchored` `ArgAbi` scope, even when the
// reference is hidden behind an alias.
#[wasm_bindgen]
pub async fn alias_async_str_len(s: StrRef<'_>) -> usize {
    s.len()
}

// References to exported classes.
#[wasm_bindgen]
pub struct Counter {
    n: u32,
}

type CounterRef<'a> = &'a Counter;
type CounterMut<'a> = &'a mut Counter;

#[wasm_bindgen]
pub fn alias_counter_get(c: CounterRef) -> u32 {
    c.n
}

#[wasm_bindgen]
pub fn alias_counter_bump(c: CounterMut) {
    c.n += 1;
}

// === Aliases hiding a `Result` in `catch` import return position ===

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = JSON, js_name = parse)]
    fn parse_fallible(s: &str) -> Fallible;
}

// === Green guards: these already work and must keep working ===

// Owned aliases resolve through the type system today.
#[wasm_bindgen]
pub fn alias_owned_roundtrip(t: Text) -> Text {
    t
}

// `Option<T>` export arguments also resolve through `ArgAbi`, so an
// alias works.
#[wasm_bindgen]
pub fn alias_option_roundtrip(t: OptText) -> OptText {
    t
}

// Import *arguments* already go through a single trait (`IntoWasmAbi` on
// the written type), so an alias-hidden reference works there.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_str(s: StrRef);
}

fn main() {}

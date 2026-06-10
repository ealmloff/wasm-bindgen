//! Type aliases must behave exactly like the types they alias - both must
//! compile and must produce identical runtime behavior.
//!
//! These cases cover the type-system dispatch paths that replaced the
//! macro's old syntactic per-shape matching, including `catch` import
//! returns that are aliases of `Result<...>`.

use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

type StrRef<'a> = &'a str;
type Bytes<'a> = &'a [u8];
type BytesMut<'a> = &'a mut [u8];
type JsRef<'a> = &'a JsValue;
type Text = String;
type OptText = Option<String>;
type Fallible = Result<JsValue, JsValue>;
type UnitFallible = Result<(), JsValue>;

#[wasm_bindgen(module = "tests/wasm/type_aliases.js")]
extern "C" {
    fn js_sync_aliases_work();
    #[wasm_bindgen(catch)]
    async fn js_async_aliases_work() -> Result<(), JsValue>;

    #[wasm_bindgen(catch)]
    async fn js_async_unit_alias_ok() -> UnitFallible;

    #[wasm_bindgen(catch)]
    async fn js_async_unit_alias_err() -> UnitFallible;

    // Import argument position already resolves through the type system
    // (`IntoWasmAbi` on the written type), so the alias works today.
    fn js_take_str(s: StrRef) -> bool;

    // Import return position with `catch` resolves through
    // `CatchFromWasmAbi`, so aliases behave like literal `Result<...>`.
    #[wasm_bindgen(catch)]
    fn js_throw() -> Fallible;
}

// Every exported pair: the plain version works today, the `_alias`
// version must behave identically once green.

#[wasm_bindgen]
pub fn ta_str_len(s: &str) -> usize {
    s.len()
}

#[wasm_bindgen]
pub fn ta_str_len_alias(s: StrRef) -> usize {
    s.len()
}

#[wasm_bindgen]
pub fn ta_bytes_sum(b: &[u8]) -> u32 {
    b.iter().map(|&b| u32::from(b)).sum()
}

#[wasm_bindgen]
pub fn ta_bytes_sum_alias(b: Bytes) -> u32 {
    b.iter().map(|&b| u32::from(b)).sum()
}

#[wasm_bindgen]
pub fn ta_bytes_fill(b: &mut [u8], v: u8) {
    b.fill(v);
}

#[wasm_bindgen]
pub fn ta_bytes_fill_alias(b: BytesMut, v: u8) {
    b.fill(v);
}

#[wasm_bindgen]
pub fn ta_js_is_null(v: &JsValue) -> bool {
    v.is_null()
}

#[wasm_bindgen]
pub fn ta_js_is_null_alias(v: JsRef) -> bool {
    v.is_null()
}

#[wasm_bindgen]
pub async fn ta_async_str_len(s: &str) -> usize {
    s.len()
}

#[wasm_bindgen]
pub async fn ta_async_str_len_alias(s: StrRef<'_>) -> usize {
    s.len()
}

#[wasm_bindgen]
pub struct TaCounter {
    n: u32,
}

#[wasm_bindgen]
impl TaCounter {
    #[wasm_bindgen(constructor)]
    pub fn new(n: u32) -> TaCounter {
        TaCounter { n }
    }
}

type TaCounterRef<'a> = &'a TaCounter;
type TaCounterMut<'a> = &'a mut TaCounter;

#[wasm_bindgen]
pub fn ta_counter_get(c: &TaCounter) -> u32 {
    c.n
}

#[wasm_bindgen]
pub fn ta_counter_get_alias(c: TaCounterRef) -> u32 {
    c.n
}

#[wasm_bindgen]
pub fn ta_counter_bump(c: &mut TaCounter) {
    c.n += 1;
}

#[wasm_bindgen]
pub fn ta_counter_bump_alias(c: TaCounterMut) {
    c.n += 1;
}

// Green guards: owned and Option aliases already resolve through the type
// system and must keep working.

#[wasm_bindgen]
pub fn ta_owned_roundtrip(t: Text) -> Text {
    t
}

#[wasm_bindgen]
pub fn ta_option_roundtrip(t: OptText) -> OptText {
    t
}

#[wasm_bindgen_test]
fn sync_aliases() {
    js_sync_aliases_work();
    assert!(js_take_str("via alias import"));
    assert!(js_throw().is_err());
}

#[wasm_bindgen_test]
async fn async_aliases() {
    js_async_aliases_work().await.unwrap();
    js_async_unit_alias_ok().await.unwrap();
    assert!(js_async_unit_alias_err().await.is_err());
}

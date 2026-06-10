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
type MaybeU32 = Option<u32>;
type Fallible = Result<JsValue, JsValue>;
type UnitFallible = Result<(), JsValue>;
type StrFallible = Result<String, JsValue>;
type StringResult<E> = Result<String, E>;

/// A custom error type exercising the `E: From<JsValue>` side of
/// `CatchFromWasmAbi` through an alias.
pub struct TaErr(#[allow(dead_code)] JsValue);

impl From<JsValue> for TaErr {
    fn from(v: JsValue) -> Self {
        TaErr(v)
    }
}

#[wasm_bindgen(module = "tests/wasm/type_aliases.js")]
extern "C" {
    fn js_sync_aliases_work();
    #[wasm_bindgen(catch)]
    async fn js_async_aliases_work() -> Result<(), JsValue>;

    #[wasm_bindgen(catch)]
    async fn js_async_unit_alias_ok() -> UnitFallible;

    #[wasm_bindgen(catch)]
    async fn js_async_unit_alias_err() -> UnitFallible;

    // Async exports with aliased reference arguments (`Anchored` guards)
    // are exercised from the JS side.
    #[wasm_bindgen(catch)]
    async fn js_async_arg_aliases_work() -> Result<(), JsValue>;

    // Import argument position already resolves through the type system
    // (`IntoWasmAbi` on the written type), so the alias works today.
    fn js_take_str(s: StrRef) -> bool;

    // Import return position with `catch` resolves through
    // `CatchFromWasmAbi`, so aliases behave like literal `Result<...>`.
    #[wasm_bindgen(catch)]
    fn js_throw() -> Fallible;

    // A *parameterized* alias of `Result` is not unwrapped syntactically
    // (its first type argument is not the Ok type) and resolves through
    // `CatchFromWasmAbi`, including a non-`JsValue` error type.
    #[wasm_bindgen(catch, js_name = js_string_ok)]
    fn js_string_ok_generic_alias() -> StringResult<TaErr>;
    #[wasm_bindgen(catch, js_name = js_string_throw)]
    fn js_string_throw_generic_alias() -> StringResult<TaErr>;

    // Async `catch` import with an aliased non-unit `Result`.
    #[wasm_bindgen(catch)]
    async fn js_async_string_ok() -> StrFallible;
    #[wasm_bindgen(catch)]
    async fn js_async_string_err() -> StrFallible;
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

// An alias of an `Option` of a sentinel-encoded primitive (`u32` rides the
// JS number ABI as `f64` when optional).
#[wasm_bindgen]
pub fn ta_opt_u32_roundtrip(v: MaybeU32) -> MaybeU32 {
    v
}

// Async exports with aliased reference arguments: each `Anchored` guard
// shape beyond `&str` (slice copy, owned JS handle, class anchor).

#[wasm_bindgen]
pub async fn ta_async_bytes_sum_alias(b: Bytes<'_>) -> u32 {
    b.iter().map(|&b| u32::from(b)).sum()
}

#[wasm_bindgen]
pub async fn ta_async_js_is_null_alias(v: JsRef<'_>) -> bool {
    v.is_null()
}

#[wasm_bindgen]
pub async fn ta_async_counter_get_alias(c: TaCounterRef<'_>) -> u32 {
    c.n
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
    js_async_arg_aliases_work().await.unwrap();
}

#[wasm_bindgen_test]
fn catch_generic_alias() {
    match js_string_ok_generic_alias() {
        Ok(s) => assert_eq!(s, "ok-string"),
        Err(_) => panic!("expected Ok"),
    }
    assert!(js_string_throw_generic_alias().is_err());
}

#[wasm_bindgen_test]
async fn async_catch_string_alias() {
    assert_eq!(js_async_string_ok().await.unwrap(), "async-string");
    assert!(js_async_string_err().await.is_err());
}

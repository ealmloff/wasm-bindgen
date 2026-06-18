use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

type StrRef<'a> = &'a str;
type SliceRef<'a> = &'a [u8];
type MutSliceRef<'a> = &'a mut [u8];
type JsRef<'a> = &'a JsValue;
type CounterRef<'a> = &'a AliasCounter;
type CounterRefMut<'a> = &'a mut AliasCounter;

#[wasm_bindgen(module = "tests/wasm/arg_aliases.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    fn js_arg_aliases() -> Result<(), JsValue>;
    #[wasm_bindgen(catch)]
    async fn js_async_arg_aliases() -> Result<(), JsValue>;
}

#[wasm_bindgen]
pub fn alias_args(
    s: StrRef<'_>,
    slice: SliceRef<'_>,
    mut_slice: MutSliceRef<'_>,
    value: JsRef<'_>,
    counter: CounterRefMut<'_>,
) -> bool {
    for b in mut_slice.iter_mut() {
        *b += 1;
    }

    counter.n += 2;
    s.len() == 5
        && slice.iter().map(|b| u32::from(*b)).sum::<u32>() == 6
        && value.as_string().is_some()
        && counter.n == 3
}

#[wasm_bindgen]
pub struct AliasCounter {
    n: u32,
}

#[wasm_bindgen]
impl AliasCounter {
    #[wasm_bindgen(constructor)]
    pub fn new(n: u32) -> AliasCounter {
        AliasCounter { n }
    }
}

#[wasm_bindgen]
pub async fn alias_async_args(s: StrRef<'_>, value: JsRef<'_>, counter: CounterRef<'_>) -> bool {
    s.len() == 5 && value.as_string().is_some() && counter.n == 7
}

#[wasm_bindgen_test]
fn arg_aliases() {
    js_arg_aliases().unwrap();
}

#[wasm_bindgen_test]
async fn async_arg_aliases() {
    js_async_arg_aliases().await.unwrap();
}

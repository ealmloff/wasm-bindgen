const wasm = require('wasm-bindgen-test.js');
const assert = require('assert');

// Each aliased export must behave exactly like its plain twin.
exports.js_sync_aliases_work = () => {
    assert.strictEqual(wasm.ta_str_len('hello'), wasm.ta_str_len_alias('hello'));
    assert.strictEqual(wasm.ta_str_len_alias('hello'), 5);

    const bytes = new Uint8Array([1, 2, 3, 4]);
    assert.strictEqual(wasm.ta_bytes_sum(bytes), wasm.ta_bytes_sum_alias(bytes));
    assert.strictEqual(wasm.ta_bytes_sum_alias(bytes), 10);

    const a = new Uint8Array(3);
    const b = new Uint8Array(3);
    wasm.ta_bytes_fill(a, 7);
    wasm.ta_bytes_fill_alias(b, 7);
    assert.deepStrictEqual(Array.from(b), Array.from(a));
    assert.deepStrictEqual(Array.from(b), [7, 7, 7]);

    assert.strictEqual(wasm.ta_js_is_null_alias(null), wasm.ta_js_is_null(null));
    assert.strictEqual(wasm.ta_js_is_null_alias(null), true);
    assert.strictEqual(wasm.ta_js_is_null_alias({}), false);

    const c1 = new wasm.TaCounter(41);
    const c2 = new wasm.TaCounter(41);
    wasm.ta_counter_bump(c1);
    wasm.ta_counter_bump_alias(c2);
    assert.strictEqual(wasm.ta_counter_get_alias(c2), wasm.ta_counter_get(c1));
    assert.strictEqual(wasm.ta_counter_get_alias(c2), 42);

    assert.strictEqual(wasm.ta_owned_roundtrip('owned'), 'owned');
    assert.strictEqual(wasm.ta_option_roundtrip('some'), 'some');
    assert.strictEqual(wasm.ta_option_roundtrip(undefined), undefined);
};

exports.js_async_aliases_work = async () => {
    assert.strictEqual(await wasm.ta_async_str_len('hello'), 5);
    assert.strictEqual(await wasm.ta_async_str_len_alias('hello'), 5);
};

exports.js_async_unit_alias_ok = async () => {};

exports.js_async_unit_alias_err = async () => {
    throw new Error('intentional async');
};

exports.js_async_arg_aliases_work = async () => {
    const bytes = new Uint8Array([1, 2, 3, 4]);
    assert.strictEqual(await wasm.ta_async_bytes_sum_alias(bytes), 10);
    assert.strictEqual(await wasm.ta_async_js_is_null_alias(null), true);
    assert.strictEqual(await wasm.ta_async_js_is_null_alias({}), false);
    const c = new wasm.TaCounter(7);
    assert.strictEqual(await wasm.ta_async_counter_get_alias(c), 7);
    assert.strictEqual(wasm.ta_opt_u32_roundtrip(42), 42);
    assert.strictEqual(wasm.ta_opt_u32_roundtrip(undefined), undefined);
};

exports.js_string_ok = () => 'ok-string';

exports.js_string_throw = () => {
    throw new Error('intentional sync');
};

exports.js_async_string_ok = async () => 'async-string';

exports.js_async_string_err = async () => {
    throw new Error('intentional async string');
};

exports.js_take_str = s => s === 'via alias import';

exports.js_throw = () => {
    throw new Error('intentional');
};

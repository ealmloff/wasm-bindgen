const wasm = require('wasm-bindgen-test.js');
const assert = require('assert');

exports.js_arg_aliases = () => {
    const buffer = new Uint8Array([1, 2, 3]);
    const counter = new wasm.AliasCounter(1);
    assert.strictEqual(
        wasm.alias_args('alias', new Uint8Array([1, 2, 3]), buffer, 'x', counter),
        true
    );
    assert.deepStrictEqual(buffer, new Uint8Array([2, 3, 4]));
};

exports.js_async_arg_aliases = async () => {
    const counter = new wasm.AliasCounter(7);
    assert.strictEqual(await wasm.alias_async_args('alias', 'x', counter), true);
};

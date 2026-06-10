# Vendored serde-wasm-bindgen 0.6.5 (temporary)

This is a vendored copy of [serde-wasm-bindgen] 0.6.5 with a minimal patch
for wasm-bindgen's `ArgAbi` refactor, which removed the unstable
`FromWasmAbi`/`RefFromWasmAbi` traits the published crate uses:

- `src/lib.rs` (`preserve` module): `FromWasmAbi::from_abi` →
  `<JsValue as ArgAbi<CallScoped>>::arg_from_abi(..).unwrap()`.
- `src/ser.rs` (`serialize_newtype_struct`): `JsValue::ref_from_abi` →
  `<&JsValue as ArgAbi<CallScoped>>::arg_from_abi` (the guard derefs to the
  borrowed `JsValue`, which is cloned exactly as before).

It exists only so the in-repo `typescript-tests` and
`examples/raytrace-parallel` workspace members keep building; it is wired up
via `[patch.crates-io]` in the root `Cargo.toml` and excluded from the
workspace.

**Delete this directory, the `[patch.crates-io]` entry, and the
`[workspace] exclude` line once upstream serde-wasm-bindgen releases a
version compatible with the `ArgAbi` API.** Publishing wasm-bindgen to
crates.io is blocked on that upstream release regardless — external users
of serde-wasm-bindgen (and of crates like `tsify` and
`wasm-bindgen-derive` that use the removed traits) get no benefit from
this in-repo patch.

[serde-wasm-bindgen]: https://github.com/RReverser/serde-wasm-bindgen

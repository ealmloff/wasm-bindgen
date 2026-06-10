# Vendored serde-wasm-bindgen 0.6.5 (temporary)

This is a vendored copy of [serde-wasm-bindgen] 0.6.5 with one patch: the
`preserve` feature no longer smuggles raw wasm-bindgen heap indices
through serde via the unstable `wasm_bindgen::convert` traits
(`IntoWasmAbi`/`FromWasmAbi`/`RefFromWasmAbi`, which the `ArgAbi`
refactor removed). It instead stashes the `JsValue` in a crate-private
thread-local stack and smuggles the stash slot — stable API only, no
`unsafe`, works against both old and new wasm-bindgen.

The same patch is prepared for upstream as the
`preserve-without-internal-abi` branch of the local clone at
`~/Desktop/serde-wasm-bindgen`; because it only uses stable API, the
upstream PR builds against *published* wasm-bindgen and does not need to
wait for a wasm-bindgen release.

This copy exists only so the in-repo `typescript-tests` and
`examples/raytrace-parallel` workspace members keep building; it is wired
up via `[patch.crates-io]` in the root `Cargo.toml` and excluded from the
workspace.

**Delete this directory, the `[patch.crates-io]` entry, and the
`[workspace] exclude` line once upstream serde-wasm-bindgen releases with
the patch.** Publishing wasm-bindgen to crates.io is blocked on that
release regardless — external users of serde-wasm-bindgen ≤ 0.6.5 (and
of crates like `tsify` and `wasm-bindgen-derive` that also use the
removed traits) get no benefit from this in-repo copy.

[serde-wasm-bindgen]: https://github.com/RReverser/serde-wasm-bindgen

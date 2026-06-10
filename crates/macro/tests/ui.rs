#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("ui-tests/*.rs");
    // Red-green: desired behavior that does not compile yet. These are
    // expected to PASS; they stay red until ABI trait selection moves from
    // macro-time syntax inspection into the type system. See the header
    // comments in the individual files.
    t.pass("ui-tests/pass/*.rs");
}

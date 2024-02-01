#[test]
fn tests() {
    let t = trybuild::TestCases::new();

    t.compile_fail("tests/ui/double-get-without-scope.rs");
    t.pass("tests/ui/double-get-in-scope.rs");
    t.compile_fail("tests/ui/put-while-holding-ref.rs");
    t.pass("tests/ui/len-while-holding-mut.rs");
    t.compile_fail("tests/ui/handle-with-wrong-perm.rs");
    t.pass("tests/ui/drop-handle-while-holding-ref.rs");
}

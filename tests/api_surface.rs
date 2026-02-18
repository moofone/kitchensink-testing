use kitchensink_testing::prelude::*;

#[test]
fn prelude_compiles_and_exports_core() {
    let _ = bounded_f64(0.0, 1.0);
    let _ = alphanumeric_id(8);
    assert_approx_eq(1.0, 1.0, 0.0);
    assert_commutative(1_i32, 2_i32, |a, b| a + b);
}

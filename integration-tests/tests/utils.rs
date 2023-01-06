/// Helper function in test to check if two large number are close enough (<0.5%).
pub fn assert_aprox_eq(a: u128, b: u128) {
    assert!(a.abs_diff(b) < (a + b) / 200)
}

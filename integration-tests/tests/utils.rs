/// Helper function in test to check if two large number are close enough (<0.5%).
pub fn assert_aprox_eq(a: u128, b: u128) {
    assert!(a.abs_diff(b) < (a + b) / 200)
}

/// Assert the equality between two structs which need to be converted into string.
/// It's useful when the structs are hard to compare directly due to their type difference.
#[macro_export]
macro_rules! assert_to_string_eq {
    ($left:expr, $right:expr) => {
        assert_eq!($left.to_string(), $right.to_string());
    };
}

#[macro_export]
macro_rules! assert_parses {
    ($parsed:expr, $expected:expr) => {
        assert_eq!($parsed, Ok($expected));
    };
}

#[macro_export]
macro_rules! assert_errors {
    ($parsed:expr) => {
        assert!($parsed.is_err())
    };
}

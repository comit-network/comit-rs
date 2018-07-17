macro_rules! from_str {
    ($name:tt) => {
        impl<'a> From<&'a str> for $name {
            fn from(string: &str) -> Self {
                $name(string.to_string())
            }
        }
    };
}

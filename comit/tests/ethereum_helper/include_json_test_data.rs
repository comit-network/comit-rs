#[macro_export]
macro_rules! include_json_test_data {
    ($file:expr) => {
        serde_json::from_str(include_str!($file)).expect("failed to deserialize test_data")
    };
}

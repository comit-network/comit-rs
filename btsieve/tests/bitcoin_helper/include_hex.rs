#[macro_export]
macro_rules! include_hex {
    ($file:expr) => {{
        use bitcoin::consensus::deserialize;

        let hex = include_str!($file);
        let bytes = hex::decode(hex.trim()).unwrap();
        deserialize(bytes.as_slice()).unwrap()
    }};
}

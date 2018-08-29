macro_rules! include_json_line {
    ($file:expr) => {
        include_str!($file).replace("\n", "").replace(" ", "")
    };
}

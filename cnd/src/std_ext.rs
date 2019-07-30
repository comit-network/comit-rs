pub mod path {
    #[derive(Debug)]
    pub struct PrintablePath<'a>(pub &'a std::path::PathBuf);

    impl<'a> std::fmt::Display for PrintablePath<'a> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let path = self.0.to_str().ok_or_else(|| {
                eprintln!("path is not valid unicode an cannot be printed");
                std::fmt::Error
            })?;

            write!(f, "{}", path)
        }
    }
}

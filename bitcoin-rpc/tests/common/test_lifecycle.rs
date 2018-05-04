extern crate env_logger;

pub fn setup() -> () {
    let _ = env_logger::try_init();
}

pub trait IntoFrame<F> {
    fn into_frame(self) -> F;
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Status {
    OK(u8),
    SE(u8),
    RE(u8),
}

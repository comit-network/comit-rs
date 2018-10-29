#[macro_export]
macro_rules! transition_save {
    ( $repo:expr, $new_state:expr) => {{
        let save_state = $new_state;
        $repo.save(save_state.clone().into());
        return Ok(::futures::Async::Ready(save_state.into()));
    }};
}

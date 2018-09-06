use swap::SwapResponse;

pub trait SwapRequestHandler<Req, Res>: Send + 'static {
    fn handle(&mut self, _request: Req) -> SwapResponse<Res> {
        SwapResponse::Decline
    }
}

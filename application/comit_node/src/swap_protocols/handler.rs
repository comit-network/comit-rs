use swap_protocols::wire_types::SwapResponse;

pub trait SwapRequestHandler<Req>: Send + 'static {
    fn handle(&mut self, _request: Req) -> SwapResponse {
        SwapResponse::Decline
    }
}

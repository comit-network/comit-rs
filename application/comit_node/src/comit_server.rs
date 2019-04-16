use bam::json::{Response, ValidatedIncomingRequest};
use futures::Future;

pub trait Server: Send + Sync + 'static {
    fn handle_request(
        &self,
        request: ValidatedIncomingRequest,
    ) -> Box<dyn Future<Item = Response, Error = ()> + Send>;
}

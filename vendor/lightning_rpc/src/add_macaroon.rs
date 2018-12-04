use futures::Poll;
use http::{header::HeaderValue, Request};
use macaroon::Macaroon;
use tower_grpc::codegen::server::tower::Service;

#[derive(Debug)]
pub struct AddMacaroon<T> {
    inner: T,
    macaroon: Macaroon,
}

impl<T> AddMacaroon<T> {
    pub fn new(inner: T, macaroon: Macaroon) -> Self {
        AddMacaroon { inner, macaroon }
    }
}

impl<T, B> Service for AddMacaroon<T>
where
    T: Service<Request = Request<B>>,
{
    type Request = Request<B>;
    type Response = T::Response;
    type Error = T::Error;
    type Future = T::Future;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.inner.poll_ready()
    }

    fn call(&mut self, req: Self::Request) -> Self::Future {
        // Split the request into the head and the body.
        let (mut head, body) = req.into_parts();

        {
            let headers = &mut head.headers;

            match self.macaroon.to_hex().parse::<HeaderValue>() {
                Ok(header_value) => {
                    headers.insert("macaroon", header_value);
                }
                Err(e) => {
                    warn!("Unable to add macaroon header: {:?}", e);
                }
            }
        }

        let request = Request::from_parts(head, body);

        // Call the inner service
        self.inner.call(request)
    }
}

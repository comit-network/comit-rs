use crate::api::{IntoFrame, ResponseFrameSource};
use futures::{
    future,
    sync::mpsc::{self, UnboundedSender},
    Future, Stream,
};
use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};

type ResponseSource<Frame> = Arc<Mutex<dyn ResponseFrameSource<Frame>>>;

#[derive(DebugStub)]
pub struct Client<Frame, Req, Res> {
    #[debug_stub = "ResponseSource"]
    response_source: ResponseSource<Frame>,
    next_id: u32,
    #[debug_stub = "Sender"]
    sender: UnboundedSender<Frame>,
    request_type: PhantomData<Req>,
    response_type: PhantomData<Res>,
}

#[derive(Debug, PartialEq)]
pub enum Error<F> {
    Send(F),
    Canceled,
}

impl<Frame: 'static + Send, Req: IntoFrame<Frame> + 'static, Res: From<Frame> + 'static>
    Client<Frame, Req, Res>
{
    pub fn new(
        response_source: Arc<Mutex<dyn ResponseFrameSource<Frame>>>,
    ) -> (Self, impl Stream<Item = Frame, Error = ()>) {
        let (sender, receiver) = mpsc::unbounded();

        let client = Self {
            response_source,
            next_id: 0,
            sender,
            request_type: PhantomData,
            response_type: PhantomData,
        };

        (client, receiver)
    }

    pub fn send_request(
        &mut self,
        request: Req,
    ) -> Box<dyn Future<Item = Res, Error = Error<Frame>> + Send> {
        let (request_frame, response_future) = {
            let mut response_source = self.response_source.lock().unwrap();

            let frame_id = self.next_id;

            let request_frame = request.into_frame(frame_id);
            let response_future = response_source
                .on_response_frame(frame_id)
                .map(Res::from)
                .map_err(|_| Error::Canceled);

            self.next_id += 1;

            (request_frame, response_future)
        };

        Box::new(
            self.send_frame(request_frame)
                .and_then(move |_| response_future),
        )
    }

    pub fn send_frame(
        &mut self,
        frame: Frame,
    ) -> Box<dyn Future<Item = (), Error = Error<Frame>> + Send> {
        let send_result = self.sender.unbounded_send(frame);

        match send_result {
            Ok(_) => Box::new(future::ok(())),
            Err(e) => Box::new(future::err(Error::Send(e.into_inner()))),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{api::Status, json};
    use futures::Async;
    use serde_json;
    use std::{collections::HashMap, time::Instant};
    use tokio::runtime::Runtime;

    struct StaticResponseFrameSource {
        responses: HashMap<u32, json::Frame>,
    }

    impl StaticResponseFrameSource {
        fn new() -> Self {
            Self {
                responses: HashMap::new(),
            }
        }

        fn add_response(&mut self, id: u32, response_frame: json::Frame) {
            self.responses.insert(id, response_frame);
        }
    }

    impl ResponseFrameSource<json::Frame> for StaticResponseFrameSource {
        fn on_response_frame(
            &mut self,
            frame_id: u32,
        ) -> Box<dyn Future<Item = json::Frame, Error = ()> + Send> {
            let future = match self.responses.remove(&frame_id) {
                Some(response) => future::ok(response),
                None => future::err(()),
            };

            Box::new(future)
        }
    }

    #[test]
    fn given_a_request_emits_it_on_stream() {
        let response_source = Arc::new(Mutex::new(StaticResponseFrameSource::new()));

        let (mut client, mut receiver) = Client::new(response_source.clone());

        let request = json::Request::new("FOO".into(), HashMap::new(), serde_json::Value::Null);

        {
            let mut response_source = response_source.lock().unwrap();
            response_source.add_response(0, json::Response::new(Status::OK(0)).into_frame(0));
        }

        let mut future = client.send_request(request);

        assert_eq!(
            future.poll(),
            Ok(Async::Ready(json::Response::new(Status::OK(0))))
        );
        assert_eq!(
            receiver.poll(),
            Ok(Async::Ready(Some(
                json::Request::new("FOO".into(), HashMap::new(), serde_json::Value::Null,)
                    .into_frame(0)
            )))
        );
    }

    #[test]
    fn resolves_correct_future_for_request() {
        let response_source = Arc::new(Mutex::new(StaticResponseFrameSource::new()));

        let (mut client, mut receiver) = Client::new(response_source.clone());

        let foo_request = json::Request::new("FOO".into(), HashMap::new(), serde_json::Value::Null);

        let bar_request = json::Request::new("BAR".into(), HashMap::new(), serde_json::Value::Null);

        {
            let mut response_source = response_source.lock().unwrap();
            response_source.add_response(0, json::Response::new(Status::OK(0)).into_frame(0));
            response_source.add_response(1, json::Response::new(Status::SE(0)).into_frame(1));
        }

        {
            let mut foo_response = client.send_request(foo_request);
            assert_eq!(
                foo_response.poll(),
                Ok(Async::Ready(json::Response::new(Status::OK(0))))
            );
        };
        {
            let mut bar_response = client.send_request(bar_request);
            assert_eq!(
                bar_response.poll(),
                Ok(Async::Ready(json::Response::new(Status::SE(0))))
            );
        };

        assert_eq!(
            receiver.poll(),
            Ok(Async::Ready(Some(
                json::Request::new("FOO".into(), HashMap::new(), serde_json::Value::Null)
                    .into_frame(0)
            )))
        );
        assert_eq!(
            receiver.poll(),
            Ok(Async::Ready(Some(
                json::Request::new("BAR".into(), HashMap::new(), serde_json::Value::Null)
                    .into_frame(1)
            )))
        );
    }

    #[derive(Default)]
    struct RememberInvocation {
        when: Option<Instant>,
    }

    impl ResponseFrameSource<json::Frame> for RememberInvocation {
        fn on_response_frame(
            &mut self,
            frame_id: u32,
        ) -> Box<dyn Future<Item = json::Frame, Error = ()> + Send> {
            self.when = Some(Instant::now());

            Box::new(future::ok(
                json::Response::new(Status::OK(0)).into_frame(frame_id),
            ))
        }
    }

    #[test]
    fn registers_response_before_sending_request() {
        let response_frame_source = Arc::new(Mutex::new(RememberInvocation::default()));

        let (mut client, requests) = Client::<json::Frame, json::Request, json::Response>::new(
            response_frame_source.clone(),
        );

        let next_request = requests.into_future().map(|_| Instant::now());
        let response = client.send_request(json::Request::new(
            String::from("BAR"),
            HashMap::default(),
            serde_json::Value::Null,
        ));

        let combined = next_request.map_err(|_| ()).join(response.map_err(|_| ()));

        let mut runtime = Runtime::new().unwrap();

        let (send_timestamp, _) = runtime.block_on(combined).unwrap();

        let response_frame_source = response_frame_source.lock().unwrap();
        let register_timestamp = response_frame_source.when.unwrap();

        assert!(register_timestamp < send_timestamp);
    }
}

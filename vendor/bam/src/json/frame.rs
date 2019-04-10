use crate::{
    api::{Error, FrameHandler, IntoFrame, ResponseFrameSource, Status},
    config::Config,
    json::{
        self,
        header::Header,
        request::{UnknownMandatoryHeaders, UnvalidatedIncomingRequest, ValidatedIncomingRequest},
    },
};
use futures::{
    future,
    sync::oneshot::{self, Sender},
    Future,
};
use serde_json::{self, Value as JsonValue};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub struct Frame {
    #[serde(rename = "type")]
    frame_type: String,
    id: u32,
    payload: JsonValue,
}

impl Frame {
    pub fn new(frame_type: String, id: u32, payload: JsonValue) -> Self {
        Self {
            frame_type,
            id,
            payload,
        }
    }
}

#[derive(DebugStub)]
pub struct JsonFrameHandler {
    next_expected_id: u32,
    #[debug_stub = "ResponseSource"]
    response_source: Arc<Mutex<JsonResponseSource>>,
    config: Config<json::ValidatedIncomingRequest, json::Response>,
}

#[derive(Default, Debug)]
pub struct JsonResponseSource {
    awaiting_responses: HashMap<u32, Sender<json::Response>>,
}

impl ResponseFrameSource<json::Response> for JsonResponseSource {
    fn on_response_frame(
        &mut self,
        frame_id: u32,
    ) -> Box<dyn Future<Item = json::Response, Error = ()> + Send> {
        let (sender, receiver) = oneshot::channel();

        self.awaiting_responses.insert(frame_id, sender);

        Box::new(receiver.map_err(|_| {
            log::warn!(
                "Sender of response future was unexpectedly dropped before response was received."
            )
        }))
    }
}

impl JsonResponseSource {
    pub fn get_awaiting_response(&mut self, id: u32) -> Option<Sender<json::Response>> {
        self.awaiting_responses.remove(&id)
    }
}

impl FrameHandler<json::Frame> for JsonFrameHandler {
    fn handle(
        &mut self,
        frame: json::Frame,
    ) -> Result<Option<Box<dyn Future<Item = json::Frame, Error = ()> + Send + 'static>>, Error>
    {
        match frame.frame_type.as_str() {
            "REQUEST" => {
                if frame.id < self.next_expected_id {
                    return Err(Error::OutOfOrderRequest);
                }

                self.next_expected_id = frame.id + 1;

                let frame_id = frame.id;

                let response_future = serde_json::from_value(frame.payload)
                    .map_err(malformed_request)
                    .and_then(|request| self.validate_request(request))
                    .and_then(|request| self.dispatch_request(request))
                    .unwrap_or_else(|error_response| Box::new(future::ok(error_response)))
                    .and_then(move |response| Ok(response.into_frame(frame_id)));

                Ok(Some(Box::new(response_future)))
            }
            "RESPONSE" => {
                let mut response_source = self.response_source.lock().unwrap();

                let sender = response_source
                    .get_awaiting_response(frame.id)
                    .ok_or(Error::UnexpectedResponse)?;

                log::debug!(
                    "attempting to deserialize payload '{:?}' of frame {} as RESPONSE",
                    frame.payload,
                    frame.id
                );

                let response = serde_json::from_value(frame.payload);

                match response {
                    Ok(response) => {
                        log::debug!("dispatching {:?} to stored handler", response);
                        sender.send(response).unwrap()
                    }
                    Err(e) => log::error!(
                        "payload of frame {} is not a well-formed RESPONSE: {:?}",
                        frame.id,
                        e
                    ),
                }

                Ok(None)
            }
            _ => Err(Error::UnknownFrameType(frame.frame_type)),
        }
    }
}

fn malformed_request(error: serde_json::Error) -> json::Response {
    log::warn!("incoming request was malformed: {:?}", error);

    json::Response::new(Status::SE(0))
}

fn unknown_request_type(request_type: &str) -> json::Response {
    log::warn!("request type '{}' is unknown", request_type);

    json::Response::new(Status::SE(2))
}

fn unknown_mandatory_headers(unknown_headers: UnknownMandatoryHeaders) -> json::Response {
    json::Response::new(Status::SE(1)).with_header(
        "Unsupported-Headers",
        Header::with_value(unknown_headers)
            .expect("list of strings should serialize to serde_json::Value"),
    )
}

impl JsonFrameHandler {
    pub fn create(
        config: Config<json::ValidatedIncomingRequest, json::Response>,
        response_source: Arc<Mutex<JsonResponseSource>>,
    ) -> Self {
        Self {
            next_expected_id: 0,
            response_source,
            config,
        }
    }

    fn validate_request(
        &self,
        request: UnvalidatedIncomingRequest,
    ) -> Result<ValidatedIncomingRequest, json::Response> {
        self.config
            .known_headers_for(request.request_type())
            .ok_or_else(|| unknown_request_type(request.request_type()))
            .and_then(|known_headers| {
                request
                    .ensure_no_unknown_mandatory_headers(known_headers)
                    .map_err(unknown_mandatory_headers)
            })
    }

    fn dispatch_request(
        &mut self,
        request: ValidatedIncomingRequest,
    ) -> Result<Box<dyn Future<Item = json::Response, Error = ()> + Send>, json::Response> {
        self.config
            .request_handler_for(request.request_type())
            .ok_or_else(|| unknown_request_type(request.request_type()))
            .and_then(|request_handler| Ok(request_handler(request)))
    }
}

use crate::{
    api::{Error, FrameHandler, IntoFrame, ResponseFrameSource},
    config::Config,
    json::{self, header::Header, request::UnvalidatedIncomingRequest},
    RequestError,
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
    _type: String,
    id: u32,
    payload: JsonValue,
}

impl Frame {
    pub fn new(_type: String, id: u32, payload: JsonValue) -> Self {
        Self { _type, id, payload }
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
            warn!(
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

impl From<HeaderErrors> for RequestError {
    fn from(header_errors: HeaderErrors) -> Self {
        let unknown_mandatory_headers =
            header_errors.get_error_of_kind(&HeaderErrorKind::UnknownMandatoryHeader);
        if !unknown_mandatory_headers.is_empty() {
            RequestError::UnknownMandatoryHeaders(
                unknown_mandatory_headers
                    .iter()
                    .map(|e| e.key.clone())
                    .collect(),
            )
        } else {
            header_errors.all()[0].clone().into_request_error()
        }
    }
}

impl FrameHandler<json::Frame> for JsonFrameHandler {
    fn handle(
        &mut self,
        frame: json::Frame,
    ) -> Result<Option<Box<dyn Future<Item = json::Frame, Error = ()> + Send + 'static>>, Error>
    {
        match frame._type.as_str() {
            "REQUEST" => {
                let payload = frame.payload;

                let request: UnvalidatedIncomingRequest = serde_json::from_value(payload)
                    .map_err(|e| Error::InvalidFieldFormat("".to_string()))?;

                if frame.id < self.next_expected_id {
                    return Err(Error::OutOfOrderRequest);
                }

                self.next_expected_id = frame.id + 1;

                let frame_id = frame.id;

                let response = self
                    .dispatch_request(request)
                    .then(|result| match result {
                        // TODO: Validate generated response here
                        // TODO check if header or body in response failed to serialize here
                        Ok(response) => Ok(response),
                        Err(e) => Ok(Self::response_from_error(e)),
                    })
                    .and_then(move |response| Ok(response.into_frame(frame_id)));

                Ok(Some(Box::new(response)))
            }
            "RESPONSE" => {
                let mut response_source = self.response_source.lock().unwrap();

                let sender = response_source
                    .get_awaiting_response(frame.id)
                    .ok_or(Error::UnexpectedResponse)?;

                debug!("Dispatching response frame {:?} to stored handler.", frame);

                let response = serde_json::from_value(frame.payload);

                match response {
                    Ok(response) => sender.send(response).unwrap(),
                    // TODO: Decide what happens when response fails to deserialize
                    Err(e) => info!("Failed to deserialize response: {:?}", e),
                }

                Ok(None)
            }
            _ => Err(Error::UnknownFrameType(frame._type)),
        }
    }
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

    fn dispatch_request(
        &mut self,
        request: UnvalidatedIncomingRequest,
    ) -> Box<dyn Future<Item = json::Response, Error = RequestError> + Send + 'static> {
        let _type = request.request_type().to_string();

        let validated_request = match self
            .config
            .known_headers_for(_type.as_ref())
            .ok_or_else(|| RequestError::UnknownRequestType(_type.clone()))
            .and_then(move |known_headers| request.validate(known_headers).map_err(From::from))
        {
            Ok(validated_request) => validated_request,
            Err(e) => return Box::new(future::err(e)),
        };

        let request_handler = match self.config.request_handler_for(_type.as_ref()) {
            Some(request_handler) => request_handler,
            None => return Box::new(future::err(RequestError::UnknownRequestType(_type))),
        };

        Box::new(request_handler(validated_request).map_err(|_| RequestError::HandlerError))
    }

    fn response_from_error(error: RequestError) -> json::Response {
        let status = error.status();
        let response = json::Response::new(status);

        warn!("Failed to dispatch request to handler because: {:?}", error);

        match error {
            RequestError::UnknownMandatoryHeaders(header_keys) => response.with_header(
                "Unsupported-Headers",
                Header::with_value(header_keys)
                    .expect("list of strings should serialize to serde_json::Value"),
            ),
            _ => response,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum HeaderErrorKind {
    UnknownMandatoryHeader,
}

#[derive(Debug, Clone)]
pub(crate) struct HeaderError {
    key: String,
    kind: HeaderErrorKind,
}

impl HeaderError {
    fn into_request_error(self) -> RequestError {
        RequestError::MalformedHeader(self.key)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HeaderErrors {
    errors: Vec<HeaderError>,
}

impl HeaderErrors {
    pub(crate) fn new() -> Self {
        HeaderErrors { errors: vec![] }
    }

    pub(crate) fn add_error(&mut self, key: String, kind: HeaderErrorKind) {
        self.errors.push(HeaderError { key, kind })
    }

    pub(crate) fn all(&self) -> Vec<HeaderError> {
        self.errors.clone()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub(crate) fn get_error_of_kind(&self, kind: &HeaderErrorKind) -> Vec<&HeaderError> {
        self.errors.iter().filter(|x| x.kind == *kind).collect()
    }
}

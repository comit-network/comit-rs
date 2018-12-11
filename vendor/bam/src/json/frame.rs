use crate::{
    api::{Error, FrameHandler, IntoFrame, ResponseFrameSource},
    config::Config,
    json::{self, response::Response},
    RequestError,
};
use futures::{
    future,
    sync::oneshot::{self, Sender},
    Future,
};
use serde_json::{self, Value as JsonValue};
use std::{
    collections::{HashMap, HashSet},
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

impl From<Frame> for Response {
    fn from(f: Frame) -> Self {
        serde_json::from_value(f.payload).unwrap()
    }
}

#[derive(DebugStub)]
pub struct JsonFrameHandler {
    next_expected_id: u32,
    #[debug_stub = "ResponseSource"]
    response_source: Arc<Mutex<JsonResponseSource>>,
    config: Config<json::Request, json::Response>,
}

#[derive(Default, Debug)]
pub struct JsonResponseSource {
    awaiting_responses: HashMap<u32, Sender<json::Frame>>,
}

impl ResponseFrameSource<json::Frame> for JsonResponseSource {
    fn on_response_frame(
        &mut self,
        frame_id: u32,
    ) -> Box<dyn Future<Item = json::Frame, Error = ()> + Send> {
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
    pub fn get_awaiting_response(&mut self, id: u32) -> Option<Sender<json::Frame>> {
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

impl FrameHandler<json::Frame, json::Request, json::Response> for JsonFrameHandler {
    fn create(
        config: Config<json::Request, json::Response>,
    ) -> (Self, Arc<Mutex<dyn ResponseFrameSource<json::Frame>>>) {
        let response_source = Arc::new(Mutex::new(JsonResponseSource::default()));

        let handler = JsonFrameHandler {
            next_expected_id: 0,
            response_source: response_source.clone(),
            config,
        };

        (handler, response_source)
    }

    fn handle(
        &mut self,
        frame: json::Frame,
    ) -> Result<Option<Box<dyn Future<Item = json::Frame, Error = ()> + Send + 'static>>, Error>
    {
        match frame._type.as_str() {
            "REQUEST" => {
                let mut payload = frame.payload;

                let (_type, headers, body) = (
                    payload["type"].take(),
                    payload["headers"].take(),
                    payload["body"].take(),
                );

                if frame.id < self.next_expected_id {
                    return Err(Error::OutOfOrderRequest);
                }

                self.next_expected_id = frame.id + 1;

                let frame_id = frame.id;

                let response = self
                    .dispatch_request(&_type, headers, body)
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

                sender.send(frame).unwrap();

                Ok(None)
            }
            _ => Err(Error::UnknownFrameType(frame._type)),
        }
    }
}

impl JsonFrameHandler {
    fn dispatch_request(
        &mut self,
        _type: &JsonValue,
        headers: JsonValue,
        body: JsonValue,
    ) -> Box<dyn Future<Item = json::Response, Error = RequestError> + Send + 'static> {
        let _type = match _type.as_str() {
            Some(_type) => _type,
            None => {
                return Box::new(future::err(RequestError::MalformedField(
                    "type".to_string(),
                )));
            }
        };

        let request_headers = match headers {
            serde_json::Value::Object(map) => map,
            serde_json::Value::Null => serde_json::Map::default(),
            _ => {
                return Box::new(future::err(RequestError::MalformedField(
                    "headers".to_string(),
                )));
            }
        };

        let parsed_headers = match self
            .config
            .known_headers_for(_type)
            .ok_or_else(|| RequestError::UnknownRequestType(_type.into()))
            .and_then(|known_headers| {
                Self::parse_headers(known_headers, request_headers).map_err(From::from)
            }) {
            Ok(parsed_headers) => parsed_headers,
            Err(e) => return Box::new(future::err(e)),
        };

        let request = json::Request::new(_type.to_string(), parsed_headers, body);

        let request_handler = match self.config.request_handler_for(_type) {
            Some(request_handler) => request_handler,
            None => return Box::new(future::err(RequestError::UnknownRequestType(_type.into()))),
        };

        Box::new(request_handler(request).map_err(|_| RequestError::HandlerError))
    }

    fn parse_headers(
        known_headers: &HashSet<String>,
        request_headers: serde_json::Map<String, JsonValue>,
    ) -> Result<HashMap<String, JsonValue>, HeaderErrors> {
        let mut parsed_headers = HashMap::new();
        let mut header_errors = HeaderErrors::new();

        for (key, value) in request_headers.into_iter() {
            if let Err(e) = Self::validate_header(&value) {
                header_errors.add_error(key.clone(), e)
                // TODO make test that forces continue here
            }

            let value = Self::normalize_compact_header(value);
            let (key, must_understand) = Self::normalize_non_mandatory_header_key(key);

            if !known_headers.contains(key.as_str()) && must_understand {
                header_errors.add_error(key.clone(), HeaderErrorKind::UnknownMandatoryHeader)
                // TODO test for continue
            }

            parsed_headers.insert(key, value);
        }

        if !header_errors.is_empty() {
            return Err(header_errors);
        }

        Ok(parsed_headers)
    }

    // TODO: Replace with JSON schema validation
    fn validate_header(header: &JsonValue) -> Result<(), HeaderErrorKind> {
        match *header {
            JsonValue::Null => Err(HeaderErrorKind::DecodingError),
            JsonValue::Object(ref map) => {
                if map.get("value").is_none() {
                    return Err(HeaderErrorKind::DecodingError);
                }

                if map.len() == 2 && map.get("parameters").is_none() {
                    return Err(HeaderErrorKind::DecodingError);
                }

                if map.len() > 2 {
                    return Err(HeaderErrorKind::DecodingError);
                }

                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn normalize_compact_header(value: JsonValue) -> JsonValue {
        match value {
            JsonValue::Object(_) => value,
            _ => json!({ "value": value }),
        }
    }

    fn normalize_non_mandatory_header_key(mut key: String) -> (String, bool) {
        let non_mandatory = key.starts_with('_');

        if non_mandatory {
            key.remove(0);
        }

        let must_understand = !non_mandatory;

        (key, must_understand)
    }

    fn response_from_error(error: RequestError) -> json::Response {
        let status = error.status();
        let response = json::Response::new(status);

        warn!("Failed to dispatch request to handler because: {:?}", error);

        match error {
            RequestError::UnknownMandatoryHeaders(header_keys) => {
                response.with_header("Unsupported-Headers", header_keys)
            }
            _ => response,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum HeaderErrorKind {
    UnknownMandatoryHeader,
    DecodingError,
}

#[derive(Debug, Clone)]
struct HeaderError {
    key: String,
    kind: HeaderErrorKind,
}

impl HeaderError {
    fn into_request_error(self) -> RequestError {
        RequestError::MalformedHeader(self.key)
    }
}

#[derive(Debug, Clone)]
struct HeaderErrors {
    errors: Vec<HeaderError>,
}

impl HeaderErrors {
    fn new() -> Self {
        HeaderErrors { errors: vec![] }
    }

    fn add_error(&mut self, key: String, kind: HeaderErrorKind) {
        self.errors.push(HeaderError { key, kind })
    }

    fn all(&self) -> Vec<HeaderError> {
        self.errors.clone()
    }

    fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    fn get_error_of_kind(&self, kind: &HeaderErrorKind) -> Vec<&HeaderError> {
        self.errors.iter().filter(|x| x.kind == *kind).collect()
    }
}

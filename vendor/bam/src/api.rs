use futures::Future;

#[derive(Debug, PartialEq)]
pub enum Error {
    UnknownFrameType(String),
    InvalidFieldFormat(String),
    UnexpectedResponse,
    OutOfOrderRequest,
}

#[derive(Debug, PartialEq)]
pub enum RequestError {
    UnknownRequestType(String),
    UnknownMandatoryHeaders(Vec<String>),
    MalformedHeader(String),
    MalformedField(String),
    HandlerError,
}

impl RequestError {
    pub fn status(&self) -> Status {
        match *self {
            RequestError::UnknownRequestType(_) => Status::SE(2),
            RequestError::UnknownMandatoryHeaders(_) => Status::SE(1),
            RequestError::MalformedHeader(_) => Status::SE(0),
            RequestError::MalformedField(_) => Status::SE(0),
            RequestError::HandlerError => Status::SE(0),
        }
    }
}

pub trait FrameHandler<Frame>
where
    Self: Sized,
{
    fn handle(
        &mut self,
        frame: Frame,
    ) -> Result<Option<Box<dyn Future<Item = Frame, Error = ()> + Send + 'static>>, Error>;
}

pub trait IntoFrame<F> {
    fn into_frame(self, id: u32) -> F;
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Status {
    OK(u8),
    SE(u8),
    RE(u8),
}

pub trait ResponseFrameSource<F>: Send {
    fn on_response_frame(&mut self, frame_id: u32) -> Box<dyn Future<Item = F, Error = ()> + Send>;
}

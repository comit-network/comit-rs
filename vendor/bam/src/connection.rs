use crate::api::{self, FrameHandler};
use futures::{Future, Sink, Stream};
use std::{fmt::Debug, io};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_codec::{Decoder, Encoder};

#[derive(Debug)]
pub enum ClosedReason<C> {
    CodecError(C),
    InternalError,
    InvalidFrame(api::Error),
}

pub fn new<
    Frame: Debug + Send + 'static,
    CodecErr: From<io::Error> + Send + Debug + 'static,
    Codec: Encoder<Item = Frame, Error = CodecErr>
        + Decoder<Item = Frame, Error = CodecErr>
        + Send
        + 'static,
    Socket: AsyncRead + AsyncWrite + Send + 'static,
>(
    codec: Codec,
    socket: Socket,
    mut incoming_frames: impl FrameHandler<Frame> + Send + 'static,
    outgoing_frames: impl Stream<Item = Frame, Error = ()> + Send + 'static,
) -> impl Future<Item = (), Error = ClosedReason<CodecErr>> + Send {
    let (sink, stream) = codec.framed(socket).split();

    stream
        .map_err(ClosedReason::CodecError)
        .inspect(|frame| trace!("<--- Incoming {:?}", frame))
        .and_then(move |frame| {
            // Some errors are non-fatal, keep going if we get these
            match incoming_frames.handle(frame) {
                Err(crate::api::Error::UnexpectedResponse) => {
                    warn!("Received unexpected response - ignoring it.");
                    Ok(None)
                }
                Err(crate::api::Error::OutOfOrderRequest) => {
                    warn!("Received out of order request - ignoring it.");
                    Ok(None)
                }
                Ok(result) => Ok(result),
                Err(e) => Err(ClosedReason::InvalidFrame(e)),
            }
        })
        .filter(Option::is_some)
        .map(|option| {
            // FIXME: When we have Never (https://github.com/rust-lang/rust/issues/35121)
            // and Future.recover we should be able to clean this up
            option
                .unwrap()
                .map_err(|_| unreachable!("frame_handler ensures the error never happens"))
        })
        .buffer_unordered(std::usize::MAX)
        .select(outgoing_frames.map_err(|_| ClosedReason::InternalError))
        .inspect(|frame| trace!("---> Outgoing {:?}", frame))
        .forward(sink.sink_map_err(ClosedReason::CodecError))
        .map(|_| ())
}

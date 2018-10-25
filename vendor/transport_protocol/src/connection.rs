use api::{FrameHandler, IntoFrame};
use client::Client;
use config::Config;
use futures::{Future, Sink, Stream};
use std::{fmt::Debug, io};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_codec::{Decoder, Encoder};

pub type ConnectionLoop<E> = Box<Future<Item = (), Error = ClosedReason<E>> + Send>;

#[derive(Debug)]
pub enum ClosedReason<C> {
    CodecError(C),
    InternalError,
    InvalidFrame(::api::Error),
}

#[derive(Debug)]
pub struct Connection<Req, Res, Codec, Socket> {
    config: Config<Req, Res>,
    codec: Codec,
    socket: Socket,
}

impl<
        Frame: Debug + Send + 'static,
        Req: IntoFrame<Frame> + 'static,
        Res: From<Frame> + 'static,
        CodecErr: From<io::Error> + Send + Debug + 'static,
        Codec: Encoder<Item = Frame, Error = CodecErr>
            + Decoder<Item = Frame, Error = CodecErr>
            + Send
            + 'static,
        Socket: AsyncRead + AsyncWrite + Send + 'static,
    > Connection<Req, Res, Codec, Socket>
{
    pub fn new(config: Config<Req, Res>, codec: Codec, socket: Socket) -> Self {
        Self {
            config,
            codec,
            socket,
        }
    }

    pub fn start<FH: FrameHandler<Frame, Req, Res> + Send + 'static>(
        self,
    ) -> (ConnectionLoop<CodecErr>, Client<Frame, Req, Res>) {
        let (sink, stream) = self.codec.framed(self.socket).split();

        let (mut frame_handler, response_source) = FH::new(self.config);
        let (client, request_stream) = Client::new(response_source);

        let connection_loop = stream
            .map_err(ClosedReason::CodecError)
            .inspect(|frame| trace!("<--- Incoming {:?}", frame))
            .and_then(move |frame| {
                frame_handler.handle(frame).then(|result| {
                    // Some errors are non-fatal, keep going if we get these
                    let result = match result {
                        Err(::api::Error::UnexpectedResponse) => {
                            warn!("Received unexpected response - ignoring it.");
                            Ok(None)
                        }
                        Err(::api::Error::OutOfOrderRequest) => {
                            warn!("Received out of order request - ignoring it.");
                            Ok(None)
                        }
                        _ => result,
                    };

                    result.map_err(ClosedReason::InvalidFrame)
                })
            }).filter(Option::is_some)
            .map(Option::unwrap)
            .select(request_stream.map_err(|_| ClosedReason::InternalError))
            .inspect(|frame| trace!("---> Outgoing {:?}", frame))
            .forward(sink.sink_map_err(ClosedReason::CodecError))
            .map(|_| ());

        (Box::new(connection_loop), client)
    }
}

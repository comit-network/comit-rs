use api::{FrameHandler, IntoFrame};
use client::Client;
use config::Config;
use futures::{Future, Sink, Stream};
use std::{fmt::Debug, io};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_codec::{Decoder, Encoder};

#[derive(Debug)]
pub enum Error<C> {
    Codec(C),
    Request,
    FrameHandler(::api::Error),
}

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
    ) -> (
        Box<Future<Item = (), Error = ()> + Send>,
        Client<Frame, Req, Res>,
    ) {
        let (sink, stream) = self.codec.framed(self.socket).split();

        let (mut frame_handler, response_source) = FH::new(self.config);
        let (client, request_stream) = Client::new(response_source);

        let connection_loop = stream
            .map_err(Error::Codec)
            .inspect(|frame| trace!("<--- Incoming {:?}", frame))
            .map(move |frame| {
                let result = frame_handler.handle(frame);

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

                result.map_err(Error::FrameHandler)
            })
            .then(|result| {
                match result {
                    // Stream is fine and we could handle the frame => flatten one layer
                    Ok(Ok(maybe_frame)) => Ok(maybe_frame),
                    // Stream is fine but we failed to handle the frame => error on stream
                    Ok(Err(e)) => Err(e),
                    // Stream is already fucked, continue
                    Err(e) => Err(e),
                }
            })
            .filter(Option::is_some)
            .map(Option::unwrap)
            .select(request_stream.map_err(|_| Error::Request))
            .inspect(|frame| trace!("---> Outgoing {:?}", frame))
            .forward(sink.sink_map_err(Error::Codec))
            .map_err(|e| error!("Error in connection: {:?}", e))
            .map(|_| ());

        (Box::new(connection_loop), client)
    }
}

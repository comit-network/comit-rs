use crate::libp2p_bam::handler::InboundMessage;
use crate::libp2p_bam::{
    handler::ProtocolOutEvent, BamHandler, BamHandlerEvent, PendingInboundRequest,
};
use bam::frame::{JsonFrameCodec, Response};
use futures::{Future, Stream};
use libp2p::core::{ProtocolsHandler, ProtocolsHandlerEvent};
use multistream_select::Negotiated;
use std::collections::{HashMap, HashSet};
use tokio::{
    codec::{Decoder, Encoder, Framed},
    net::{TcpListener, TcpStream},
    prelude::{AsyncRead, AsyncWrite},
};

// Copied from multistream_select crate
#[derive(Debug)]
struct VecRefIntoIter<T>(Vec<T>);

impl<'a, T> IntoIterator for &'a VecRefIntoIter<T>
where
    T: Clone,
{
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.clone().into_iter()
    }
}

pub fn setup_substream<CD: Encoder + Decoder, CL: Encoder + Decoder>(
    codec_dialer: CD,
    codec_listener: CL,
) -> impl Future<
    Item = (
        Framed<Negotiated<TcpStream>, CD>,
        Framed<Negotiated<TcpStream>, CL>,
    ),
    Error = multistream_select::ProtocolChoiceError,
> {
    let listener = TcpListener::bind(&"127.0.0.1:0".parse().unwrap()).unwrap();
    let listener_addr = listener.local_addr().unwrap();

    let listener = listener
        .incoming()
        .into_future()
        .map(|s| s.0.unwrap())
        .map_err(|(e, _)| e.into())
        .and_then(move |connec| {
            multistream_select::listener_select_proto(connec, VecRefIntoIter(vec![b"/proto1"]))
                .map(|result| result.1)
        })
        .map(move |substream| codec_listener.framed(substream));

    let dialer = TcpStream::connect(&listener_addr)
        .from_err()
        .and_then(move |connec| {
            multistream_select::dialer_select_proto(connec, vec![b"/proto1"]).map(|result| result.1)
        })
        .map(move |substream| codec_dialer.framed(substream));

    dialer.join(listener)
}

pub fn setup_substream_with_json_codec() -> impl Future<
    Item = (
        Framed<Negotiated<TcpStream>, JsonFrameCodec>,
        Framed<Negotiated<TcpStream>, JsonFrameCodec>,
    ),
    Error = multistream_select::ProtocolChoiceError,
> {
    setup_substream(JsonFrameCodec::default(), JsonFrameCodec::default())
}

pub fn request_with_no_headers<S: Into<String>>(
    request_type: S,
) -> HashMap<String, HashSet<String>> {
    let mut headers = HashMap::new();
    headers.insert(request_type.into(), HashSet::new());
    headers
}

pub trait IntoFutureWithResponse {
    fn into_future_with_response(
        self,
        response: Response,
    ) -> Box<dyn Future<Item = (), Error = ()> + Send>;
}

impl<TSubstream: 'static + AsyncRead + AsyncWrite + Send> IntoFutureWithResponse
    for BamHandler<TSubstream>
{
    fn into_future_with_response(
        self,
        response: Response,
    ) -> Box<dyn Future<Item = (), Error = ()> + Send> {
        let future = self.into_event_stream().for_each(move |event| {
            // assume we only want to handle requests
            match event {
                ProtocolsHandlerEvent::Custom(ProtocolOutEvent::Message(
                    InboundMessage::Request(PendingInboundRequest { channel, .. }),
                )) => {
                    channel.send(response.clone()).unwrap();
                }
                _ => panic!("expected event to be a PendingInboundRequest"),
            }
            Ok(())
        });

        Box::new(future)
    }
}

pub trait IntoEventStream {
    fn into_event_stream(self) -> Box<dyn Stream<Item = BamHandlerEvent, Error = ()> + Send>;
}

impl<TSubstream: 'static + AsyncRead + AsyncWrite + Send> IntoEventStream
    for BamHandler<TSubstream>
{
    fn into_event_stream(mut self) -> Box<dyn Stream<Item = BamHandlerEvent, Error = ()> + Send> {
        let stream = futures::stream::poll_fn(move || self.poll().map(|ok| ok.map(Some)))
            // ignore all errors
            .map_err(|e| panic!("{:?}", e));

        Box::new(stream)
    }
}

pub trait WaitForFrame {
    fn wait_for_frame(
        self,
    ) -> Box<dyn Future<Item = Option<bam::Frame>, Error = bam::frame::CodecError> + Send>;
}

impl WaitForFrame for Framed<Negotiated<TcpStream>, JsonFrameCodec> {
    fn wait_for_frame(
        self,
    ) -> Box<dyn Future<Item = Option<bam::Frame>, Error = bam::frame::CodecError> + Send> {
        Box::new(
            self.into_future()
                .map(|(item, _stream)| item)
                .map_err(|(error, _stream)| error),
        )
    }
}

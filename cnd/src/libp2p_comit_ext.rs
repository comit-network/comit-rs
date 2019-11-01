use libp2p_comit::frame::Header;

pub trait FromHeader
where
    Self: Sized,
{
    fn from_header(header: Header) -> Result<Self, serde_json::Error>;
}

pub trait ToHeader {
    fn to_header(&self) -> Result<Header, serde_json::Error>;
}

#[macro_export(local_inner_macros)]
macro_rules! header {
    ($e:expr) => {
        header_internal!($e, {
            log::info!("Header was not present, early returning with decline response!");
            let decline_body = DeclineResponseBody {
                reason: Some(SwapDeclineReason::MissingMandatoryHeader),
            };

            return Err(Response::empty().with_header(
                "decision",
                Decision::Declined
                    .to_header()
                    .expect("Decision should not fail to serialize"),
            )
            .with_body(serde_json::to_value(decline_body).expect(
                "decline body should always serialize into serde_json::Value",
            )));
        })
    };
}

#[macro_export]
macro_rules! body {
    ($e:expr) => {
        match $e {
            Ok(body) => body,
            Err(e) => {
                log::error!("Failed to deserialize body because of unexpected field: {:?}", e);
                let decline_body = DeclineResponseBody {
                    reason: Some(SwapDeclineReason::BadJsonField),
                };

                return Err(Response::empty().with_header(
                    "decision",
                    Decision::Declined
                        .to_header()
                        .expect("Decision should not fail to serialize"),
                )
                .with_body(serde_json::to_value(decline_body).expect(
                    "decline body should always serialize into serde_json::Value",
                )));
            }
        }
    };
}

#[macro_export]
macro_rules! header_internal {
    ($e:expr, $none:expr) => {
        match $e {
            Some(Ok(header)) => header,
            Some(Err(e)) => {
                log::error!("Failed to deserialize header because of unexpected field: {:?}", e);

                let decline_body = DeclineResponseBody {
                    reason: Some(SwapDeclineReason::BadJsonField),
                };

                return Err(Response::empty().with_header(
                    "decision",
                    Decision::Declined
                        .to_header()
                        .expect("Decision should not fail to serialize"),
                )
                .with_body(serde_json::to_value(decline_body).expect(
                    "decline body should always serialize into serde_json::Value",
                )));

            },
            None => $none,
        }
    };
}

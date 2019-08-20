#[macro_export(local_inner_macros)]
macro_rules! try_header {
    ($e:expr) => {
        header_internal!($e, {
            let value = Default::default();

            log::info!(
                "Header was not present, falling back to default value: '{:?}'",
                value
            );

            value
        })
    };
}

#[macro_export(local_inner_macros)]
macro_rules! header {
    ($e:expr) => {
        header_internal!($e, {
            log::info!("Header was not present, early returning with decline response!");
            let decline_body = DeclineResponseBody {
                reason: SwapDeclineReason::MissingHeader,
            };

            return Box::new(futures::future::ok(Response::default().with_header(
                "decision",
                Decision::Declined
                    .to_bam_header()
                    .expect("Decision should not fail to serialize"),
            )
            .with_body(serde_json::to_value(decline_body).expect(
                "decline body should always serialize into serde_json::Value",
            ))));
        })
    };
}

#[macro_export]
macro_rules! body {
    ($e:expr) => {
        match $e {
            Ok(body) => body,
            Err(e) => {
                log::error!("Failed to deserialize body: {:?}", e);
                let decline_body = DeclineResponseBody {
                    reason: SwapDeclineReason::MalformedJson,
                };

                return Box::new(futures::future::ok(Response::default().with_header(
                    "decision",
                    Decision::Declined
                        .to_bam_header()
                        .expect("Decision should not fail to serialize"),
                )
                .with_body(serde_json::to_value(decline_body).expect(
                    "decline body should always serialize into serde_json::Value",
                ))));
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
                log::error!("Failed to deserialize header: {:?}", e);

                let decline_body = DeclineResponseBody {
                    reason: SwapDeclineReason::MalformedJson,
                };

                return Box::new(futures::future::ok(Response::default().with_header(
                    "decision",
                    Decision::Declined
                        .to_bam_header()
                        .expect("Decision should not fail to serialize"),
                )
                .with_body(serde_json::to_value(decline_body).expect(
                    "decline body should always serialize into serde_json::Value",
                ))));

            },
            None => $none,
        }
    };
}

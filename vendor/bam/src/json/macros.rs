#[macro_export]
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

#[macro_export]
macro_rules! header {
    ($e:expr) => {
        header_internal!($e, {
            log::info!("Header was not present, early returning with error response (SE00)!");
            return Box::new(futures::future::ok(Response::new(Status::SE(0))));
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
                return Box::new(futures::future::ok(Response::new(Status::SE(0))));
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
                return Box::new(futures::future::ok(Response::new(Status::SE(0))));
            },
            None => $none,
        }
    };
}

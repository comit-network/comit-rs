#[macro_export]
macro_rules! try_header {
    ($e:expr) => {
        match $e {
            Some(Ok(header)) => header,
            Some(Err(_)) => return Response::new(Status::SE(0)),
            None => Default::default(),
        }
    };
}

#[macro_export]
macro_rules! header {
    ($e:expr) => {
        match $e {
            Some(Ok(header)) => header,
            Some(Err(_)) => return Response::new(Status::SE(0)),
            None => return Response::new(Status::SE(0)),
        }
    };
}

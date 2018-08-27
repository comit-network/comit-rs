use snow;
use std::io;

#[derive(Debug)]
pub enum Error<E> {
    Io(io::Error),
    Snow(snow::SnowError),
    Inner(E),
}

impl<E> From<io::Error> for Error<E> {
    fn from(e: io::Error) -> Error<E> {
        Error::Io(e)
    }
}

impl<E> From<snow::SnowError> for Error<E> {
    fn from(e: snow::SnowError) -> Error<E> {
        Error::Snow(e)
    }
}

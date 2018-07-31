use std::io::{self, BufRead, BufReader, Read};

#[derive(Debug)]
pub enum WaitError {
    EndOfStream,
    IO(io::Error),
}

impl From<io::Error> for WaitError {
    fn from(e: io::Error) -> Self {
        WaitError::IO(e)
    }
}

pub trait WaitForMessage {
    fn wait_for_message(self, message: &str) -> Result<(), WaitError>;
}

impl<T> WaitForMessage for T
where
    T: Read,
{
    fn wait_for_message(self, message: &str) -> Result<(), WaitError> {
        let logs = BufReader::new(self);

        for line in logs.lines() {
            if line?.contains(message) {
                return Ok(());
            }
        }

        Err(WaitError::EndOfStream)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn given_logs_when_line_contains_message_should_find_it() {
        let logs = r"
            Message one
            Message two
            Message three
        "
            .as_bytes();

        let result = logs.wait_for_message("Message three");

        assert!(result.is_ok())
    }

}

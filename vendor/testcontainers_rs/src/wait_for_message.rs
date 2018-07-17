use std::io::{BufRead, BufReader, Read};

#[derive(PartialEq, Debug)]
pub enum WaitResult {
    Found,
    EndOfStream,
}

pub trait WaitForMessage {
    fn wait_for_message(self, message: &str) -> WaitResult;
}

impl<T> WaitForMessage for T
where
    T: Read,
{
    fn wait_for_message(self, message: &str) -> WaitResult {
        let logs = BufReader::new(self);

        for line in logs.lines() {
            let line = line.unwrap();

            if line.contains(message) {
                return WaitResult::Found;
            }
        }

        WaitResult::EndOfStream
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

        assert_eq!(result, WaitResult::Found)
    }

}

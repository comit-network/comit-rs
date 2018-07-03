use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;

#[derive(PartialEq, Debug)]
pub enum WaitResult {
    Found,
    EndOfStream,
}

pub trait WaitForMessage {
    fn wait_for_messages(self, message: &[&str]) -> WaitResult;
}

fn remove_messages_contained_in_line(line: String, messages: &mut Vec<&str>) {
    messages.retain(|message| !line.contains(message));
}

impl<T> WaitForMessage for T
where
    T: Read,
{
    fn wait_for_messages(self, messages: &[&str]) -> WaitResult {
        let logs = BufReader::new(self);

        let mut remaining_messages = messages.to_vec();

        for line in logs.lines() {
            let line = line.unwrap();

            remove_messages_contained_in_line(line, &mut remaining_messages);

            if remaining_messages.is_empty() {
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
    fn given_a_line_should_remove_all_messages_that_are_contained() {
        let mut messages = ["one", "two"].to_vec();

        let line = "a line with one in it".into();

        remove_messages_contained_in_line(line, &mut messages);

        assert_eq!(messages, ["two"].to_vec())
    }

    #[test]
    fn given_logs_when_messages_appear_in_given_order_should_release_block() {
        let logs = r"
            Message one
            Message two
            Message three
        "
            .as_bytes();

        let result = logs.wait_for_messages(&["Message two", "Message three"]);

        assert_eq!(result, WaitResult::Found)
    }

    #[test]
    fn given_logs_when_messages_appear_do_not_appear_in_order_should_release_block() {
        let logs = r"
            Message one
            asda adsa Message two
            asdasdsad Message three
        "
            .as_bytes();

        let result = logs.wait_for_messages(&["Message three", "Message two"]);

        assert_eq!(result, WaitResult::Found)
    }

}

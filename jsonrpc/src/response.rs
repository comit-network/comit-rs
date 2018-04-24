use version::Version;

#[derive(Debug, Deserialize, PartialEq)]
pub struct Error {
    code: i32,
    message: String,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Response<R> {
    Successful {
        id: String,
        #[serde(rename = "jsonrpc")]
        version: Version,
        result: R,
    },
    Error {
        id: String,
        #[serde(rename = "jsonrpc")]
        version: Version,
        error: Error,
    },
}

#[cfg(test)]
mod tests {

    use serde_json::from_str;
    use spectral::assert_that;
    use response::Response;

    #[test]
    fn can_deserialize_successful_response_into_generic_type() {
        let result = r#"{
            "jsonrpc": "1.0",
            "id": "test",
            "result": 519521,
            "error": null
        }"#;

        let deserialized_response: Response<i32> = from_str(result).unwrap();

        match deserialized_response {
            Response::Successful {
                id,
                version,
                result,
            } => {
                assert_that(&id).is_equal_to("test".to_string());
                assert_that(&result).is_equal_to(519521);
            }
            Response::Error { id, version, error } => panic!("Should not yield error"),
        }
    }

    #[test]
    fn can_deserialize_error_response() {
        let result = r#"{
            "id": "test",
            "jsonrpc": "1.0",
            "result": null,
            "error": {
                "code": -123,
                "message": "Something went wrong"
            }
        }"#;

        let deserialized_response: Response<i32> = from_str(result).unwrap();

        match deserialized_response {
            Response::Successful {
                id,
                version,
                result,
            } => {
                panic!("Should not yield successful result");
            }
            Response::Error { id, version, error } => {
                assert_that(&id).is_equal_to("test".to_string());
                assert_that(&error.code).is_equal_to(-123);
                assert_that(&error.message).is_equal_to("Something went wrong".to_string());
            }
        }
    }
}

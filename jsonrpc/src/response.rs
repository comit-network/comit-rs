use version::JsonRpcVersion;
use std::result::Result as StdResult;

#[derive(Debug, Deserialize, PartialEq)]
pub struct RpcError {
    code: i32,
    message: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct RpcResponse<R> {
    id: String,
    result: Option<R>,
    error: Option<RpcError>,
}

impl<R> Into<StdResult<R, RpcError>> for RpcResponse<R> {
    fn into(self) -> Result<R, RpcError> {
        match self {
            RpcResponse {
                result: Some(result),
                error: None,
                ..
            } => Ok(result),
            RpcResponse {
                result: None,
                error: Some(rpc_error),
                ..
            } => Err(rpc_error),
            _ => panic!("Response must contain either result or error."),
        }
    }
}

impl<R> RpcResponse<R> {
    pub fn into_result(self) -> StdResult<R, RpcError> {
        self.into()
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

#[cfg(test)]
mod tests {

    use serde_json::from_str;
    use spectral::assert_that;
    use super::*;

    #[test]
    fn can_deserialize_successful_response_into_generic_type() {
        let result = r#"{
            "jsonrpc": "1.0",
            "id": "test",
            "result": 519521,
            "error": null
        }"#;

        let deserialized_response: RpcResponse<i32> = from_str(result).unwrap();

        assert_eq!(deserialized_response.into_result(), Ok(519521));
    }

    #[test]
    fn can_deserialize_successful_btc_rsponse() {
        let result = r#"{
                "id": "curltest",
                "result": 1,
                "error": null
            }"#;

        let result: RpcResponse<i32> = from_str(result).unwrap();

        assert_eq!(result.into_result(), Ok(1))
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

        let deserialized_response: RpcResponse<i32> = from_str(result).unwrap();

        assert_eq!(
            deserialized_response.into_result(),
            Err(RpcError {
                code: -123,
                message: "Something went wrong".to_string(),
            })
        )
    }
}

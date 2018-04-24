extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate spectral;

use self::reqwest::{Client as HTTPClient, Error as ResponseError};
use self::serde::de::DeserializeOwned;
use self::serde::ser::Serialize;

#[derive(Serialize, Debug, Deserialize, PartialEq)]
pub enum Version {
    #[serde(rename = "1.0")]
    V1,

    #[serde(rename = "2.0")]
    V2,
}

#[derive(Serialize)]
pub struct Request<P>
where
    P: Serialize,
{
    jsonrpc: Version,
    id: String,
    method: String,
    params: P,
}

impl Request<()> {
    pub fn new0(version: Version, id: &str, method: &str) -> Request<()> {
        Request::new(version, id, method, ())
    }

    pub fn new1<A>(version: Version, id: &str, method: &str, first: A) -> Request<Vec<A>>
    where
        A: Serialize,
    {
        Request::new(version, id, method, vec![first]) // Handles the special case of one parameter. A tuple would be serialized as a single value.
    }

    pub fn new2<A, B>(
        version: Version,
        id: &str,
        method: &str,
        first: A,
        second: B,
    ) -> Request<(A, B)>
    where
        A: Serialize,
        B: Serialize,
    {
        Request::new(version, id, method, (first, second))
    }

    pub fn new3<A, B, C>(
        version: Version,
        id: &str,
        method: &str,
        first: A,
        second: B,
        third: C,
    ) -> Request<(A, B, C)>
    where
        A: Serialize,
        B: Serialize,
        C: Serialize,
    {
        Request::new(version, id, method, (first, second, third))
    }

    pub fn new4<A, B, C, D>(
        version: Version,
        id: &str,
        method: &str,
        first: A,
        second: B,
        third: C,
        fourth: D,
    ) -> Request<(A, B, C, D)>
    where
        A: Serialize,
        B: Serialize,
        C: Serialize,
        D: Serialize,
    {
        Request::new(version, id, method, (first, second, third, fourth))
    }

    pub fn new5<A, B, C, D, E>(
        version: Version,
        id: &str,
        method: &str,
        first: A,
        second: B,
        third: C,
        fourth: D,
        fifth: E,
    ) -> Request<(A, B, C, D, E)>
    where
        A: Serialize,
        B: Serialize,
        C: Serialize,
        D: Serialize,
        E: Serialize,
    {
        Request::new(version, id, method, (first, second, third, fourth, fifth))
    }

    pub fn new6<A, B, C, D, E, F>(
        version: Version,
        id: &str,
        method: &str,
        first: A,
        second: B,
        third: C,
        fourth: D,
        fifth: E,
        sixth: F,
    ) -> Request<(A, B, C, D, E, F)>
    where
        A: Serialize,
        B: Serialize,
        C: Serialize,
        D: Serialize,
        E: Serialize,
        F: Serialize,
    {
        Request::new(
            version,
            id,
            method,
            (first, second, third, fourth, fifth, sixth),
        )
    }

    fn new<P>(version: Version, id: &str, method: &str, params: P) -> Request<P>
    where
        P: Serialize,
    {
        Request {
            jsonrpc: version,
            id: id.to_string(),
            method: method.to_string(),
            params: params,
        }
    }
}

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

pub struct Client {
    client: HTTPClient,
    url: String,
}

impl Client {
    pub fn new(client: HTTPClient, url: &str) -> Self {
        Client {
            client,
            url: url.to_string(),
        }
    }

    pub fn send<R, T>(&self, request: Request<T>) -> Result<Response<R>, ResponseError>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        self.client
            .post(self.url.as_str())
            .json(&request)
            .send()
            .and_then(|mut res| res.json::<Response<R>>())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use super::reqwest::header::*;
    use super::serde_json::Value;
    use super::spectral::prelude::*;

    #[test]
    fn can_serialize_payload_with_0_params() {
        let payload = Request::new0(Version::V1, "test", "test");
        let expected_payload =
            r#"{"jsonrpc":"1.0","id":"test","method":"test","params":null}"#.to_string();
        let serialized_payload = serde_json::to_string(&payload).unwrap();

        assert_that(&serialized_payload).is_equal_to(expected_payload);
    }

    #[test]
    fn can_serialize_payload_with_1_param() {
        let payload = Request::new1(Version::V1, "test", "test", 100);
        let expected_payload =
            r#"{"jsonrpc":"1.0","id":"test","method":"test","params":[100]}"#.to_string();
        let serialized_payload = serde_json::to_string(&payload).unwrap();

        assert_that(&serialized_payload).is_equal_to(expected_payload);
    }

    #[test]
    fn can_serialize_payload_with_2_params() {
        let payload = Request::new2(Version::V1, "test", "test", 100, "foo");
        let expected_payload =
            r#"{"jsonrpc":"1.0","id":"test","method":"test","params":[100,"foo"]}"#.to_string();
        let serialized_payload = serde_json::to_string(&payload).unwrap();

        assert_that(&serialized_payload).is_equal_to(expected_payload);
    }

    #[test]
    fn can_deserialize_successful_response_into_generic_type() {
        let result = r#"{
            "jsonrpc": "1.0",
            "id": "test",
            "result": 519521,
            "error": null
        }"#;

        let deserialized_response: Response<i32> = serde_json::from_str(result).unwrap();

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

        let deserialized_response: Response<i32> = serde_json::from_str(result).unwrap();

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

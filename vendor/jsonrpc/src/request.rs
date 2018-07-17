use serde::Serialize;
use version::JsonRpcVersion;

#[derive(Debug, Serialize)]
pub struct RpcRequest<P>
where
    P: Serialize,
{
    jsonrpc: JsonRpcVersion,
    id: String,
    method: String,
    params: P,
}

impl RpcRequest<()> {
    pub fn new0(version: JsonRpcVersion, id: &str, method: &str) -> RpcRequest<()> {
        RpcRequest::new(version, id, method, ())
    }

    pub fn new1<A>(version: JsonRpcVersion, id: &str, method: &str, first: A) -> RpcRequest<Vec<A>>
    where
        A: Serialize,
    {
        RpcRequest::new(version, id, method, vec![first]) // Handles the special case of one parameter. A tuple would be serialized as a single value.
    }

    pub fn new2<A, B>(
        version: JsonRpcVersion,
        id: &str,
        method: &str,
        first: A,
        second: B,
    ) -> RpcRequest<(A, B)>
    where
        A: Serialize,
        B: Serialize,
    {
        RpcRequest::new(version, id, method, (first, second))
    }

    pub fn new3<A, B, C>(
        version: JsonRpcVersion,
        id: &str,
        method: &str,
        first: A,
        second: B,
        third: C,
    ) -> RpcRequest<(A, B, C)>
    where
        A: Serialize,
        B: Serialize,
        C: Serialize,
    {
        RpcRequest::new(version, id, method, (first, second, third))
    }

    pub fn new4<A, B, C, D>(
        version: JsonRpcVersion,
        id: &str,
        method: &str,
        first: A,
        second: B,
        third: C,
        fourth: D,
    ) -> RpcRequest<(A, B, C, D)>
    where
        A: Serialize,
        B: Serialize,
        C: Serialize,
        D: Serialize,
    {
        RpcRequest::new(version, id, method, (first, second, third, fourth))
    }

    pub fn new5<A, B, C, D, E>(
        version: JsonRpcVersion,
        id: &str,
        method: &str,
        first: A,
        second: B,
        third: C,
        fourth: D,
        fifth: E,
    ) -> RpcRequest<(A, B, C, D, E)>
    where
        A: Serialize,
        B: Serialize,
        C: Serialize,
        D: Serialize,
        E: Serialize,
    {
        RpcRequest::new(version, id, method, (first, second, third, fourth, fifth))
    }

    pub fn new6<A, B, C, D, E, F>(
        version: JsonRpcVersion,
        id: &str,
        method: &str,
        first: A,
        second: B,
        third: C,
        fourth: D,
        fifth: E,
        sixth: F,
    ) -> RpcRequest<(A, B, C, D, E, F)>
    where
        A: Serialize,
        B: Serialize,
        C: Serialize,
        D: Serialize,
        E: Serialize,
        F: Serialize,
    {
        RpcRequest::new(
            version,
            id,
            method,
            (first, second, third, fourth, fifth, sixth),
        )
    }

    fn new<P>(version: JsonRpcVersion, id: &str, method: &str, params: P) -> RpcRequest<P>
    where
        P: Serialize,
    {
        RpcRequest {
            jsonrpc: version,
            id: id.to_string(),
            method: method.to_string(),
            params: params,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::to_string;
    use spectral::assert_that;

    #[test]
    fn can_serialize_request_with_0_params() {
        let payload = RpcRequest::new0(JsonRpcVersion::V1, "test", "test");
        let expected_payload =
            r#"{"jsonrpc":"1.0","id":"test","method":"test","params":null}"#.to_string();
        let serialized_payload = to_string(&payload).unwrap();

        assert_that(&serialized_payload).is_equal_to(expected_payload);
    }

    #[test]
    fn can_serialize_request_with_1_param() {
        let payload = RpcRequest::new1(JsonRpcVersion::V1, "test", "test", 100);
        let expected_payload =
            r#"{"jsonrpc":"1.0","id":"test","method":"test","params":[100]}"#.to_string();
        let serialized_payload = to_string(&payload).unwrap();

        assert_that(&serialized_payload).is_equal_to(expected_payload);
    }

    #[test]
    fn can_serialize_request_with_2_params() {
        let payload = RpcRequest::new2(JsonRpcVersion::V1, "test", "test", 100, "foo");
        let expected_payload =
            r#"{"jsonrpc":"1.0","id":"test","method":"test","params":[100,"foo"]}"#.to_string();
        let serialized_payload = to_string(&payload).unwrap();

        assert_that(&serialized_payload).is_equal_to(expected_payload);
    }

    #[test]
    fn can_serialize_request_with_new_type_structs() {
        #[derive(Serialize)]
        struct Test(String);

        let payload = RpcRequest::new2(
            JsonRpcVersion::V1,
            "test",
            "test",
            Test("ABCD".to_string()),
            "foo",
        );
        let expected_payload =
            r#"{"jsonrpc":"1.0","id":"test","method":"test","params":["ABCD","foo"]}"#.to_string();
        let serialized_payload = to_string(&payload).unwrap();

        assert_that(&serialized_payload).is_equal_to(expected_payload);
    }
}

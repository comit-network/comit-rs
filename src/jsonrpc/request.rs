use jsonrpc::version::Version;
use serde::Serialize;

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::to_string;
    use spectral::assert_that;

    #[test]
    fn can_serialize_request_with_0_params() {
        let payload = Request::new0(Version::V1, "test", "test");
        let expected_payload =
            r#"{"jsonrpc":"1.0","id":"test","method":"test","params":null}"#.to_string();
        let serialized_payload = to_string(&payload).unwrap();

        assert_that(&serialized_payload).is_equal_to(expected_payload);
    }

    #[test]
    fn can_serialize_request_with_1_param() {
        let payload = Request::new1(Version::V1, "test", "test", 100);
        let expected_payload =
            r#"{"jsonrpc":"1.0","id":"test","method":"test","params":[100]}"#.to_string();
        let serialized_payload = to_string(&payload).unwrap();

        assert_that(&serialized_payload).is_equal_to(expected_payload);
    }

    #[test]
    fn can_serialize_request_with_2_params() {
        let payload = Request::new2(Version::V1, "test", "test", 100, "foo");
        let expected_payload =
            r#"{"jsonrpc":"1.0","id":"test","method":"test","params":[100,"foo"]}"#.to_string();
        let serialized_payload = to_string(&payload).unwrap();

        assert_that(&serialized_payload).is_equal_to(expected_payload);
    }
}

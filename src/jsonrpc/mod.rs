mod client;
//
//#[cfg(test)]
//mod tests {
//
//    use super::*;
//    use super::reqwest::header::*;
//
//    #[test]
//    fn test_adssad() {
//        let mut headers = Headers::new();
//        headers.set(Authorization(Basic {
//            username: "bitcoinrpc".to_string(),
//            password: Some("ic1RhcJW+aO3G36iAevasRZA+Q0pOJ5GG9uoGrC0DSpo".to_string()),
//        }));
//
//        let client = HTTPClient::builder()
//            .default_headers(headers)
//            .build()
//            .unwrap();
//
//        let rpc_client = JsonRpcClient::new(client, "http://127.0.0.1:8332");
//
////        rpc_client.call1("id", "generate", 100);
//        let result: Result<Response<i32, String>, Error> = rpc_client.call0("id", "getblockcount");
//
//        println!("{:?}", result);
//    }
//}
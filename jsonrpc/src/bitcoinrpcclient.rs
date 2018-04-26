use reqwest::{Client as HTTPClient, Error};
use client::Client;
use response::Response;
use request::Request;
use version::Version;
use reqwest::header::{Authorization, Basic, Headers};

struct BitcoinRPCClient {
    client: Client,
}

impl BitcoinRPCClient {
    pub fn new() -> Self {
        let mut headers = Headers::new();
        headers.set(Authorization(Basic {
            username: "bitcoin".to_string(),
            password: Some("password".to_string()),
        }));

        let client = HTTPClient::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        let rpc_client = Client::new(client, "http://127.0.0.1:18332");

        BitcoinRPCClient { client: rpc_client }
    }

    fn get_block_count(&self) -> Result<Response<i32>, Error> {
        let block_count = Request::new0(Version::V1, "block_count", "getblockcount");
        self.client.send(block_count)
    }

    fn get_new_address(&self) -> Result<Response<String>, Error> {
        let new_address_request = Request::new0(Version::V1, "new_address", "getnewaddress");
        self.client.send(new_address_request)
    }

    fn generate_block(&self) -> Result<Response<String>, Error> {
        let generate_block = Request::new1(Version::V1, "generate_new_block", "generate", 1);

        self.client.send(generate_block)
    }
}

#[cfg(test)]
mod tests {
    use client::Client;
    use version::Version;
    use request::Request;
    use response::Response;
    use spectral::assert_that;
    use reqwest::{Client as HTTPClient, Error};
    use reqwest::header::{Authorization, Basic, Headers};
    use bitcoinrpcclient::BitcoinRPCClient;

    #[test]
    fn test_get_block_count() {
        let bitcoin_rpc_client = BitcoinRPCClient::new();

        let result: Result<Response<i32>, Error> = bitcoin_rpc_client.get_block_count();

        println!("result: {:?}", result);

        let response = result.unwrap();
        match response {
            Response::Successful { id, result } => {
                assert_that(&id).is_equal_to("block_count".to_string());
                assert_that(&result).is_equal_to(2);
            }
            Response::Error {
                id,
                /*version, */ error,
            } => panic!("Should not yield error"),
        }
    }

    #[test]
    fn test_get_new_address() {
        let bitcoin_rpc_client = BitcoinRPCClient::new();

        let result: Result<Response<String>, Error> = bitcoin_rpc_client.get_new_address();

        println!("result: {:?}", result);

        let response = result.unwrap();
        match response {
            Response::Successful { id, result } => {
                assert_that(&id).is_equal_to("new_address".to_string());
                assert_that(&result.len()).is_equal_to(34);
            }
            Response::Error {
                id,
                /*version, */
                error,
            } => panic!("Should not yield error"),
        }
    }

    #[test]
    fn test_generate_block() {
        let bitcoin_rpc_client = BitcoinRPCClient::new();

        let result: Result<Response<String>, Error> = bitcoin_rpc_client.generate_block();

        println!("{:?}", result);
        //        assert_that(&result.unwrap()).is_equal_to("mgMrYuUEYweWVDgjwHJm4Rs4YW6pkb9tcq");
    }
}

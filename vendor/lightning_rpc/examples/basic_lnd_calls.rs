/// Documentation for `lighting_rpc` examples
/// To run these examples you need to:
/// 1. Run lnd. see https://github.com/lightningnetwork/lnd/
///    We expect lnd to listen on 127.0.0.1:10009
/// 2. Have access to the lnd's tls.cert:
///     - By default, it is expected to be at ~/.lnd/tls.cert
///     - if using docker: `docker cp lnd_btc:/root/.lnd/tls.cert ~/.lnd/`
/// 3. Have access to lnd admin.macaroon file
///     - By default, it is expected to be at ~/.lnd/admin.macaroon
///     - if using docker: `docker cp lnd_btc:/root/.lnd/admin.macaroon ~/.lnd/`
/// 4.a. run this example, with:
///     - tls.cert file in ~/.lnd/
///     - lnd started with --no-macaroons OR admin.macaroon file in ~/.lnd/
///    `cargo run --package lightning_rpc --example basic_lnd_calls`
/// 4.b. run this example, passing tls.cert file path, lnd started with --no-macaroons
///    `cargo run --package lightning_rpc --example basic_lnd_calls -- $HOME/.lnd/tls.cert`
/// 4.c. run this example, passing both tls.cert and macaroon file paths
///    `cargo run --package lighthning_rpc --example basic_lnd_calls -- $HOME/.lnd/tls.cert $HOME/.lnd/admin.macaroon`
extern crate hex;
extern crate http;
extern crate lightning_rpc;
extern crate tower_grpc;

use lightning_rpc::{
    certificate::Certificate, lightning_rpc_api::LightningRpcApi, lnd_api::LndClient, lnrpc::*,
    macaroon::Macaroon, FromFile,
};
use std::env;

static LND_URI: &'static str = "127.0.0.1:10009";
// This is https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Origin
static ORIGIN_URI: &'static str = "http://127.0.0.1";

fn main() {
    let cert_path = env::args().nth(1);
    let macaroon_path = env::args().nth(2);

    let cert_path = cert_path.unwrap_or({ format!("{}/.lnd/tls.cert", env::var("HOME").unwrap()) });
    let macaroon_path =
        macaroon_path.unwrap_or({ format!("{}/.lnd/admin.macaroon", env::var("HOME").unwrap()) });

    let mut lnd_client = create_lnd_client(cert_path, macaroon_path);

    let info = lnd_client.get_info();
    println!("Lnd Info:\n{:#?}", info.unwrap());

    add_invoice(&mut lnd_client);
    // do something with ti invoice_response.payment_request.
    add_invoice_with_pre_image(&mut lnd_client);

    send_payment(&mut lnd_client);
}

fn add_invoice(lnd_client: &mut LndClient) {
    let invoice = Invoice {
        memo: "Example".to_string(),
        value: 5400,
        ..Default::default()
    };

    let response = lnd_client.add_invoice(invoice).unwrap();
    println!("Payment request: {}", response.payment_request);
}

// This can only be ran once per LND
// as LND does not accept 2 invoices with the same image
fn add_invoice_with_pre_image(lnd_client: &mut LndClient) {
    let pre_image: Vec<u8> =
        hex::decode("68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4c2c").unwrap();

    let invoice = Invoice {
        memo: "Test".to_string(),
        r_preimage: pre_image,
        value: 5400,
        ..Default::default()
    };

    match lnd_client.add_invoice(invoice) {
        Ok(response) => {
            let hash = hex::encode(response.r_hash);
            println!("Hash: {:#?}", hash);
        }
        Err(e) => {
            // If examples are run twice on the same lnd, failure is expected
            println!("Add Invoice Error: {:?}", e)
        }
    };
}

fn create_lnd_client(cert_path: String, macaroon_path: String) -> LndClient {
    let certificate = Certificate::from_file(cert_path).unwrap().into();
    let macaroon = Macaroon::from_file(macaroon_path).ok();
    let lnd_addr = LND_URI.parse().unwrap();
    let origin_uri: http::Uri = ORIGIN_URI.parse().unwrap();

    LndClient::new(certificate, macaroon, lnd_addr, origin_uri).unwrap()
}

fn send_payment(lnd_client: &mut LndClient) {
    let payment_request = "lnsb1pdk0tr7pp5gfsjkmqgdgzeadnu7ykxjpsdy2\
    m7jyrys9zcxeq6kffz9vhucvrqdqvg4uxzmtsd3jscqzysxq97zvuqmcs8396psp7my90d\
    sq2ws2r34u3fzj6v7rfrlgmdcrrvl6twyt8q2xa9cm6dyd6mr0ppemh6exxjj45smrsl8kgy2uqt667xwwesjtsq9uft9s"
        .to_string();

    let send_request = SendRequest {
        payment_request,
        amt: 5400,
        ..Default::default()
    };

    let response = lnd_client.send_payment(send_request);
    // This will error out as it is a random invoice (cannot find path)
    println!("Payment response (error) : {:?}", response);
}

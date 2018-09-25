use bitcoin_support::BitcoinQuantity;
use ethereum_support::EthereumQuantity;
use swap_protocols::{
    handler::SwapRequestHandler,
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    rfc003,
    wire_types::{Asset, Ledger, SwapProtocol, SwapRequestHeaders},
};
use transport_protocol::{
    config::Config,
    json::{Request, Response},
    Status,
};

pub fn json_config<
    H: SwapRequestHandler<
            rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>,
            rfc003::AcceptResponse<Bitcoin, Ethereum>,
        > + SwapRequestHandler<
            rfc003::Request<Ethereum, Bitcoin, EthereumQuantity, BitcoinQuantity>,
            rfc003::AcceptResponse<Ethereum, Bitcoin>,
        >,
>(
    mut handler: H,
) -> Config<Request, Response> {
    Config::new().on_request(
        "SWAP",
        &[
            "target_ledger",
            "source_ledger",
            "target_asset",
            "source_asset",
            "swap_protocol",
        ],
        move |request: Request| {
            let headers = SwapRequestHeaders {
                source_ledger: header!(request.get_header("source_ledger")),
                target_ledger: header!(request.get_header("target_ledger")),
                source_asset: header!(request.get_header("source_asset")),
                target_asset: header!(request.get_header("target_asset")),
                swap_protocol: header!(request.get_header("swap_protocol")),
            };

            match headers.swap_protocol {
                SwapProtocol::ComitRfc003 => match headers {
                    SwapRequestHeaders {
                        source_ledger: Ledger::Bitcoin,
                        source_asset:
                            Asset::Bitcoin {
                                quantity: source_quantity,
                            },
                        target_ledger: Ledger::Ethereum,
                        target_asset:
                            Asset::Ether {
                                quantity: target_quantity,
                            },
                        ..
                    } => {
                        let request = rfc003::Request::new(
                            Bitcoin {},
                            Ethereum {},
                            source_quantity,
                            target_quantity,
                            body!(request.get_body()),
                        );
                        handler.handle(request).into()
                    }
                    SwapRequestHeaders {
                        source_ledger: Ledger::Ethereum,
                        source_asset:
                            Asset::Ether {
                                quantity: source_quantity,
                            },
                        target_ledger: Ledger::Bitcoin,
                        target_asset:
                            Asset::Bitcoin {
                                quantity: target_quantity,
                            },
                        ..
                    } => {
                        let request = rfc003::Request::new(
                            Ethereum {},
                            Bitcoin {},
                            source_quantity,
                            target_quantity,
                            body!(request.get_body()),
                        );
                        handler.handle(request).into()
                    }
                    _ => Response::new(Status::SE(21)),
                },
            }
        },
    )
}

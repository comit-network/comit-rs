use bitcoin_support::{BitcoinQuantity, Blocks};
use comit_client;
use futures::{
    future::{self, Either},
    Future, Stream,
};
use lightning_rpc::{
    lnrpc::{InvoiceSubscription, Request},
    LndClient,
};
use secp256k1_support;
use std::sync::{Arc, Mutex};
use swap_protocols::{
    asset::Asset,
    ledger::Lightning,
    rfc003::{
        self,
        events::{
            CommunicationEvents, Deployed, Funded, LedgerEvents, RedeemedOrRefunded,
            ResponseFuture, StateMachineResponseFuture,
        },
        ledger::{FundTransaction, Ledger, RedeemTransaction},
        roles::Alice,
        secret::{Secret, SecretHash},
        state_machine::HtlcParams,
    },
};

pub struct AliceToBob<C, AL: Ledger> {
    lnd_client: Arc<Mutex<LndClient>>,
    comit_client: Arc<C>,
    response_future:
        Option<Box<StateMachineResponseFuture<AL::Identity, secp256k1_support::PublicKey, Blocks>>>,
}

pub struct AliceLightningEvents {
    lnd_client: Arc<Mutex<LndClient>>,
    invoice_paid: Option<Box<Deployed<Lightning>>>,
    dummy_funded: Box<Funded<Lightning>>,
    dummy_redeemed_or_refunded: Box<RedeemedOrRefunded<Lightning>>,
}

impl AliceLightningEvents {
    fn new(lnd_client: Arc<Mutex<LndClient>>, secret: Secret) -> Self {
        AliceLightningEvents {
            lnd_client,
            invoice_paid: None,
            dummy_funded: Box::new(future::ok(Some(FundTransaction(())))),
            dummy_redeemed_or_refunded: Box::new(future::ok(Either::A(RedeemTransaction::<
                Lightning,
            > {
                transaction: (),
                secret,
            }))),
        }
    }
}

impl<C: comit_client::Client, AL: Ledger, AA: Asset, BA: Asset>
    CommunicationEvents<Alice<AL, Lightning, AA, BA>> for AliceToBob<C, AL>
{
    fn request_responded(
        &mut self,
        request: &comit_client::rfc003::Request<AL, Lightning, AA, BA>,
    ) -> &mut ResponseFuture<Alice<AL, Lightning, AA, BA>> {
        let lnd_client = Arc::clone(&self.lnd_client);
        let comit_client = Arc::clone(&self.comit_client);
        self.response_future.get_or_insert_with(move || {
            let mut lnd_client = lnd_client.lock().unwrap();
            let request = request.clone();
            let got_response = comit_client
                .send_swap_request(request.clone())
                .map_err(rfc003::Error::SwapResponse)
                .map(|result| result.map(Into::into));

            let invoice_was_added = lnd_client
                .subscribe_invoices(Request::new(InvoiceSubscription {
                    add_index: 0,
                    settle_index: 0,
                }))
                .map_err(|e| {
                    error!("Couldn't subscribe to invoices: {:?}", e);
                    rfc003::Error::Lnd
                })
                .and_then(|invoices| {
                    invoices
                        .into_inner()
                        .filter(move |invoice| {
                            SecretHash(invoice.r_hash.clone()) == request.secret_hash
                        })
                        .into_future()
                        .map_err(|(e, _)| {
                            error!("Error during invoice subscription: {:?}", e);
                            rfc003::Error::Lnd
                        })
                        .and_then(|(invoice, _)| {
                            invoice.ok_or_else(|| {
                                error!("Invoice subscription terminated before finding invoice");
                                rfc003::Error::Lnd
                            })
                        })
                });

            Box::new(
                got_response
                    .join(invoice_was_added)
                    .map(|(response, _)| response),
            )
        })
    }
}

impl LedgerEvents<Lightning, BitcoinQuantity> for AliceLightningEvents {
    fn htlc_deployed(
        &mut self,
        htlc_params: HtlcParams<Lightning, BitcoinQuantity>,
    ) -> &mut Deployed<Lightning> {
        let lnd_client = Arc::clone(&self.lnd_client);
        self.invoice_paid.get_or_insert_with(|| {
            let mut lnd_client = lnd_client.lock().unwrap();
            let invoice_was_settled = lnd_client
                .subscribe_invoices(Request::new(InvoiceSubscription {
                    add_index: 0,
                    settle_index: 0,
                }))
                .map_err(|e| {
                    error!("Couldn't subscribe to invoices: {:?}", e);
                    rfc003::Error::Lnd
                })
                .and_then(|invoices| {
                    invoices
                        .into_inner()
                        .filter_map(move |invoice| {
                            if invoice.settled
                                && SecretHash(invoice.r_hash.clone()) == htlc_params.secret_hash
                            {
                                Some(())
                            } else {
                                None
                            }
                        })
                        .into_future()
                        .map_err(|(e, _)| {
                            error!("Error during invoice subscription: {:?}", e);
                            rfc003::Error::Lnd
                        })
                        .and_then(|(invoice, _)| {
                            invoice.ok_or_else(|| {
                                error!("Invoice subscription terminated before finding invoice");
                                rfc003::Error::Lnd
                            })
                        })
                });

            Box::new(invoice_was_settled)
        })
    }

    fn htlc_funded(
        &mut self,
        _htlc_params: HtlcParams<Lightning, BitcoinQuantity>,
        _htlc_location: &(),
    ) -> &mut Funded<Lightning> {
        &mut self.dummy_funded
    }

    fn htlc_redeemed_or_refunded(
        &mut self,
        _htlc_params: HtlcParams<Lightning, BitcoinQuantity>,
        _htlc_location: &(),
    ) -> &mut RedeemedOrRefunded<Lightning> {
        &mut self.dummy_redeemed_or_refunded
    }
}

pub struct BobLightningEvents {
    lnd_client: Arc<Mutex<LndClient>>,
    invoice_paid: Option<Box<Deployed<Lightning>>>,
    dummy_funded: Box<Funded<Lightning>>,
    //    redeemed_or_refunded: Option<Box<RedeemedOrRefunded<Lightning>>>,
    secret: Arc<Mutex<Option<Secret>>>,
}

impl BobLightningEvents {
    fn new(lnd_client: Arc<Mutex<LndClient>>) -> Self {
        BobLightningEvents {
            lnd_client,
            invoice_paid: None,
            dummy_funded: Box::new(future::ok(Some(FundTransaction(())))),
            // redeemed_or_refunded: None,
            secret: Arc::new(Mutex::new(None)),
        }
    }
}

impl LedgerEvents<Lightning, BitcoinQuantity> for BobLightningEvents {
    fn htlc_deployed(
        &mut self,
        htlc_params: HtlcParams<Lightning, BitcoinQuantity>,
    ) -> &mut Deployed<Lightning> {
        let lnd_client = Arc::clone(&self.lnd_client);
        self.invoice_paid.get_or_insert_with(|| {
            let mut lnd_client = lnd_client.lock().unwrap();
            // let payment_was_made = unimplemented!();

            // Box::new(payment_was_made)
            unimplemented!()
        })
    }

    fn htlc_funded(
        &mut self,
        _htlc_params: HtlcParams<Lightning, BitcoinQuantity>,
        _htlc_location: &(),
    ) -> &mut Funded<Lightning> {
        &mut self.dummy_funded
    }

    fn htlc_redeemed_or_refunded(
        &mut self,
        _htlc_params: HtlcParams<Lightning, BitcoinQuantity>,
        _htlc_location: &(),
    ) -> &mut RedeemedOrRefunded<Lightning> {
        unimplemented!()
        //&mut self.redeemed_or_refunded
    }
}

// TODO: DO THE PAYMENTS POLL THING

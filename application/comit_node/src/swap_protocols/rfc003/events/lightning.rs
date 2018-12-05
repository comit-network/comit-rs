use bitcoin_support::{BitcoinQuantity, Blocks};
use comit_client;
use futures::{
    future::{self, Either},
    stream, Future, IntoFuture, Stream,
};
use lightning_rpc::{
    lnrpc::{InvoiceSubscription, ListPaymentsRequest, Payment, Request},
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

impl<C, AL: Ledger> AliceToBob<C, AL> {
    pub fn new(comit_client: Arc<C>, lnd_client: Arc<Mutex<LndClient>>) -> Self {
        Self {
            comit_client,
            lnd_client,
            response_future: None,
        }
    }
}

pub struct AliceLightningEvents {
    lnd_client: Arc<Mutex<LndClient>>,
    invoice_paid: Option<Box<Deployed<Lightning>>>,
    dummy_funded: Box<Funded<Lightning>>,
    dummy_redeemed_or_refunded: Box<RedeemedOrRefunded<Lightning>>,
}

impl AliceLightningEvents {
    pub fn new(lnd_client: Arc<Mutex<LndClient>>, secret: Secret) -> Self {
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
    redeemed_or_refunded: Option<Box<RedeemedOrRefunded<Lightning>>>,
    secret: Arc<Mutex<Option<Secret>>>,
}

impl BobLightningEvents {
    pub fn new(lnd_client: Arc<Mutex<LndClient>>) -> Self {
        BobLightningEvents {
            lnd_client,
            invoice_paid: None,
            dummy_funded: Box::new(future::ok(Some(FundTransaction(())))),
            redeemed_or_refunded: None,
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
        let secret_store_hack = Arc::clone(&self.secret);
        self.invoice_paid.get_or_insert_with(|| {
            Box::new(
                lnd_client
                    .subscribe_payments()
                    .filter(move |payment| {
                        payment.payment_hash == format!("{:x}", htlc_params.secret_hash)
                    })
                    .into_future()
                    .map_err(|(e, _)| e)
                    .and_then(|(payment, _)| {
                        payment.ok_or_else(|| {
                            error!("Payment stream stopped before payment found");
                            rfc003::Error::Lnd
                        })
                    })
                    .map(move |payment| {
                        use std::str::FromStr;
                        let mut secret = secret_store_hack.lock().unwrap();
                        *secret = Some(
                            Secret::from_str(&payment.payment_preimage)
                                .expect("This cannot happen"),
                        );
                        ()
                    }),
            )
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
        let secret = self.secret.lock().unwrap().clone();
        match secret {
            Some(secret) => self.redeemed_or_refunded.get_or_insert_with(|| {
                Box::new(future::ok(Either::A(RedeemTransaction::<Lightning> {
                    transaction: (),
                    secret,
                })))
            }),
            None => unreachable!(
                "We should never have got polled here unless we got the secret in htlc_deployed"
            ),
        }
    }
}

trait StreamPayments {
    fn subscribe_payments(&self) -> Box<Stream<Item = Payment, Error = rfc003::Error> + Send>;
}

impl StreamPayments for Arc<Mutex<LndClient>> {
    fn subscribe_payments(&self) -> Box<Stream<Item = Payment, Error = rfc003::Error> + Send> {
        use std::time::{Duration, Instant};
        use tokio::timer::Interval;
        let mut seen_payments = Vec::new();
        let lnd_client = Arc::clone(&self);
        Box::new(
            Interval::new(Instant::now(), Duration::from_secs(1))
                .map_err(|e| rfc003::Error::Internal(String::from("Interval stopped working")))
                .and_then(move |_tick| {
                    let mut lnd_client = lnd_client.lock().unwrap();
                    lnd_client
                        .list_payments(Request::new(ListPaymentsRequest {}))
                        .into_future()
                        .map(|payments_response| payments_response.into_inner().payments)
                        .map_err(|e| {
                            error!("List payments failed: {:?}", e);
                            rfc003::Error::Lnd
                        })
                })
                .map(stream::iter_ok)
                .flatten()
                .filter(move |payment| {
                    let is_new_payment = !seen_payments.contains(payment);
                    if is_new_payment {
                        seen_payments.push(payment.clone())
                    }
                    is_new_payment
                }),
        )
    }
}

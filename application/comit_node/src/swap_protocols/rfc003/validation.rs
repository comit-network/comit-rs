use bitcoin_support::{BitcoinQuantity, OutPoint, Transaction, TransactionId};
use swap_protocols::{
    asset::Asset,
    ledger::Bitcoin,
    rfc003::{bitcoin::bitcoin_htlc_address, state_machine::OngoingSwap, Ledger, SecretHash},
};

#[derive(Debug, PartialEq)]
pub struct PoorGuy;

pub trait IsContainedInSourceLedgerTransaction<SL, TL, SA, TA, S>: Send + Sync
where
    SL: Ledger,
    TL: Ledger,
    SA: Asset,
    TA: Asset,
    S: Into<SecretHash> + Send + Sync + Clone,
{
    fn is_contained_in_source_ledger_transaction(
        swap: OngoingSwap<SL, TL, SA, TA, S>,
        tx: &SL::TxId, // TODO: should be full tx
    ) -> Result<SL::HtlcLocation, PoorGuy>;
}

pub trait IsContainedInTargetLedgerTransaction<SL, TL, SA, TA, S>: Send + Sync
where
    SL: Ledger,
    TL: Ledger,
    SA: Asset,
    TA: Asset,
    S: Into<SecretHash> + Send + Sync + Clone,
{
    fn is_contained_in_target_ledger_transaction(
        swap: OngoingSwap<SL, TL, SA, TA, S>,
        tx: &TL::TxId, // TODO: should be full tx
    ) -> Result<TL::HtlcLocation, PoorGuy>;
}

impl<TL, TA, S> IsContainedInSourceLedgerTransaction<Bitcoin, TL, BitcoinQuantity, TA, S>
    for BitcoinQuantity
where
    TL: Ledger,
    TA: Asset,
    S: Into<SecretHash> + Send + Sync + Clone,
{
    #[allow(unreachable_code, unused_variables)] // TODO: remove once properly implemented
    fn is_contained_in_source_ledger_transaction(
        swap: OngoingSwap<Bitcoin, TL, BitcoinQuantity, TA, S>,
        tx: &TransactionId,
    ) -> Result<OutPoint, PoorGuy> {
        let transaction: Transaction = unimplemented!();

        let (vout, txout) = transaction
            .output
            .iter()
            .enumerate()
            .find(|(vout, txout)| {
                txout.script_pubkey == bitcoin_htlc_address(&swap).script_pubkey()
            }).unwrap();

        let location = OutPoint {
            txid: tx.clone(),
            vout: vout as u32,
        };

        let actual_value = BitcoinQuantity::from_satoshi(txout.value);
        let required_value = swap.source_asset;

        debug!("Value of HTLC at {:?} is {}", location, actual_value);

        let has_enough_money = actual_value >= required_value;

        trace!(
            "{} >= {} -> {}",
            actual_value,
            required_value,
            has_enough_money
        );
        if has_enough_money {
            Ok(location)
        } else {
            Err(PoorGuy)
        }
    }
}

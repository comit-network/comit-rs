use bitcoin_support::{BitcoinQuantity, OutPoint, Transaction};
use swap_protocols::{
    asset::Asset,
    ledger::{Bitcoin, Ledger as SwapProtocolsLedger},
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
        transaction: SL::Transaction,
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
        tx: TL::Transaction,
    ) -> Result<TL::HtlcLocation, PoorGuy>;
}

impl<TL, TA, S> IsContainedInSourceLedgerTransaction<Bitcoin, TL, BitcoinQuantity, TA, S>
    for BitcoinQuantity
where
    TL: Ledger,
    TA: Asset,
    S: Into<SecretHash> + Send + Sync + Clone,
{
    fn is_contained_in_source_ledger_transaction(
        swap: OngoingSwap<Bitcoin, TL, BitcoinQuantity, TA, S>,
        transaction: <Bitcoin as SwapProtocolsLedger>::Transaction,
    ) -> Result<OutPoint, PoorGuy> {
        let transaction: Transaction = transaction.into();

        let (vout, txout) = transaction
            .output
            .iter()
            .enumerate()
            .find(|(_, txout)| txout.script_pubkey == bitcoin_htlc_address(&swap).script_pubkey())
            .unwrap();

        let location = OutPoint {
            txid: transaction.txid(),
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

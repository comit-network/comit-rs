use crate::{
    btsieve::{ethereum::ReceiptByHash, fetch_blocks_since, BlockByHash, LatestBlock},
    ethereum::{Address, Block, Hash, Transaction, TransactionReceipt},
};
use anyhow::Result;
use genawaiter::GeneratorState;
use time::OffsetDateTime;

// This tracing context is useful because it conveys information through its
// name although we skip all fields because they would add too much noise.
#[tracing::instrument(level = "debug", skip(connector, start_of_swap, expected_bytecode))]
pub async fn watch_for_contract_creation<C>(
    connector: &C,
    start_of_swap: OffsetDateTime,
    expected_bytecode: &[u8],
) -> Result<(Transaction, Address)>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + ReceiptByHash,
{
    let (transaction, receipt) =
        matching_transaction_and_receipt(connector, start_of_swap, |transaction| {
            // transaction.to address is None if, and only if, the transaction
            // creates a contract.

            let is_contract_creation = transaction.to.is_none();
            let is_expected_contract = transaction.input.as_slice() == expected_bytecode;

            if !is_contract_creation {
                tracing::trace!("rejected because transaction doesn't create a contract");
            }

            if !is_expected_contract {
                tracing::trace!("rejected because contract code doesn't match");

                // only compute levenshtein distance if we are on trace level, converting to hex is expensive at this scale
                if tracing::level_enabled!(tracing::level_filters::LevelFilter::TRACE) {
                    let actual = hex::encode(&transaction.input);
                    let expected = hex::encode(expected_bytecode);

                    let distance = levenshtein::levenshtein(&actual, &expected);

                    // We probably need to find a meaningful value here, expiry is 4 bytes.
                    if distance < 10 {
                        tracing::warn!("found contract with slightly different parameters (levenshtein-distance < 10), this could be a bug!")
                    }
                }
            }

            is_contract_creation && is_expected_contract
        })
            .await?;

    match receipt.contract_address {
        Some(location) => Ok((transaction, location)),
        None => Err(anyhow::anyhow!("contract address missing from receipt")),
    }
}

pub async fn matching_transaction_and_receipt<C, F>(
    connector: &C,
    start_of_swap: OffsetDateTime,
    matcher: F,
) -> Result<(Transaction, TransactionReceipt)>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + ReceiptByHash,
    F: Fn(&Transaction) -> bool,
{
    let mut block_generator = fetch_blocks_since(connector, start_of_swap);

    loop {
        match block_generator.async_resume().await {
            GeneratorState::Yielded(block) => {
                let block_span = tracing::error_span!("block", hash = %block.hash, tx_count = %block.transactions.len());
                let _enter_block_span = block_span.enter();

                for transaction in block.transactions.into_iter() {
                    let tx_hash = transaction.hash;
                    let tx_span = tracing::error_span!("tx", hash = %tx_hash);
                    let _enter_tx_span = tx_span.enter();

                    if matcher(&transaction) {
                        let receipt = connector.receipt_by_hash(tx_hash).await?;
                        if !receipt.successful {
                            // This can be caused by a failed attempt to complete an action,
                            // for example, sending a transaction with low gas.
                            tracing::warn!("transaction matched but status was NOT OK");
                            continue;
                        }
                        tracing::info!("transaction matched");
                        return Ok((transaction, receipt));
                    }
                }

                tracing::info!("no transaction matched")
            }
            GeneratorState::Complete(Err(e)) => return Err(e),
            // By matching against the never type explicitly, we assert that the `Ok` value of the
            // result is actually the never type and has not been changed since this line was
            // written. The never type can never be constructed, so we can never reach this line.
            GeneratorState::Complete(Ok(never)) => match never {},
        }
    }
}

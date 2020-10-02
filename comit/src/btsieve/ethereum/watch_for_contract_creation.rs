use crate::{
    btsieve::{ethereum::ReceiptByHash, fetch_blocks_since, BlockByHash, LatestBlock},
    ethereum::{Address, Block, Hash, Transaction, TransactionReceipt},
};
use anyhow::Result;
use genawaiter::GeneratorState;
use time::OffsetDateTime;
use tracing_futures::Instrument;

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
    F: Fn(&Transaction) -> bool + Clone,
{
    let mut block_generator = fetch_blocks_since(connector, start_of_swap);

    loop {
        match block_generator.async_resume().await {
            GeneratorState::Yielded(block) => {
                if let Some(result) = process_block(block, connector, matcher.clone()).await? {
                    return Ok(result);
                }
            }
            GeneratorState::Complete(Err(e)) => return Err(e),
            // By matching against the never type explicitly, we assert that the `Ok` value of the
            // result is actually the never type and has not been changed since this line was
            // written. The never type can never be constructed, so we can never reach this line.
            GeneratorState::Complete(Ok(never)) => match never {},
        }
    }
}

#[tracing::instrument(name = "block", skip(block, connector, matcher), fields(hash = %block.hash, tx_count = %block.transactions.len()))]
async fn process_block<C, F>(
    block: Block,
    connector: &C,
    matcher: F,
) -> Result<Option<(Transaction, TransactionReceipt)>>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + ReceiptByHash,
    F: Fn(&Transaction) -> bool + Clone,
{
    for transaction in block.transactions.into_iter() {
        if let Some(result) = process_transaction(transaction, connector, matcher.clone())
            .in_current_span()
            .await?
        {
            return Ok(Some(result));
        }
    }

    tracing::debug!("no transaction matched");

    Ok(None)
}

#[tracing::instrument(name = "tx", skip(tx, connector, matcher), fields(hash = %tx.hash))]
async fn process_transaction<C, F>(
    tx: Transaction,
    connector: &C,
    matcher: F,
) -> Result<Option<(Transaction, TransactionReceipt)>>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + ReceiptByHash,
    F: Fn(&Transaction) -> bool,
{
    if matcher(&tx) {
        let receipt = connector.receipt_by_hash(tx.hash).await?;

        if !receipt.successful {
            // This can be caused by a failed attempt to complete an action,
            // for example, sending a transaction with low gas.
            tracing::warn!("transaction matched but status was NOT OK");
            return Ok(None);
        }

        tracing::info!("transaction matched");
        return Ok(Some((tx, receipt)));
    }

    Ok(None)
}

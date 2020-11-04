use crate::{
    bitcoin,
    command::CreateTransaction,
    database::{Database, Load},
    ethereum,
    ethereum::to_clarity_address,
    swap::SwapKind,
};
use anyhow::{Context, Result};

pub async fn create_transaction(
    input: CreateTransaction,
    db: Database,
    bitcoin_wallet: bitcoin::Wallet,
    bitcoin_fee: bitcoin::Fee,
    ethereum_wallet: ethereum::Wallet,
    gas_price: ethereum::GasPrice,
) -> Result<String> {
    let swap_id = input.swap_id();
    let swap = db
        .load(swap_id)?
        .with_context(|| format!("unable to find swap with id {}", swap_id))?;

    let hex =
        match (swap, input) {
            (
                SwapKind::HbitHerc20(params),
                CreateTransaction::Redeem {
                    secret,
                    outpoint,
                    fund_amount,
                    ..
                },
            ) => {
                let redeem_address = bitcoin_wallet.new_address().await?;
                let vbyte_rate = bitcoin_fee.vbyte_rate().await?;

                let transaction = params.hbit_params.build_spend_action(
                fund_amount.unwrap_or(params.hbit_params.shared.asset),
                outpoint.context(
                    "HTLC outpoint required but not provided, please provide with --outpoint",
                )?,
                redeem_address,
                |htlc, secret_key| {
                    htlc.unlock_with_secret(&crate::SECP, secret_key, secret.into_raw_secret())
                },
            ).sign(&crate::SECP,
                   vbyte_rate)?;

                ::bitcoin::consensus::encode::serialize_hex(&transaction)
            }
            (SwapKind::HbitHerc20(params), CreateTransaction::Refund { address, .. }) => {
                let action = params.herc20_params.build_refund_action(address.context(
                    "HTLC address required but not provided, please provide with --address",
                )?);
                let gas_price = gas_price.gas_price().await?;
                let to = to_clarity_address(action.to)?;
                let chain_id = action.chain_id;

                let (signed_transaction, _) = ethereum_wallet
                    .sign(
                        |nonce| clarity::Transaction {
                            nonce,
                            gas_price: gas_price.into(),
                            gas_limit: action.gas_limit.into(),
                            to,
                            value: 0u32.into(),
                            data: action.data.unwrap_or_default(),
                            signature: None,
                        },
                        chain_id,
                    )
                    .await?;

                format!(
                    "0x{}",
                    hex::encode(
                        signed_transaction
                            .to_bytes()
                            .context("failed to serialize signed transaction to bytes")?
                    )
                )
            }

            (
                SwapKind::Herc20Hbit(params),
                CreateTransaction::Redeem {
                    secret, address, ..
                },
            ) => {
                let action = params.herc20_params.build_redeem_action(
                    address.context(
                        "HTLC address required but not provided, please provide with --address",
                    )?,
                    secret,
                );

                let gas_price = gas_price.gas_price().await?;
                let to = to_clarity_address(action.to)?;
                let chain_id = action.chain_id;

                let (signed_transaction, _) = ethereum_wallet
                    .sign(
                        |nonce| clarity::Transaction {
                            nonce,
                            gas_price: gas_price.into(),
                            gas_limit: action.gas_limit.into(),
                            to,
                            value: 0u32.into(),
                            data: action.data.unwrap_or_default(),
                            signature: None,
                        },
                        chain_id,
                    )
                    .await?;

                format!(
                    "0x{}",
                    hex::encode(
                        signed_transaction
                            .to_bytes()
                            .context("failed to serialize signed transaction to bytes")?
                    )
                )
            }
            (
                SwapKind::Herc20Hbit(params),
                CreateTransaction::Refund {
                    outpoint,
                    fund_amount,
                    ..
                },
            ) => {
                let refund_address = bitcoin_wallet.new_address().await?;
                let vbyte_rate = bitcoin_fee.vbyte_rate().await?;

                let transaction = params.hbit_params.build_spend_action(
                    fund_amount.unwrap_or(params.hbit_params.shared.asset),
                    outpoint.context(
                        "HTLC outpoint required but not provided, please provide with --outpoint",
                    )?,
                    refund_address,
                    |htlc, secret_key| htlc.unlock_after_timeout(&crate::SECP, secret_key),
                ).sign(
                    &crate::SECP,
                    vbyte_rate)?;

                ::bitcoin::consensus::encode::serialize_hex(&transaction)
            }
        };

    Ok(hex)
}

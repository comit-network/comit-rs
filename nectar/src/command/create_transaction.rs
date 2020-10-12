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

    let hex = match (swap, input) {
        (
            SwapKind::HbitHerc20(params),
            CreateTransaction::Redeem {
                secret, outpoint, ..
            },
        ) => {
            let redeem_address = bitcoin_wallet.new_address().await?;
            let vbyte_rate = bitcoin_fee.vbyte_rate().await?;

            let action = params.hbit_params.shared.build_redeem_action(
                &crate::SECP,
                params.hbit_params.shared.asset, /* TODO: allow the user to override this on the
                                                  * commandline */
                outpoint.context(
                    "HTLC outpoint required but not provided, please provide with --outpoint",
                )?,
                params.hbit_params.transient_sk,
                redeem_address,
                secret,
                vbyte_rate,
            )?;

            hex::encode(::bitcoin::consensus::serialize(&action.transaction))
        }
        (SwapKind::HbitHerc20(params), CreateTransaction::Refund { address, .. }) => {
            let action = params.herc20_params.build_refund_action(address.context(
                "HTLC address required but not provided, please provide with --address",
            )?);
            let gas_price = gas_price.gas_price().await?;
            let to = to_clarity_address(action.to)?;
            let chain_id = action.chain_id;

            ethereum_wallet
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
                .await?
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

            ethereum_wallet
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
                .await?
        }
        (SwapKind::Herc20Hbit(params), CreateTransaction::Refund { outpoint, .. }) => {
            let redeem_address = bitcoin_wallet.new_address().await?;
            let vbyte_rate = bitcoin_fee.vbyte_rate().await?;

            let action = params.hbit_params.shared.build_refund_action(
                &crate::SECP,
                params.hbit_params.shared.asset, /* TODO: allow the user to override this on the
                                                  * commandline */
                outpoint.context(
                    "HTLC outpoint required but not provided, please provide with --outpoint",
                )?,
                params.hbit_params.transient_sk,
                redeem_address,
                vbyte_rate,
            )?;

            hex::encode(::bitcoin::consensus::serialize(&action.transaction))
        }
    };

    Ok(hex)
}

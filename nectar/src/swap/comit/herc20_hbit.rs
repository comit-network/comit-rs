use crate::swap::{
    comit::{EstimateBitcoinFee, EstimateEthereumGasPrice, SwapFailedShouldRefund, Timestamp},
    hbit, herc20,
};
use anyhow::{Context, Result};
use clarity::Uint256;
use comit::{ethereum::ChainId, ledger};
use genawaiter::sync::{Gen, GenBoxed};
use time::OffsetDateTime;

pub enum Action {
    /// The caller should send a Bitcoin transaction with the given parameters
    /// from their wallet.
    BitcoinSendFromWallet {
        to: bitcoin::Address,
        amount: bitcoin::Amount,
        sats_per_kbyte: bitcoin::Amount,
        network: ledger::Bitcoin,
    },
    /// The caller should send an Ethereum transaction with the given parameters
    /// from their wallet.
    EthereumSendFromWallet {
        gas_price: Uint256,
        gas_limit: Uint256,
        to: clarity::Address,
        value: Uint256,
        data: Vec<u8>,
        chain_id: ChainId,
    },
    /// The caller should send this transaction to the network at the given
    /// timestamp.
    BitcoinSendTransaction {
        tx: bitcoin::Transaction,
        /// When the transaction should be sent
        at: Timestamp,
        network: ledger::Bitcoin,
    },
}

pub enum Event {
    Herc20Deployed(herc20::Deployed),
    Herc20Funded(herc20::Funded),
    HbitFunded(hbit::Funded),
    HbitRedeemed(hbit::Redeemed),
    Herc20Redeemed(herc20::Redeemed),
}

pub enum Out {
    Event(Event),
    Action(Action),
}

/// Execute a Herc20<->Hbit swap for Bob.
pub fn herc20_hbit_bob<W>(
    world: W,
    herc20_params: herc20::Params,
    hbit_params: hbit::Params,
    utc_start_of_swap: OffsetDateTime,
) -> GenBoxed<Out, (), anyhow::Result<()>>
where
    W: hbit::WatchForFunded
        + hbit::WatchForRedeemed
        + herc20::WatchForDeployed
        + herc20::WatchForFunded
        + herc20::WatchForRedeemed
        + EstimateBitcoinFee
        + EstimateEthereumGasPrice
        + Send
        + Sync
        + 'static,
{
    Gen::new_boxed(|co| async move {
        tracing::info!("starting swap");

        let swap_result: Result<()> = async {
            let herc20_deployed = world
                .watch_for_deployed(herc20_params.clone(), utc_start_of_swap)
                .await?;

            tracing::info!("alice deployed the herc20 htlc");
            co.yield_(Out::Event(Event::Herc20Deployed(herc20_deployed.clone())))
                .await;

            let herc20_funded = herc20::WatchForFunded::watch_for_funded(
                &world,
                herc20_params.clone(),
                herc20_deployed.clone(),
                utc_start_of_swap,
            )
            .await?;

            tracing::info!("alice funded the herc20 htlc");
            co.yield_(Out::Event(Event::Herc20Funded(herc20_funded.clone())))
                .await;
            co.yield_(Out::Action(Action::BitcoinSendFromWallet {
                to: hbit_params.shared.compute_address(),
                amount: hbit_params.shared.asset,
                sats_per_kbyte: world
                    .estimate_bitcoin_fee(comit::expiries::bitcoin_mine_within_blocks(
                        comit::Network::Main, // TODO: Make this available from the config
                    ))
                    .await,
                network: hbit_params.shared.network,
            }))
            .await;

            let hbit_funded =
                hbit::WatchForFunded::watch_for_funded(&world, &hbit_params, utc_start_of_swap)
                    .await?;

            tracing::info!("we funded the hbit htlc");
            co.yield_(Out::Event(Event::HbitFunded(hbit_funded))).await;

            let hbit_redeemed = hbit::WatchForRedeemed::watch_for_redeemed(
                &world,
                &hbit_params,
                hbit_funded,
                utc_start_of_swap,
            )
            .await
            .context(SwapFailedShouldRefund(hbit_funded))?;

            tracing::info!("alice redeemed the hbit htlc");
            co.yield_(Out::Event(Event::HbitRedeemed(hbit_redeemed.clone())))
                .await;

            let call_contract =
                herc20_params.build_redeem_action(herc20_deployed.location, hbit_redeemed.secret);
            co.yield_(Out::Action(Action::EthereumSendFromWallet {
                gas_price: world
                    .estimate_ethereum_gas_price(comit::expiries::ethereum_mine_within_blocks(
                        comit::Network::Main, // TODO: Make this available from the config
                    ))
                    .await,
                gas_limit: call_contract.gas_limit.into(),
                to: clarity::Address::from_slice(call_contract.to.as_bytes())
                    .context("failed to create private key from byte slice")?,
                value: Uint256::from(0u32),
                data: call_contract.data.unwrap_or_default(),
                chain_id: call_contract.chain_id,
            }))
            .await;

            let herc20_redeemed = herc20::WatchForRedeemed::watch_for_redeemed(
                &world,
                herc20_deployed.clone(),
                utc_start_of_swap,
            )
            .await
            .context(SwapFailedShouldRefund(hbit_funded))?;

            tracing::info!("we redeemed the herc20 htlc");
            co.yield_(Out::Event(Event::Herc20Redeemed(herc20_redeemed.clone())))
                .await;

            Ok(())
        }
        .await;

        if let Err(e) = swap_result {
            if let Some(error) = e.downcast_ref::<SwapFailedShouldRefund<hbit::Funded>>() {
                co.yield_(Out::Action(Action::BitcoinSendTransaction {
                    tx: hbit_params
                        .shared
                        .build_refund_action(
                            &crate::SECP, // TODO: This should be a parameter
                            error.0.asset,
                            error.0.location,
                            hbit_params.transient_sk,
                            hbit_params.final_address,
                            world
                                .estimate_bitcoin_fee(comit::expiries::bitcoin_mine_within_blocks(
                                    comit::Network::Main, /* TODO: Make this available from the
                                                           * config */
                                ))
                                .await,
                        )?
                        .transaction,
                    at: hbit_params.shared.expiry,
                    network: hbit_params.shared.network,
                }))
                .await;
            }

            return Err(e);
        }

        Ok(())
    })
}

use crate::{
    actions::bitcoin::{sign_with_fixed_rate, BroadcastSignedTransaction, SpendOutput},
    http_api::{
        hbit, herc20,
        herc20::build_erc20_htlc,
        protocol::{
            AlphaAbsoluteExpiry, AlphaEvents, AlphaLedger, AlphaParams, BetaAbsoluteExpiry,
            BetaEvents, BetaLedger, BetaParams, Hbit, Herc20, Ledger, LedgerEvents,
        },
        ActionNotFound, BobSwap,
    },
    identity, DeployAction, FundAction, InitAction, RedeemAction, RefundAction, Timestamp,
};
use blockchain_contracts::ethereum::rfc003::{Erc20Htlc, EtherHtlc};
use comit::{
    actions::ethereum,
    asset,
    ethereum::{Bytes, ChainId},
    hbit::build_bitcoin_htlc,
    Never,
};

impl DeployAction
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    type Output = ethereum::DeployContract;

    fn deploy_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    hbit::FinalizedAsRedeemer {
                        state: hbit::State::Funded { .. },
                        ..
                    },
                beta_finalized:
                    herc20::Finalized {
                        state: herc20::State::None,
                        asset: herc20_asset,
                        refund_identity: herc20_refund_identity,
                        redeem_identity: herc20_redeem_identity,
                        expiry: herc20_expiry,
                    },
                secret_hash,
                ..
            } => {
                let htlc = build_erc20_htlc(
                    herc20_asset.clone(),
                    *herc20_redeem_identity,
                    *herc20_refund_identity,
                    *herc20_expiry,
                    *secret_hash,
                );
                let gas_limit = Erc20Htlc::deploy_tx_gas_limit();
                let chain_id = ChainId::regtest();

                Ok(ethereum::DeployContract {
                    data: htlc.into(),
                    amount: asset::Ether::zero(),
                    gas_limit,
                    chain_id,
                })
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl FundAction
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    type Output = ethereum::CallContract;

    fn fund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    hbit::FinalizedAsRedeemer {
                        state: hbit::State::Funded { .. },
                        ..
                    },
                beta_finalized:
                    herc20::Finalized {
                        state: herc20::State::Deployed { htlc_location, .. },
                        asset: herc20_asset,
                        ..
                    },
                ..
            } => {
                let herc20_asset = herc20_asset.clone();
                let to = herc20_asset.token_contract;
                let htlc_address = blockchain_contracts::ethereum::Address((*htlc_location).into());
                let data = Erc20Htlc::transfer_erc20_tx_payload(
                    herc20_asset.quantity.into(),
                    htlc_address,
                );
                let data = Some(Bytes(data));

                let gas_limit = Erc20Htlc::fund_tx_gas_limit();
                let chain_id = ChainId::regtest();
                let min_block_timestamp = None;

                Ok(ethereum::CallContract {
                    to,
                    data,
                    gas_limit,
                    chain_id,
                    min_block_timestamp,
                })
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RedeemAction
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    type Output = BroadcastSignedTransaction;

    fn redeem_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    hbit::FinalizedAsRedeemer {
                        state:
                            hbit::State::Funded {
                                htlc_location,
                                fund_transaction,
                                ..
                            },
                        final_redeem_identity,
                        transient_redeem_identity: transient_redeem_sk,
                        transient_refund_identity,
                        expiry,
                        ..
                    },
                beta_finalized:
                    herc20::Finalized {
                        state: herc20::State::Redeemed { secret, .. },
                        ..
                    },
                secret_hash,
                ..
            } => {
                let network = bitcoin::Network::Regtest;
                let spend_output = {
                    let transient_redeem_identity =
                        identity::Bitcoin::from_secret_key(&*crate::SECP, &transient_redeem_sk);
                    let htlc = build_bitcoin_htlc(
                        transient_redeem_identity,
                        *transient_refund_identity,
                        *expiry,
                        *secret_hash,
                    );

                    let previous_output = *htlc_location;
                    let value = bitcoin::Amount::from_sat(
                        fund_transaction.output[htlc_location.vout as usize].value,
                    );
                    let input_parameters = htlc.unlock_with_secret(
                        &*crate::SECP,
                        *transient_redeem_sk,
                        secret.into_raw_secret(),
                    );

                    SpendOutput::new(previous_output, value, input_parameters, network)
                };

                let primed_transaction =
                    spend_output.spend_to(final_redeem_identity.clone().into());
                let transaction = sign_with_fixed_rate(&*crate::SECP, primed_transaction)?;

                Ok(BroadcastSignedTransaction {
                    transaction,
                    network,
                })
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RefundAction
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    type Output = ethereum::CallContract;

    fn refund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                beta_finalized:
                    herc20::Finalized {
                        state: herc20::State::Funded { htlc_location, .. },
                        expiry: herc20_expiry,
                        ..
                    },
                ..
            } => {
                let to = *htlc_location;
                let data = None;
                let gas_limit = EtherHtlc::refund_tx_gas_limit();
                let chain_id = ChainId::regtest();
                let min_block_timestamp = Some(*herc20_expiry);

                Ok(ethereum::CallContract {
                    to,
                    data,
                    gas_limit,
                    chain_id,
                    min_block_timestamp,
                })
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl AlphaParams
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    type Output = Hbit;
    fn alpha_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl AlphaEvents
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    fn alpha_events(&self) -> Option<LedgerEvents> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    hbit::FinalizedAsRedeemer {
                        state: bitcoin_state,
                        ..
                    },
                ..
            } => Some(From::<hbit::State>::from(bitcoin_state.clone())),
            _ => None,
        }
    }
}

impl BetaParams
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    type Output = Herc20;
    fn beta_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl BetaEvents
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    fn beta_events(&self) -> Option<LedgerEvents> {
        match self {
            BobSwap::Finalized {
                beta_finalized:
                    herc20::Finalized {
                        state: herc20_state,
                        ..
                    },
                ..
            } => Some(From::<herc20::State>::from(herc20_state.clone())),
            _ => None,
        }
    }
}

impl InitAction
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    type Output = Never;
    fn init_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl From<BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>>
    for Hbit
{
    fn from(
        from: BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>,
    ) -> Self {
        match from {
            BobSwap::Created {
                alpha_created: bitcoin_asset,
                ..
            }
            | BobSwap::Finalized {
                alpha_finalized:
                    hbit::FinalizedAsRedeemer {
                        asset: bitcoin_asset,
                        ..
                    },
                ..
            } => Self {
                protocol: "hbit".to_owned(),
                quantity: bitcoin_asset.as_sat().to_string(),
            },
        }
    }
}

impl From<BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>>
    for Herc20
{
    fn from(
        from: BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>,
    ) -> Self {
        match from {
            BobSwap::Created {
                beta_created: herc20_asset,
                ..
            }
            | BobSwap::Finalized {
                beta_finalized:
                    herc20::Finalized {
                        asset: herc20_asset,
                        ..
                    },
                ..
            } => Self {
                protocol: "herc20".to_owned(),
                quantity: herc20_asset.quantity.to_wei_dec(),
                token_contract: herc20_asset.token_contract.to_string(),
            },
        }
    }
}

impl AlphaLedger
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    fn alpha_ledger(&self) -> Ledger {
        Ledger::Bitcoin
    }
}

impl BetaLedger
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    fn beta_ledger(&self) -> Ledger {
        Ledger::Ethereum
    }
}

impl AlphaAbsoluteExpiry
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    fn alpha_absolute_expiry(&self) -> Option<Timestamp> {
        match self {
            BobSwap::Created { .. } => None,
            BobSwap::Finalized {
                alpha_finalized: hbit::FinalizedAsRedeemer { expiry, .. },
                ..
            } => Some(*expiry),
        }
    }
}

impl BetaAbsoluteExpiry
    for BobSwap<asset::Bitcoin, asset::Erc20, hbit::FinalizedAsRedeemer, herc20::Finalized>
{
    fn beta_absolute_expiry(&self) -> Option<Timestamp> {
        match self {
            BobSwap::Created { .. } => None,
            BobSwap::Finalized {
                beta_finalized: herc20::Finalized { expiry, .. },
                ..
            } => Some(*expiry),
        }
    }
}

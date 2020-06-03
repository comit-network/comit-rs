use crate::{
    actions::bitcoin::{sign_with_fixed_rate, BroadcastSignedTransaction, SpendOutput},
    http_api::{
        hbit, herc20,
        herc20::build_erc20_htlc,
        protocol::{
            AlphaAbsoluteExpiry, AlphaEvents, AlphaLedger, AlphaParams, BetaAbsoluteExpiry,
            BetaEvents, BetaLedger, BetaParams, Hbit, Herc20, Ledger, LedgerEvents,
        },
        ActionNotFound, AliceSwap,
    },
    identity, DeployAction, FundAction, InitAction, RedeemAction, RefundAction, Timestamp,
};
use blockchain_contracts::ethereum::rfc003::{Erc20Htlc, EtherHtlc};
use comit::{
    actions::ethereum,
    asset,
    ethereum::{Bytes, ChainId},
    hbit::build_bitcoin_htlc,
    Never, SecretHash,
};

impl DeployAction
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    type Output = ethereum::DeployContract;

    fn deploy_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::Finalized {
                alpha_finalized:
                    herc20::Finalized {
                        state: herc20::State::None,
                        asset: herc20_asset,
                        refund_identity: herc20_refund_identity,
                        redeem_identity: herc20_redeem_identity,
                        expiry: herc20_expiry,
                        ..
                    },
                beta_finalized:
                    hbit::FinalizedAsRedeemer {
                        state: hbit::State::None,
                        ..
                    },
                secret,
                ..
            } => {
                let htlc = build_erc20_htlc(
                    herc20_asset.clone(),
                    *herc20_redeem_identity,
                    *herc20_refund_identity,
                    *herc20_expiry,
                    SecretHash::new(*secret),
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
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    type Output = ethereum::CallContract;

    fn fund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::Finalized {
                alpha_finalized:
                    herc20::Finalized {
                        state: herc20::State::Deployed { htlc_location, .. },
                        asset: herc20_asset,
                        ..
                    },
                beta_finalized:
                    hbit::FinalizedAsRedeemer {
                        state: hbit::State::None,
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
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    type Output = BroadcastSignedTransaction;

    fn redeem_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::Finalized {
                beta_finalized:
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
                secret,
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
                        SecretHash::new(*secret),
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
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    type Output = ethereum::CallContract;

    fn refund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::Finalized {
                alpha_finalized:
                    herc20::Finalized {
                        state: herc20::State::Funded { htlc_location, .. },
                        expiry: herc20_expiry,
                        ..
                    },
                beta_finalized:
                    hbit::FinalizedAsRedeemer {
                        state: hbit::State::Funded { .. },
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
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    type Output = Herc20;
    fn alpha_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl AlphaEvents
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    fn alpha_events(&self) -> Option<LedgerEvents> {
        match self {
            AliceSwap::Created { .. } => None,
            AliceSwap::Finalized {
                alpha_finalized:
                    herc20::Finalized {
                        state: herc20_state,
                        ..
                    },
                ..
            } => Some(From::<herc20::State>::from(herc20_state.clone())),
        }
    }
}

impl BetaParams
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    type Output = Hbit;
    fn beta_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl BetaEvents
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    fn beta_events(&self) -> Option<LedgerEvents> {
        match self {
            AliceSwap::Finalized {
                beta_finalized:
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

impl InitAction
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    type Output = Never;
    fn init_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl From<AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>>
    for Herc20
{
    fn from(
        from: AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>,
    ) -> Self {
        match from {
            AliceSwap::Created {
                alpha_created: herc20_asset,
                ..
            }
            | AliceSwap::Finalized {
                alpha_finalized:
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

impl From<AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>>
    for Hbit
{
    fn from(
        from: AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>,
    ) -> Self {
        match from {
            AliceSwap::Created {
                beta_created: bitcoin_asset,
                ..
            }
            | AliceSwap::Finalized {
                beta_finalized:
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

impl AlphaLedger
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    fn alpha_ledger(&self) -> Ledger {
        Ledger::Ethereum
    }
}

impl BetaLedger
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    fn beta_ledger(&self) -> Ledger {
        Ledger::Bitcoin
    }
}

impl AlphaAbsoluteExpiry
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    fn alpha_absolute_expiry(&self) -> Option<Timestamp> {
        match self {
            AliceSwap::<
                asset::Erc20,
                asset::Bitcoin,
                herc20::Finalized,
                hbit::FinalizedAsRedeemer,
            >::Created {
                ..
            } => None,
            AliceSwap::<
                asset::Erc20,
                asset::Bitcoin,
                herc20::Finalized,
                hbit::FinalizedAsRedeemer,
            >::Finalized {
                alpha_finalized: herc20::Finalized { expiry, .. },
                ..
            } => Some(*expiry),
        }
    }
}

impl BetaAbsoluteExpiry
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, hbit::FinalizedAsRedeemer>
{
    fn beta_absolute_expiry(&self) -> Option<Timestamp> {
        match self {
            AliceSwap::<
                asset::Erc20,
                asset::Bitcoin,
                herc20::Finalized,
                hbit::FinalizedAsRedeemer,
            >::Created {
                ..
            } => None,
            AliceSwap::<
                asset::Erc20,
                asset::Bitcoin,
                herc20::Finalized,
                hbit::FinalizedAsRedeemer,
            >::Finalized {
                beta_finalized: hbit::FinalizedAsRedeemer { expiry, .. },
                ..
            } => Some(*expiry),
        }
    }
}

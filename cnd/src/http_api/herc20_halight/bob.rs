use crate::{
    http_api::{
        halight, herc20,
        protocol::{
            AlphaAbsoluteExpiry, AlphaEvents, AlphaLedger, AlphaParams, BetaAbsoluteExpiry,
            BetaEvents, BetaLedger, BetaParams, BobSwap, Halight, Herc20, Ledger, LedgerEvents,
        },
        ActionNotFound,
    },
    swap_protocols::{
        actions::{ethereum, lnd, lnd::Chain},
        DeployAction, FundAction, InitAction, RedeemAction, RefundAction,
    },
};
use blockchain_contracts::ethereum::rfc003::EtherHtlc;
use comit::{
    asset,
    ethereum::{Bytes, ChainId},
    halight::Settled,
    Never, Timestamp,
};

impl FundAction for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized> {
    type Output = lnd::SendPayment;

    fn fund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    herc20::Finalized {
                        state: herc20::State::Funded { .. },
                        ..
                    },
                beta_finalized:
                    halight::Finalized {
                        state: halight::State::Opened(_),
                        asset: halight_asset,
                        refund_identity: halight_refund_identity,
                        redeem_identity: halight_redeem_identity,
                        cltv_expiry,
                    },
                secret_hash,
            } => {
                let to_public_key = *halight_redeem_identity;
                let amount = *halight_asset;
                let secret_hash = *secret_hash;
                let final_cltv_delta = *cltv_expiry;
                let chain = Chain::Bitcoin;
                let network = bitcoin::Network::Regtest;
                let self_public_key = *halight_refund_identity;

                Ok(lnd::SendPayment {
                    to_public_key,
                    amount,
                    secret_hash,
                    final_cltv_delta,
                    chain,
                    network,
                    self_public_key,
                })
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RedeemAction for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized> {
    type Output = ethereum::CallContract;

    fn redeem_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    herc20::Finalized {
                        state: herc20::State::Funded { htlc_location, .. },
                        ..
                    },
                beta_finalized:
                    halight::Finalized {
                        state: halight::State::Settled(Settled { secret }),
                        ..
                    },
                ..
            } => {
                let to = *htlc_location;
                let data = Some(Bytes::from(secret.into_raw_secret().to_vec()));
                let gas_limit = EtherHtlc::redeem_tx_gas_limit();
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

impl AlphaEvents for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized> {
    fn alpha_events(&self) -> Option<LedgerEvents> {
        match self {
            BobSwap::Created { .. } => None,
            BobSwap::Finalized {
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

impl BetaEvents for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized> {
    fn beta_events(&self) -> Option<LedgerEvents> {
        match self {
            BobSwap::Created { .. } => None,
            BobSwap::Finalized {
                beta_finalized:
                    halight::Finalized {
                        state: halight_state,
                        ..
                    },
                ..
            } => Some(From::<halight::State>::from(*halight_state)),
        }
    }
}

impl AlphaParams for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized> {
    type Output = Herc20;
    fn alpha_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl BetaParams for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized> {
    type Output = Halight;
    fn beta_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl DeployAction for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized> {
    type Output = Never;
    fn deploy_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl InitAction for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized> {
    type Output = Never;
    fn init_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl RefundAction for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized> {
    type Output = Never;
    fn refund_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl From<BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized>> for Herc20 {
    fn from(
        from: BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized>,
    ) -> Self {
        match from {
            BobSwap::Created {
                alpha_created: herc20_asset,
                ..
            }
            | BobSwap::Finalized {
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

impl From<BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized>>
    for Halight
{
    fn from(
        from: BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized>,
    ) -> Self {
        match from {
            BobSwap::Created {
                beta_created: halight_asset,
                ..
            }
            | BobSwap::Finalized {
                beta_finalized:
                    halight::Finalized {
                        asset: halight_asset,
                        ..
                    },
                ..
            } => Self {
                protocol: "halight".to_owned(),
                quantity: halight_asset.as_sat().to_string(),
            },
        }
    }
}

impl AlphaLedger for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized> {
    fn alpha_ledger(&self) -> Ledger {
        Ledger::Ethereum
    }
}

impl BetaLedger for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized> {
    fn beta_ledger(&self) -> Ledger {
        Ledger::Bitcoin
    }
}

impl AlphaAbsoluteExpiry
    for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized>
{
    fn alpha_absolute_expiry(&self) -> Option<Timestamp> {
        match self {
            BobSwap::Created { .. } => None,
            BobSwap::Finalized {
                alpha_finalized: herc20::Finalized { expiry, .. },
                ..
            } => Some(*expiry),
        }
    }
}

impl BetaAbsoluteExpiry
    for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized>
{
    fn beta_absolute_expiry(&self) -> Option<Timestamp> {
        None // No absolute expiry time for halight.
    }
}

use crate::{
    asset,
    ethereum::Bytes,
    http_api::{
        halight,
        halight::INVOICE_EXPIRY_SECS,
        herc20,
        herc20::build_erc20_htlc,
        protocol::{
            AlphaAbsoluteExpiry, AlphaEvents, AlphaLedger, AlphaParams, BetaAbsoluteExpiry,
            BetaEvents, BetaLedger, BetaParams, Halight, Herc20, Ledger, LedgerEvents,
        },
        ActionNotFound, BobSwap,
    },
    swap_protocols::actions::{ethereum, lnd, lnd::Chain},
    DeployAction, FundAction, InitAction, RedeemAction, RefundAction, Timestamp,
};
use blockchain_contracts::ethereum::rfc003::{Erc20Htlc, EtherHtlc};

impl InitAction for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    type Output = lnd::AddHoldInvoice;

    fn init_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    halight::Finalized {
                        asset: halight_asset,
                        network,
                        redeem_identity: halight_redeem_identity,
                        cltv_expiry,
                        state: halight::State::None,
                        ..
                    },
                secret_hash,
                ..
            } => {
                let amount = *halight_asset;
                let secret_hash = *secret_hash;
                let expiry = INVOICE_EXPIRY_SECS;
                let cltv_expiry = *cltv_expiry;
                let chain = Chain::Bitcoin;
                let network = bitcoin::Network::from(*network);
                let self_public_key = *halight_redeem_identity;

                Ok(lnd::AddHoldInvoice {
                    amount,
                    secret_hash,
                    expiry,
                    cltv_expiry,
                    chain,
                    network,
                    self_public_key,
                })
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl DeployAction for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    type Output = ethereum::DeployContract;

    fn deploy_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    halight::Finalized {
                        state: halight::State::Accepted(_),
                        ..
                    },
                beta_finalized:
                    herc20::Finalized {
                        asset: herc20_asset,
                        chain_id,
                        refund_identity: herc20_refund_identity,
                        redeem_identity: herc20_redeem_identity,
                        expiry: herc20_expiry,
                        state: herc20::State::None,
                        ..
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

                Ok(ethereum::DeployContract {
                    data: htlc.into(),
                    amount: asset::Ether::zero(),
                    gas_limit,
                    chain_id: *chain_id,
                })
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl FundAction for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    type Output = ethereum::CallContract;

    fn fund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    halight::Finalized {
                        state: halight::State::Accepted(_),
                        ..
                    },
                beta_finalized:
                    herc20::Finalized {
                        asset: herc20_asset,
                        chain_id,
                        state: herc20::State::Deployed { htlc_location, .. },
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
                let min_block_timestamp = None;

                Ok(ethereum::CallContract {
                    to,
                    data,
                    gas_limit,
                    chain_id: *chain_id,
                    min_block_timestamp,
                })
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RedeemAction for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    type Output = lnd::SettleInvoice;

    fn redeem_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    halight::Finalized {
                        network,
                        redeem_identity: halight_redeem_identity,
                        state: halight::State::Accepted(_),
                        ..
                    },
                beta_finalized:
                    herc20::Finalized {
                        state: herc20::State::Redeemed { secret, .. },
                        ..
                    },
                ..
            } => {
                let secret = *secret;
                let chain = Chain::Bitcoin;
                let network = bitcoin::Network::from(*network);
                let self_public_key = *halight_redeem_identity;

                Ok(lnd::SettleInvoice {
                    secret,
                    chain,
                    network,
                    self_public_key,
                })
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RefundAction for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    type Output = ethereum::CallContract;

    fn refund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    halight::Finalized {
                        state: halight::State::Accepted(_),
                        ..
                    },
                beta_finalized:
                    herc20::Finalized {
                        chain_id,
                        expiry: herc20_expiry,
                        state: herc20::State::Funded { htlc_location, .. },
                        ..
                    },
                ..
            } => {
                let to = *htlc_location;
                let data = None;
                let gas_limit = EtherHtlc::refund_tx_gas_limit();
                let min_block_timestamp = Some(*herc20_expiry);

                Ok(ethereum::CallContract {
                    to,
                    data,
                    gas_limit,
                    chain_id: *chain_id,
                    min_block_timestamp,
                })
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl AlphaEvents for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    fn alpha_events(&self) -> Option<LedgerEvents> {
        match self {
            BobSwap::Created { .. } => None,
            BobSwap::Finalized {
                alpha_finalized:
                    halight::Finalized {
                        state: halight_state,
                        ..
                    },
                ..
            } => Some((*halight_state).into()),
        }
    }
}

impl BetaEvents for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    fn beta_events(&self) -> Option<LedgerEvents> {
        match self {
            BobSwap::Created { .. } => None,
            BobSwap::Finalized {
                beta_finalized:
                    herc20::Finalized {
                        state: herc20_state,
                        ..
                    },
                ..
            } => Some(herc20_state.clone().into()),
        }
    }
}

impl AlphaParams for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    type Output = Halight;
    fn alpha_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl BetaParams for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    type Output = Herc20;
    fn beta_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl From<BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>>
    for Halight
{
    fn from(
        from: BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>,
    ) -> Self {
        match from {
            BobSwap::Created {
                alpha_created: halight_asset,
                ..
            }
            | BobSwap::Finalized {
                alpha_finalized:
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

impl From<BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>> for Herc20 {
    fn from(
        from: BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>,
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

impl AlphaLedger for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    fn alpha_ledger(&self) -> Ledger {
        Ledger::Bitcoin
    }
}

impl BetaLedger for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    fn beta_ledger(&self) -> Ledger {
        Ledger::Ethereum
    }
}

impl AlphaAbsoluteExpiry
    for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>
{
    fn alpha_absolute_expiry(&self) -> Option<Timestamp> {
        None // No absolute expiry time for halight.
    }
}

impl BetaAbsoluteExpiry
    for BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>
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

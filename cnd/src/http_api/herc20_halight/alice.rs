use crate::{
    http_api::{
        halight::HalightFinalized,
        herc20::Herc20Finalized,
        protocol::{
            AlphaEvents, AlphaParams, BetaEvents, BetaParams, Halight, Herc20, LedgerEvents,
        },
        ActionNotFound, AliceSwap,
    },
    swap_protocols::{
        actions::{ethereum, lnd, lnd::Chain},
        halight,
        halight::INVOICE_EXPIRY_SECS,
        herc20,
        herc20::build_erc20_htlc,
        DeployAction, FundAction, InitAction, RedeemAction, RefundAction,
    },
};
use blockchain_contracts::ethereum::rfc003::{Erc20Htlc, EtherHtlc};
use comit::{
    asset,
    ethereum::{Bytes, ChainId},
    SecretHash,
};

impl From<AliceSwap<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized>> for Herc20 {
    fn from(
        from: AliceSwap<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized>,
    ) -> Self {
        match from {
            AliceSwap::<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized>::Created {
                alpha_created: herc20_asset,
                ..
            }
            | AliceSwap::<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized>::Finalized {
                alpha_finalized: Herc20Finalized { herc20_asset, .. },
                ..
            } => Self {
                protocol: "herc20".to_owned(),
                quantity: herc20_asset.quantity.to_wei_dec(),
                token_contract: herc20_asset.token_contract.to_string(),
            },
        }
    }
}

impl From<AliceSwap<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized>> for Halight {
    fn from(
        from: AliceSwap<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized>,
    ) -> Self {
        match from {
            AliceSwap::<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized>::Created {
                beta_created: halight_asset,
                ..
            }
            | AliceSwap::<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized>::Finalized {
                beta_finalized: HalightFinalized { halight_asset, .. },
                ..
            } => Self {
                protocol: "halight".to_owned(),
                quantity: halight_asset.as_sat().to_string(),
            },
        }
    }
}

impl AlphaParams for AliceSwap<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized> {
    type Output = Herc20;
    fn alpha_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl AlphaEvents for AliceSwap<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized> {
    fn alpha_events(&self) -> Option<LedgerEvents> {
        match self {
            AliceSwap::<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized>::Created { .. } => None,
            AliceSwap::<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized>::Finalized {
                alpha_finalized: Herc20Finalized { herc20_state, .. },
                ..
            } => Some(From::<herc20::State>::from(herc20_state.clone())),
        }
    }
}

impl BetaParams for AliceSwap<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized> {
    type Output = Halight;
    fn beta_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl BetaEvents for AliceSwap<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized> {
    fn beta_events(&self) -> Option<LedgerEvents> {
        match self {
            AliceSwap::<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized>::Created { .. } => None,
            AliceSwap::<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized>::Finalized {
                beta_finalized: HalightFinalized { halight_state, .. },
                ..
            } => Some(From::<halight::State>::from(*halight_state)),
        }
    }
}

impl InitAction for AliceSwap<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized> {
    type Output = lnd::AddHoldInvoice;

    fn init_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized>::Finalized {
                beta_finalized:
                    HalightFinalized {
                        halight_state: halight::State::None,
                        halight_asset,
                        halight_redeem_identity,
                        cltv_expiry,
                        ..
                    },
                secret,
                ..
            } => {
                let amount = *halight_asset;
                let secret_hash = SecretHash::new(*secret);
                let expiry = INVOICE_EXPIRY_SECS;
                let cltv_expiry = *cltv_expiry;
                let chain = Chain::Bitcoin;
                let network = bitcoin::Network::Regtest;
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

impl DeployAction for AliceSwap<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized> {
    type Output = ethereum::DeployContract;

    fn deploy_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized>::Finalized {
                beta_finalized:
                    HalightFinalized {
                        halight_state: halight::State::Opened(_),
                        ..
                    },
                alpha_finalized:
                    Herc20Finalized {
                        herc20_asset,
                        herc20_refund_identity,
                        herc20_redeem_identity,
                        herc20_expiry,
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

impl FundAction for AliceSwap<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized> {
    type Output = ethereum::CallContract;

    fn fund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized>::Finalized {
                beta_finalized:
                    HalightFinalized {
                        halight_state: halight::State::Opened(_),
                        ..
                    },
                alpha_finalized:
                    Herc20Finalized {
                        herc20_state: herc20::State::Deployed { htlc_location, .. },
                        herc20_asset,
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

impl RedeemAction for AliceSwap<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized> {
    type Output = lnd::SettleInvoice;

    fn redeem_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized>::Finalized {
                beta_finalized:
                    HalightFinalized {
                        halight_state: halight::State::Accepted(_),
                        halight_redeem_identity,
                        ..
                    },
                secret,
                ..
            } => {
                let secret = *secret;
                let chain = Chain::Bitcoin;
                let network = bitcoin::Network::Regtest;
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

impl RefundAction for AliceSwap<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized> {
    type Output = ethereum::CallContract;

    fn refund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized>::Finalized {
                alpha_finalized:
                    Herc20Finalized {
                        herc20_state: herc20::State::Funded { htlc_location, .. },
                        herc20_expiry,
                        ..
                    },
                beta_finalized:
                    HalightFinalized {
                        halight_state: halight::State::Accepted(_),
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

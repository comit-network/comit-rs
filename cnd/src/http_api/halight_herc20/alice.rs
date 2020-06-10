use crate::{
    asset,
    http_api::{
        halight, herc20,
        protocol::{
            AlphaAbsoluteExpiry, AlphaEvents, AlphaLedger, AlphaParams, BetaAbsoluteExpiry,
            BetaEvents, BetaLedger, BetaParams, Halight, Herc20, Ledger, LedgerEvents,
        },
        ActionNotFound, AliceSwap,
    },
    swap_protocols::actions::{ethereum, lnd},
    DeployAction, FundAction, InitAction, Never, RedeemAction, RefundAction, SecretHash, Timestamp,
};

impl From<AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>>
    for Herc20
{
    fn from(
        from: AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>,
    ) -> Self {
        match from {
            AliceSwap::Created {
                beta_created: herc20_asset,
                ..
            }
            | AliceSwap::Finalized {
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

impl From<AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>>
    for Halight
{
    fn from(
        from: AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>,
    ) -> Self {
        match from {
            AliceSwap::Created {
                alpha_created: halight_asset,
                ..
            }
            | AliceSwap::Finalized {
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

impl BetaParams for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    type Output = Herc20;
    fn beta_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl BetaEvents for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    fn beta_events(&self) -> Option<LedgerEvents> {
        match self {
            AliceSwap::Created { .. } => None,
            AliceSwap::Finalized {
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

impl AlphaParams
    for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>
{
    type Output = Halight;
    fn alpha_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl AlphaEvents
    for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>
{
    fn alpha_events(&self) -> Option<LedgerEvents> {
        match self {
            AliceSwap::Created { .. } => None,
            AliceSwap::Finalized {
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

impl FundAction for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    type Output = lnd::SendPayment;

    fn fund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::Finalized {
                alpha_finalized:
                    halight
                    @
                    halight::Finalized {
                        state: halight::State::Opened(_),
                        ..
                    },
                beta_finalized:
                    herc20::Finalized {
                        state: herc20::State::None,
                        ..
                    },
                secret,
                ..
            } => {
                let secret_hash = SecretHash::new(*secret);
                let fund_action = halight.build_fund_action(secret_hash);
                Ok(fund_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RedeemAction
    for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>
{
    type Output = ethereum::CallContract;

    fn redeem_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::Finalized {
                beta_finalized:
                    herc20
                    @
                    herc20::Finalized {
                        state: herc20::State::Funded { .. },
                        ..
                    },
                secret,
                ..
            } => {
                let redeem_action = herc20.build_redeem_action(*secret)?;
                Ok(redeem_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl InitAction for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    type Output = Never;
    fn init_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl DeployAction
    for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>
{
    type Output = Never;
    fn deploy_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl RefundAction
    for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>
{
    type Output = Never;
    fn refund_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl AlphaLedger
    for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>
{
    fn alpha_ledger(&self) -> Ledger {
        Ledger::Bitcoin
    }
}

impl BetaLedger for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    fn beta_ledger(&self) -> Ledger {
        Ledger::Ethereum
    }
}

impl AlphaAbsoluteExpiry
    for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>
{
    fn alpha_absolute_expiry(&self) -> Option<Timestamp> {
        None // No absolute expiry time for halight.
    }
}

impl BetaAbsoluteExpiry
    for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>
{
    fn beta_absolute_expiry(&self) -> Option<Timestamp> {
        match self {
            AliceSwap::Created { .. } => None,
            AliceSwap::Finalized {
                beta_finalized: herc20::Finalized { expiry, .. },
                ..
            } => Some(*expiry),
        }
    }
}

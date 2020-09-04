use crate::{
    actions::{ethereum, lnd},
    asset,
    http_api::{
        halbit, herc20, ActionNotFound, AliceSwap, AlphaAbsoluteExpiry, AlphaLedger, AlphaProtocol,
        BetaAbsoluteExpiry, BetaLedger, BetaProtocol, Events, Ledger, Protocol, SwapEvent,
    },
    DeployAction, FundAction, InitAction, RedeemAction, RefundAction, SecretHash, Timestamp,
};

impl AlphaProtocol
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized>
{
    fn alpha_protocol(&self) -> Protocol {
        match self {
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
            } => Protocol::herc20_dai(herc20_asset.quantity.clone()),
        }
    }
}

impl Events for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    fn events(&self) -> Vec<SwapEvent> {
        match self {
            AliceSwap::Created { .. } => Vec::new(),
            AliceSwap::Finalized {
                alpha_finalized:
                    herc20::Finalized {
                        state: herc20_state,
                        ..
                    },
                beta_finalized:
                    halbit::Finalized {
                        state: halbit_state,
                        ..
                    },
                ..
            } => {
                let mut events = Vec::new();
                events.extend(Vec::from(herc20_state));
                events.extend(Vec::from(halbit_state));

                events
            }
        }
    }
}

impl BetaProtocol
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized>
{
    fn beta_protocol(&self) -> Protocol {
        match self {
            AliceSwap::Created {
                beta_created: halbit_asset,
                ..
            }
            | AliceSwap::Finalized {
                beta_finalized:
                    halbit::Finalized {
                        asset: halbit_asset,
                        ..
                    },
                ..
            } => Protocol::halbit(*halbit_asset),
        }
    }
}

impl InitAction for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    type Output = lnd::AddHoldInvoice;

    fn init_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::Finalized {
                alpha_finalized:
                    herc20::Finalized {
                        state: herc20::State::None,
                        ..
                    },
                beta_finalized:
                    halbit
                    @
                    halbit::Finalized {
                        state: halbit::State::None,
                        ..
                    },
                secret,
                ..
            } => {
                let secret_hash = SecretHash::new(*secret);
                let init_action = halbit.build_init_action(secret_hash);
                Ok(init_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl DeployAction
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized>
{
    type Output = ethereum::DeployContract;

    fn deploy_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::Finalized {
                alpha_finalized:
                    herc20
                    @
                    herc20::Finalized {
                        state: herc20::State::None,
                        ..
                    },
                beta_finalized:
                    halbit::Finalized {
                        state: halbit::State::Opened(_),
                        ..
                    },
                secret,
                ..
            } => {
                let secret_hash = SecretHash::new(*secret);
                let deploy_action = herc20.build_deploy_action(secret_hash);
                Ok(deploy_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl FundAction for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    type Output = ethereum::CallContract;

    fn fund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::Finalized {
                alpha_finalized:
                    herc20
                    @
                    herc20::Finalized {
                        state: herc20::State::Deployed { .. },
                        ..
                    },
                beta_finalized:
                    halbit::Finalized {
                        state: halbit::State::Opened(_),
                        ..
                    },
                secret,
                ..
            } => {
                let secret_hash = SecretHash::new(*secret);
                let fund_action = herc20.build_fund_action(secret_hash)?;
                Ok(fund_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RedeemAction
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized>
{
    type Output = lnd::SettleInvoice;

    fn redeem_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::Finalized {
                beta_finalized:
                    halbit
                    @
                    halbit::Finalized {
                        state: halbit::State::Accepted(_),
                        ..
                    },
                secret,
                ..
            } => {
                let redeem_action = halbit.build_redeem_action(*secret);
                Ok(redeem_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RefundAction
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized>
{
    type Output = ethereum::CallContract;

    fn refund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::Finalized {
                alpha_finalized:
                    herc20
                    @
                    herc20::Finalized {
                        state: herc20::State::Funded { .. },
                        ..
                    },
                secret,
                ..
            } => {
                let secret_hash = SecretHash::new(*secret);
                let refund_action = herc20.build_refund_action(secret_hash)?;
                Ok(refund_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl AlphaLedger for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    fn alpha_ledger(&self) -> Ledger {
        Ledger::Ethereum
    }
}

impl BetaLedger for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    fn beta_ledger(&self) -> Ledger {
        Ledger::Bitcoin
    }
}

impl AlphaAbsoluteExpiry
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized>
{
    fn alpha_absolute_expiry(&self) -> Option<Timestamp> {
        match self {
            AliceSwap::Created { .. } => None,
            AliceSwap::Finalized {
                alpha_finalized: herc20::Finalized { expiry, .. },
                ..
            } => Some(*expiry),
        }
    }
}

impl BetaAbsoluteExpiry
    for AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized>
{
    fn beta_absolute_expiry(&self) -> Option<Timestamp> {
        None // No absolute expiry time for halbit.
    }
}

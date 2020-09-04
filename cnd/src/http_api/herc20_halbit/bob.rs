use crate::{
    actions::{ethereum, lnd},
    asset,
    halbit::Settled,
    http_api::{
        halbit, herc20, ActionNotFound, AlphaAbsoluteExpiry, AlphaLedger, AlphaProtocol,
        BetaAbsoluteExpiry, BetaLedger, BetaProtocol, BobSwap, Events, Ledger, Protocol, SwapEvent,
    },
    DeployAction, FundAction, InitAction, Never, RedeemAction, RefundAction, Timestamp,
};

impl FundAction for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
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
                    halbit
                    @
                    halbit::Finalized {
                        state: halbit::State::Opened(_),
                        ..
                    },
                secret_hash,
            } => {
                let fund_action = halbit.build_fund_action(*secret_hash);
                Ok(fund_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RedeemAction for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    type Output = ethereum::CallContract;

    fn redeem_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized: herc20 @ herc20::Finalized { .. },
                beta_finalized:
                    halbit::Finalized {
                        state: halbit::State::Settled(Settled { secret }),
                        ..
                    },
                ..
            } => {
                let redeem_action = herc20.build_redeem_action(*secret)?;
                Ok(redeem_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl Events for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    fn events(&self) -> Vec<SwapEvent> {
        match self {
            BobSwap::Created { .. } => Vec::new(),
            BobSwap::Finalized {
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

impl AlphaProtocol for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    fn alpha_protocol(&self) -> Protocol {
        match self {
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
            } => Protocol::herc20_dai(herc20_asset.quantity.clone()),
        }
    }
}

impl BetaProtocol for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    fn beta_protocol(&self) -> Protocol {
        match self {
            BobSwap::Created {
                beta_created: halbit_asset,
                ..
            }
            | BobSwap::Finalized {
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

impl DeployAction for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    type Output = Never;
    fn deploy_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl InitAction for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    type Output = Never;
    fn init_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl RefundAction for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    type Output = Never;
    fn refund_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl AlphaLedger for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    fn alpha_ledger(&self) -> Ledger {
        Ledger::Ethereum
    }
}

impl BetaLedger for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized> {
    fn beta_ledger(&self) -> Ledger {
        Ledger::Bitcoin
    }
}

impl AlphaAbsoluteExpiry
    for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized>
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
    for BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized>
{
    fn beta_absolute_expiry(&self) -> Option<Timestamp> {
        None // No absolute expiry time for halbit.
    }
}

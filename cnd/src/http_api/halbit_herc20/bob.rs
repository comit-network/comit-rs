use crate::{
    actions::{ethereum, lnd},
    asset,
    http_api::{
        halbit, herc20, ActionNotFound, AlphaAbsoluteExpiry, AlphaLedger, AlphaProtocol,
        BetaAbsoluteExpiry, BetaLedger, BetaProtocol, BobSwap, Events, Ledger, Protocol, SwapEvent,
    },
    DeployAction, FundAction, InitAction, RedeemAction, RefundAction, Timestamp,
};

impl InitAction for BobSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized> {
    type Output = lnd::AddHoldInvoice;

    fn init_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    halbit
                    @
                    halbit::Finalized {
                        state: halbit::State::None,
                        ..
                    },
                secret_hash,
                ..
            } => {
                let init_action = halbit.build_init_action(*secret_hash);
                Ok(init_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl DeployAction for BobSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized> {
    type Output = ethereum::DeployContract;

    fn deploy_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    halbit::Finalized {
                        state: halbit::State::Accepted(_),
                        ..
                    },
                beta_finalized:
                    herc20
                    @
                    herc20::Finalized {
                        state: herc20::State::None,
                        ..
                    },
                secret_hash,
                ..
            } => {
                let deploy_action = herc20.build_deploy_action(*secret_hash);
                Ok(deploy_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl FundAction for BobSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized> {
    type Output = ethereum::CallContract;

    fn fund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    halbit::Finalized {
                        state: halbit::State::Accepted(_),
                        ..
                    },
                beta_finalized:
                    herc20
                    @
                    herc20::Finalized {
                        state: herc20::State::Deployed { .. },
                        ..
                    },
                secret_hash,
                ..
            } => {
                let fund_action = herc20.build_fund_action(*secret_hash)?;
                Ok(fund_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RedeemAction for BobSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized> {
    type Output = lnd::SettleInvoice;

    fn redeem_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    halbit
                    @
                    halbit::Finalized {
                        state: halbit::State::Accepted(_),
                        ..
                    },
                beta_finalized:
                    herc20::Finalized {
                        state: herc20::State::Redeemed { secret, .. },
                        ..
                    },
                ..
            } => {
                let redeem_action = halbit.build_redeem_action(*secret);
                Ok(redeem_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RefundAction for BobSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized> {
    type Output = ethereum::CallContract;

    fn refund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            BobSwap::Finalized {
                alpha_finalized:
                    halbit::Finalized {
                        state: halbit::State::Accepted(_),
                        ..
                    },
                beta_finalized:
                    herc20
                    @
                    herc20::Finalized {
                        state: herc20::State::Funded { .. },
                        ..
                    },
                secret_hash,
                ..
            } => {
                let refund_action = herc20.build_refund_action(*secret_hash)?;
                Ok(refund_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl Events for BobSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized> {
    fn events(&self) -> Vec<SwapEvent> {
        match self {
            BobSwap::Created { .. } => Vec::new(),
            BobSwap::Finalized {
                alpha_finalized:
                    halbit::Finalized {
                        state: halbit_state,
                        ..
                    },
                beta_finalized:
                    herc20::Finalized {
                        state: herc20_state,
                        ..
                    },
                ..
            } => {
                let mut events = Vec::new();
                events.extend(Vec::from(halbit_state));
                events.extend(Vec::from(herc20_state));

                events
            }
        }
    }
}

impl AlphaProtocol for BobSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized> {
    fn alpha_protocol(&self) -> Protocol {
        match self {
            BobSwap::Created {
                alpha_created: halbit_asset,
                ..
            }
            | BobSwap::Finalized {
                alpha_finalized:
                    halbit::Finalized {
                        asset: halbit_asset,
                        ..
                    },
                ..
            } => Protocol::halbit(*halbit_asset),
        }
    }
}

impl BetaProtocol for BobSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized> {
    fn beta_protocol(&self) -> Protocol {
        match self {
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
            } => Protocol::herc20_dai(herc20_asset.quantity.clone()),
        }
    }
}

impl AlphaLedger for BobSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized> {
    fn alpha_ledger(&self) -> Ledger {
        Ledger::Bitcoin
    }
}

impl BetaLedger for BobSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized> {
    fn beta_ledger(&self) -> Ledger {
        Ledger::Ethereum
    }
}

impl AlphaAbsoluteExpiry
    for BobSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized>
{
    fn alpha_absolute_expiry(&self) -> Option<Timestamp> {
        None // No absolute expiry time for halbit.
    }
}

impl BetaAbsoluteExpiry
    for BobSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized>
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

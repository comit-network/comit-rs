use crate::{
    actions::{ethereum, lnd},
    asset,
    http_api::{
        halbit, herc20, ActionNotFound, AliceSwap, AlphaAbsoluteExpiry, AlphaLedger, AlphaProtocol,
        BetaAbsoluteExpiry, BetaLedger, BetaProtocol, Events, Ledger, Protocol, SwapEvent,
    },
    DeployAction, FundAction, InitAction, Never, RedeemAction, RefundAction, SecretHash, Timestamp,
};

impl BetaProtocol
    for AliceSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized>
{
    fn beta_protocol(&self) -> Protocol {
        match self {
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
            } => Protocol::herc20_dai(herc20_asset.quantity.clone()),
        }
    }
}

impl AlphaProtocol
    for AliceSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized>
{
    fn alpha_protocol(&self) -> Protocol {
        match self {
            AliceSwap::Created {
                alpha_created: halbit_asset,
                ..
            }
            | AliceSwap::Finalized {
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

impl Events for AliceSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized> {
    fn events(&self) -> Vec<SwapEvent> {
        match self {
            AliceSwap::Created { .. } => Vec::new(),
            AliceSwap::Finalized {
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

impl FundAction for AliceSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized> {
    type Output = lnd::SendPayment;

    fn fund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::Finalized {
                alpha_finalized:
                    halbit
                    @
                    halbit::Finalized {
                        state: halbit::State::Opened(_),
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
                let fund_action = halbit.build_fund_action(secret_hash);
                Ok(fund_action)
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RedeemAction
    for AliceSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized>
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

impl InitAction for AliceSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized> {
    type Output = Never;
    fn init_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl DeployAction
    for AliceSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized>
{
    type Output = Never;
    fn deploy_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl RefundAction
    for AliceSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized>
{
    type Output = Never;
    fn refund_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl AlphaLedger for AliceSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized> {
    fn alpha_ledger(&self) -> Ledger {
        Ledger::Bitcoin
    }
}

impl BetaLedger for AliceSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized> {
    fn beta_ledger(&self) -> Ledger {
        Ledger::Ethereum
    }
}

impl AlphaAbsoluteExpiry
    for AliceSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized>
{
    fn alpha_absolute_expiry(&self) -> Option<Timestamp> {
        None // No absolute expiry time for halbit.
    }
}

impl BetaAbsoluteExpiry
    for AliceSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized>
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

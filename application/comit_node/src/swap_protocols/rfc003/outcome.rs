// TODO: Add a `Declined` outcome
#[derive(Debug, PartialEq, Clone)]
pub enum SwapOutcome {
    Rejected,
    AlphaRefunded,
    BothRefunded,
    BothRedeemed,
    AlphaRedeemedBetaRefunded,
    AlphaRefundedBetaRedeemed,
}

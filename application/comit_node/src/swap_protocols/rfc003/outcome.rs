#[derive(Debug, PartialEq, Clone)]
pub enum SwapOutcome {
    Rejected,
    SourceRefunded,
    BothRefunded,
    BothRedeemed,
    SourceRedeemedTargetRefunded,
    SourceRefundedTargetRedeemed,
}

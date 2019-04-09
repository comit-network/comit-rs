use crate::swap_protocols::rfc003::ledger::Ledger;
use strum_macros::EnumDiscriminants;

#[derive(Clone, Debug, PartialEq, EnumDiscriminants)]
#[strum_discriminants(
    name(HtlcState),
    derive(Serialize, rename_all = "SCREAMING_SNAKE_CASE")
)]
pub enum LedgerState<L: Ledger> {
    NotDeployed,
    Deployed {
        htlc_location: L::HtlcLocation,
        deploy_transaction: L::Transaction,
    },
    Funded {
        htlc_location: L::HtlcLocation,
        deploy_transaction: L::Transaction,
        fund_transaction: L::Transaction,
    },
    Redeemed {
        htlc_location: L::HtlcLocation,
        deploy_transaction: L::Transaction,
        fund_transaction: L::Transaction,
        redeem_transaction: L::Transaction,
    },
    Refunded {
        htlc_location: L::HtlcLocation,
        deploy_transaction: L::Transaction,
        fund_transaction: L::Transaction,
        refund_transaction: L::Transaction,
    },
}

use crate::swap_protocols::rfc003::ledger::Ledger;

#[derive(Clone, Debug, PartialEq)]
pub enum LedgerState<L: Ledger> {
    NotDeployed,
    Deployed {
        htlc_location: L::HtlcLocation,
        // deploy_transaction: L::Transaction,
    },
    Funded {
        htlc_location: L::HtlcLocation,
        /* deploy_transaction: L::Transaction
         * fund_transaction: L::Transaction, */
    },
    Redeemed {
        htlc_location: L::HtlcLocation,
        // deploy_transaction: L::Transaction,
        // fund_transaction: L::Transaction,
        redeem_transaction: L::Transaction,
    },
    Refunded {
        htlc_location: L::HtlcLocation,
        // deploy_transaction: L::Transaction,
        // fund_transaction: L::Transaction,
        refund_transaction: L::Transaction,
    },
}

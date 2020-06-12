//! Defines errors for the database module.
//!
//! These are single-error structs in contrast to enums. We use `anyhow::Result`
//! in almost all of our APIs. There is little value in defining enums that
//! describe a range of errors if you have to downcast from an anyhow::Error
//! anyway.

use crate::LocalSwapId;

#[derive(thiserror::Error, Debug, Clone, Copy)]
#[error("no secret hash found in database for swap {0}")]
pub struct NoSecretHash(pub LocalSwapId);

#[derive(thiserror::Error, Debug, Clone, Copy)]
#[error("no halbit refund identity found in database for swap {0}")]
pub struct NoHalbitRefundIdentity(pub LocalSwapId);

#[derive(thiserror::Error, Debug, Clone, Copy)]
#[error("no halbit redeem identity found in database for swap {0}")]
pub struct NoHalbitRedeemIdentity(pub LocalSwapId);

#[derive(thiserror::Error, Debug, Clone, Copy)]
#[error("no herc20 refund identity found in database for swap {0}")]
pub struct NoHerc20RefundIdentity(pub LocalSwapId);

#[derive(thiserror::Error, Debug, Clone, Copy)]
#[error("no herc20 redeem identity found in database for swap {0}")]
pub struct NoHerc20RedeemIdentity(pub LocalSwapId);

#[derive(thiserror::Error, Debug, Clone, Copy)]
#[error("no hbit refund identity found in database for swap {0}")]
pub struct NoHbitRefundIdentity(pub LocalSwapId);

#[derive(thiserror::Error, Debug, Clone, Copy)]
#[error("no hbit redeem identity found in database for swap {0}")]
pub struct NoHbitRedeemIdentity(pub LocalSwapId);

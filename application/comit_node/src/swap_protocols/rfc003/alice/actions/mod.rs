use crate::swap_protocols::{
    asset::Asset,
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        actions::Action,
        bitcoin,
        ethereum::{self, EtherHtlc},
        secret_source::SecretSource,
        state_machine::HtlcParams,
        Ledger, Secret,
    },
};
use bitcoin_support::{BitcoinQuantity, OutPoint};
use bitcoin_witness::PrimedInput;
use ethereum_support::{Address as EthereumAddress, Bytes, EtherQuantity};
mod btc_erc20;
mod btc_eth;
mod erc20_btc;

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum ActionKind<Deploy, Fund, Redeem, Refund> {
    Deploy(Deploy),
    Fund(Fund),
    Redeem(Redeem),
    Refund(Refund),
}

impl<Deploy, Fund, Redeem, Refund> ActionKind<Deploy, Fund, Redeem, Refund> {
    fn into_action(self) -> Action<ActionKind<Deploy, Fund, Redeem, Refund>> {
        Action {
            inner: self,
            invalid_until: None,
        }
    }
}

pub trait CreateActions<AL: Ledger, AA: Asset> {
    type FundActionOutput;
    type RefundActionOutput;
    type RedeemActionOutput;

    fn fund_action(htlc_params: HtlcParams<AL, AA>) -> Self::FundActionOutput;

    fn refund_action(
        alpha_asset: AA,
        htlc_params: HtlcParams<AL, AA>,
        alpha_htlc_location: AL::HtlcLocation,
        secret_source: &dyn SecretSource,
    ) -> Self::RefundActionOutput;

    fn redeem_action(
        alpha_asset: AA,
        htlc_params: HtlcParams<AL, AA>,
        alpha_htlc_location: AL::HtlcLocation,
        secret_source: &dyn SecretSource,
        secret: Secret,
    ) -> Self::RedeemActionOutput;
}

impl CreateActions<Bitcoin, BitcoinQuantity> for (Bitcoin, BitcoinQuantity) {
    type FundActionOutput = bitcoin::SendToAddress;
    type RefundActionOutput = bitcoin::SpendOutput;
    type RedeemActionOutput = bitcoin::SpendOutput;

    fn fund_action(htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>) -> Self::FundActionOutput {
        let to = htlc_params.compute_address();

        bitcoin::SendToAddress {
            to,
            amount: htlc_params.asset,
            network: htlc_params.ledger.network,
        }
    }

    fn refund_action(
        alpha_asset: BitcoinQuantity,
        htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>,
        alpha_htlc_location: OutPoint,
        secret_source: &dyn SecretSource,
    ) -> Self::RefundActionOutput {
        let htlc = bitcoin::Htlc::from(htlc_params.clone());

        bitcoin::SpendOutput {
            output: PrimedInput::new(
                alpha_htlc_location,
                alpha_asset,
                htlc.unlock_after_timeout(secret_source.secp256k1_refund()),
            ),
            network: htlc_params.ledger.network,
        }
    }

    fn redeem_action(
        asset: BitcoinQuantity,
        htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>,
        htlc_location: OutPoint,
        secret_source: &dyn SecretSource,
        secret: Secret,
    ) -> Self::RedeemActionOutput {
        let htlc = bitcoin::Htlc::from(htlc_params.clone());

        bitcoin::SpendOutput {
            output: PrimedInput::new(
                htlc_location,
                asset,
                htlc.unlock_with_secret(secret_source.secp256k1_redeem(), &secret),
            ),
            network: htlc_params.ledger.network,
        }
    }
}

impl CreateActions<Ethereum, EtherQuantity> for (Ethereum, EtherQuantity) {
    type FundActionOutput = ethereum::ContractDeploy;
    type RefundActionOutput = ethereum::SendTransaction;
    type RedeemActionOutput = ethereum::SendTransaction;

    fn fund_action(htlc_params: HtlcParams<Ethereum, EtherQuantity>) -> Self::FundActionOutput {
        htlc_params.into()
    }

    fn refund_action(
        _alpha_asset: EtherQuantity,
        htlc_params: HtlcParams<Ethereum, EtherQuantity>,
        alpha_htlc_location: EthereumAddress,
        _secret_source: &dyn SecretSource,
    ) -> Self::RefundActionOutput {
        let data = Bytes::default();
        let gas_limit = EtherHtlc::tx_gas_limit();

        ethereum::SendTransaction {
            to: alpha_htlc_location,
            data,
            gas_limit,
            amount: EtherQuantity::zero(),
            network: htlc_params.ledger.network,
        }
    }

    fn redeem_action(
        _alpha_asset: EtherQuantity,
        htlc_params: HtlcParams<Ethereum, EtherQuantity>,
        alpha_htlc_location: EthereumAddress,
        _secret_source: &dyn SecretSource,
        secret: Secret,
    ) -> Self::RedeemActionOutput {
        let data = Bytes::from(secret.raw_secret().to_vec());
        let gas_limit = EtherHtlc::tx_gas_limit();

        ethereum::SendTransaction {
            to: alpha_htlc_location,
            data,
            gas_limit,
            amount: EtherQuantity::zero(),
            network: htlc_params.ledger.network,
        }
    }
}

//////// Old stuff
// trait DeployAction<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
//    type DeployActionReturn;
//    fn deploy_action(
//        request: &rfc003::messages::Request<AL, BL, AA, BA>,
//        response: &rfc003::messages::AcceptResponseBody<AL, BL>,
//    ) -> Self::DeployActionReturn;
//}

// trait FundAction<AL: Ledger, AA: Asset> {
// type Fund;
// fn fund_action(htlc_params: HtlcParams<AL, AA>, amount: AA, network:
// AL::Network) -> Self::Fund;
// }
//
// trait RefundAction<AL: Ledger, AA: Asset> {
// type Output;
//
// fn refund_action(
// alpha_asset: AA,
// htlc_params: HtlcParams<AL, AA>,
// network: AL::Network,
// alpha_htlc_location: AL::HtlcLocation,
// secret_source: &dyn SecretSource,
// ) -> Self::Output;
// }
//
// trait RedeemAction<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
// type HTLCLocation;
// type Output;
//
// fn redeem_action(
// request: &rfc003::messages::Request<AL, BL, AA, BA>,
// beta_htlc_location: Self::HTLCLocation,
// secret: Secret,
// ) -> Self::Output;
// }
//
// impl<BL: Ledger, BA: Asset> DeployAction<Ethereum, BL, Erc20Token, BA> for
// (Ethereum, Erc20Token) {    type DeployActionReturn =
// ethereum::ContractDeploy;
//
//    fn deploy_action(
//        request: &rfc003::messages::Request<Ethereum, BL, Erc20Token, BA>,
//        response: &rfc003::messages::AcceptResponseBody<Ethereum, BL>,
//    ) -> Self::DeployActionReturn {
//        HtlcParams::new_alpha_params(request, response).into()
//    }
// }
//
// impl FundAction<Bitcoin, BitcoinQuantity> for (Bitcoin, BitcoinQuantity) {
// type Fund = bitcoin::SendToAddress;
//
// fn fund_action(
// htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>,
// alpha_asset: BitcoinQuantity,
// network: BitcoinNetwork,
// ) -> Self::Fund {
// let to = htlc_params.compute_address();
//
// bitcoin::SendToAddress {
// to,
// amount: alpha_asset,
// network,
// }
// }
// }
//
// impl<BL: Ledger, BA: Asset> FundAction<Ethereum, BL, EtherQuantity, BA>
//    for (Ethereum, EtherQuantity)
// {
//    type Fund = ethereum::SendTransaction;
//
//    fn fund_action(
//        request: &rfc003::messages::Request<Ethereum, BL, EtherQuantity, BA>,
//        response: &rfc003::messages::AcceptResponseBody<Ethereum, BL>,
//    ) -> Self::Fund {
//        let to = request.alpha_asset.token_contract;
//        let htlc = Erc20Htlc::from(HtlcParams::new_alpha_params(request,
// response));        let gas_limit = Erc20Htlc::fund_tx_gas_limit();
//        let network = request.alpha_ledger.network;
//
//        ethereum::SendTransaction {
//            to,
//            data: htlc.funding_tx_payload(alpha_htlc_location),
//            gas_limit,
//            amount: EtherQuantity::zero(),
//            network,
//        }
//    }
// }
//
// impl RefundAction<Bitcoin, BitcoinQuantity> for (Bitcoin, BitcoinQuantity) {
// type Output = bitcoin::SpendOutput;
//
// fn refund_action(
// alpha_asset: BitcoinQuantity,
// htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>,
// network: BitcoinNetwork,
// alpha_htlc_location: OutPoint,
// secret_source: &dyn SecretSource,
// ) -> Self::Output {
// let htlc = bitcoin::Htlc::from(htlc_params);
//
// bitcoin::SpendOutput {
// output: PrimedInput::new(
// alpha_htlc_location,
// alpha_asset,
// htlc.unlock_after_timeout(secret_source.secp256k1_refund()),
// ),
// network,
// }
// }
// }
//
// impl<AL: Ledger, AA: Asset> RedeemAction<AL, Ethereum, AA, EtherQuantity>
// for (Ethereum, EtherQuantity)
// {
// type HTLCLocation = ethereum_support::Address;
// type Output = ethereum::SendTransaction;
//
// fn redeem_action(
// request: &rfc003::messages::Request<AL, Ethereum, AA, EtherQuantity>,
// beta_htlc_location: Self::HTLCLocation,
// secret: Secret,
// ) -> Self::Output {
// let data = Bytes::from(secret.raw_secret().to_vec());
// let gas_limit = EtherHtlc::tx_gas_limit();
// let network = request.beta_ledger.network;
//
// ethereum::SendTransaction {
// to: beta_htlc_location,
// data,
// gas_limit,
// amount: EtherQuantity::zero(),
// network,
// }
// }
// }
//
// impl<AL: Ledger, AA: Asset> RedeemAction<AL, Ethereum, AA, Erc20Token> for
// (Ethereum, Erc20Token) { type HTLCLocation = ethereum_support::Address;
// type Output = ethereum::SendTransaction;
//
// fn redeem_action(
// request: &rfc003::messages::Request<AL, Ethereum, AA, Erc20Token>,
// beta_htlc_location: Self::HTLCLocation,
// secret: Secret,
// ) -> Self::Output {
// let data = Bytes::from(secret.raw_secret().to_vec());
// let gas_limit = Erc20Htlc::tx_gas_limit();
// let network = request.beta_ledger.network;
//
// ethereum::SendTransaction {
// to: beta_htlc_location,
// data,
// gas_limit,
// amount: EtherQuantity::zero(),
// network,
// }
// }
// }

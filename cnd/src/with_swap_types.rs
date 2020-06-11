#[macro_export]
macro_rules! within_swap_context {
    ($swap_context:expr, $fn:expr) => {{
        use crate::{
            asset,
            http_api::{halbit, hbit, herc20, AliceSwap, BobSwap},
            storage::SwapContext,
            Protocol, Role,
        };

        let swap_context: SwapContext = $swap_context;

        match swap_context {
            SwapContext {
                alpha: Protocol::Herc20,
                beta: Protocol::Halbit,
                role: Role::Alice,
                ..
            } => {
                #[allow(dead_code)]
                type ActorSwap =
                    AliceSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized>;
                #[allow(dead_code)]
                type AlphaParams = herc20::Params;
                #[allow(dead_code)]
                type BetaParams = halbit::Params;
                $fn
            }
            SwapContext {
                alpha: Protocol::Herc20,
                beta: Protocol::Halbit,
                role: Role::Bob,
                ..
            } => {
                #[allow(dead_code)]
                type ActorSwap =
                    BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halbit::Finalized>;
                #[allow(dead_code)]
                type AlphaParams = herc20::Params;
                #[allow(dead_code)]
                type BetaParams = halbit::Params;
                $fn
            }
            SwapContext {
                alpha: Protocol::Halbit,
                beta: Protocol::Herc20,
                role: Role::Alice,
                ..
            } => {
                #[allow(dead_code)]
                type ActorSwap =
                    AliceSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized>;
                #[allow(dead_code)]
                type AlphaParams = halbit::Params;
                #[allow(dead_code)]
                type BetaParams = herc20::Params;
                $fn
            }
            SwapContext {
                alpha: Protocol::Halbit,
                beta: Protocol::Herc20,
                role: Role::Bob,
                ..
            } => {
                #[allow(dead_code)]
                type ActorSwap =
                    BobSwap<asset::Bitcoin, asset::Erc20, halbit::Finalized, herc20::Finalized>;
                #[allow(dead_code)]
                type AlphaParams = halbit::Params;
                #[allow(dead_code)]
                type BetaParams = herc20::Params;
                $fn
            }
            SwapContext {
                alpha: Protocol::Herc20,
                beta: Protocol::Hbit,
                role: Role::Alice,
                ..
            } => {
                #[allow(dead_code)]
                type ActorSwap = AliceSwap<
                    asset::Erc20,
                    asset::Bitcoin,
                    herc20::Finalized,
                    hbit::FinalizedAsRedeemer,
                >;
                #[allow(dead_code)]
                type AlphaParams = herc20::Params;
                #[allow(dead_code)]
                type BetaParams = hbit::Params;

                $fn
            }
            SwapContext {
                alpha: Protocol::Herc20,
                beta: Protocol::Hbit,
                role: Role::Bob,
                ..
            } => {
                #[allow(dead_code)]
                type ActorSwap = BobSwap<
                    asset::Erc20,
                    asset::Bitcoin,
                    herc20::Finalized,
                    hbit::FinalizedAsFunder,
                >;
                #[allow(dead_code)]
                type AlphaParams = herc20::Params;
                #[allow(dead_code)]
                type BetaParams = hbit::Params;

                $fn
            }
            SwapContext {
                alpha: Protocol::Hbit,
                beta: Protocol::Herc20,
                role: Role::Alice,
                ..
            } => {
                #[allow(dead_code)]
                type ActorSwap = AliceSwap<
                    asset::Bitcoin,
                    asset::Erc20,
                    hbit::FinalizedAsFunder,
                    herc20::Finalized,
                >;
                #[allow(dead_code)]
                type AlphaParams = hbit::Params;
                #[allow(dead_code)]
                type BetaParams = herc20::Params;

                $fn
            }
            SwapContext {
                alpha: Protocol::Hbit,
                beta: Protocol::Herc20,
                role: Role::Bob,
                ..
            } => {
                #[allow(dead_code)]
                type ActorSwap = BobSwap<
                    asset::Bitcoin,
                    asset::Erc20,
                    hbit::FinalizedAsRedeemer,
                    herc20::Finalized,
                >;
                #[allow(dead_code)]
                type AlphaParams = hbit::Params;
                #[allow(dead_code)]
                type BetaParams = herc20::Params;

                $fn
            }
            _ => unimplemented!("protocol combination not supported: {:?}", swap_context),
        }
    }};
}

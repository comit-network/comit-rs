#[macro_export]
macro_rules! within_swap_context {
    ($swap_context:expr, $fn:expr) => {{
        use crate::{storage::SwapContext, LockProtocol, Role};
        use comit::{herc20, swap::hbit};

        let swap_context: SwapContext = $swap_context;

        match swap_context {
            SwapContext {
                alpha: LockProtocol::Herc20,
                beta: LockProtocol::Hbit,
                role: Role::Alice,
                ..
            } => {
                #[allow(dead_code)]
                type AlphaParams = herc20::Params;
                #[allow(dead_code)]
                type BetaParams = hbit::Params;

                $fn
            }
            SwapContext {
                alpha: LockProtocol::Herc20,
                beta: LockProtocol::Hbit,
                role: Role::Bob,
                ..
            } => {
                #[allow(dead_code)]
                type AlphaParams = herc20::Params;
                #[allow(dead_code)]
                type BetaParams = hbit::Params;

                $fn
            }
            SwapContext {
                alpha: LockProtocol::Hbit,
                beta: LockProtocol::Herc20,
                role: Role::Alice,
                ..
            } => {
                #[allow(dead_code)]
                type AlphaParams = hbit::Params;
                #[allow(dead_code)]
                type BetaParams = herc20::Params;

                $fn
            }
            SwapContext {
                alpha: LockProtocol::Hbit,
                beta: LockProtocol::Herc20,
                role: Role::Bob,
                ..
            } => {
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

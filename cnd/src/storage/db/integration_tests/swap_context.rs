use crate::{
    proptest::*,
    storage::{CreatedSwap, Load, Save, SwapContext},
    LockProtocol, Storage,
};
use proptest::prelude::*;
use tokio::runtime::Runtime;

proptest! {
    /// Test strategy:
    /// To be sure that Load<SwapContext> works as expected, we insert several(!) swaps into the DB.
    /// With only a single swap, we would not be sure that the custom SQL query actually outputs the correct combination.
    #[test]
    fn given_several_swaps_can_correctly_load_swap_context(
        first_swap in db::created_swap(hbit::created_swap(), herc20::created_swap()),
        second_swap in db::created_swap(halbit::created_swap(), herc20::created_swap()),
        third_swap in db::created_swap(herc20::created_swap(), halbit::created_swap()),
    ) {
        // GIVEN a database and three swaps
        let storage = Storage::test();
        let mut runtime = Runtime::new().unwrap();

        // WHEN we save these swaps to the database and load their protocol combinations
        let first_swap_context = runtime.block_on(save_and_load(&storage, &first_swap));
        let second_swap_context = runtime.block_on(save_and_load(&storage, &second_swap));
        let third_swap_context = runtime.block_on(save_and_load(&storage, &third_swap));

        // THEN the swap context matches our expectations
        assert_eq!(first_swap_context.alpha, LockProtocol::Hbit);
        assert_eq!(first_swap_context.beta, LockProtocol::Herc20);

        assert_eq!(second_swap_context.alpha, LockProtocol::Halbit);
        assert_eq!(second_swap_context.beta, LockProtocol::Herc20);

        assert_eq!(third_swap_context.alpha, LockProtocol::Herc20);
        assert_eq!(third_swap_context.beta, LockProtocol::Halbit);
    }
}

async fn save_and_load<A, B>(storage: &Storage, swap: &CreatedSwap<A, B>) -> SwapContext
where
    Storage: Save<CreatedSwap<A, B>>,
    CreatedSwap<A, B>: Clone,
{
    storage.save(swap.clone()).await.unwrap();
    storage.load(swap.swap_id).await.unwrap()
}

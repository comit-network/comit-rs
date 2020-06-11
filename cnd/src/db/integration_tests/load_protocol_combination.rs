use crate::{
    db::{
        tables::{Insert, IntoInsertable},
        CreatedSwap, Save, Sqlite,
    },
    proptest::*,
    Protocol, SwapContext,
};
use proptest::prelude::*;
use tokio::runtime::Runtime;

proptest! {
    /// Test strategy:
    /// To be sure that Load<http_api::Swap> works as expected, we insert several(!) swaps into the DB.
    /// With only a single swap, we would not be sure that the custom SQL query actually outputs the correct combination.
    #[test]
    fn given_several_swaps_can_correctly_load_protocol_combinations(
        first_swap in db::created_swap(hbit::created_swap(), herc20::created_swap()),
        second_swap in db::created_swap(halbit::created_swap(), herc20::created_swap()),
        third_swap in db::created_swap(herc20::created_swap(), halbit::created_swap()),
    ) {
        // GIVEN a database and three swaps
        let db = Sqlite::test();
        let mut runtime = Runtime::new().unwrap();

        // WHEN we save these swaps to the database and load their protocol combinations
        let loaded_first_swap = runtime.block_on(save_and_load(&db, &first_swap));
        let loaded_second_swap = runtime.block_on(save_and_load(&db, &second_swap));
        let loaded_third_swap = runtime.block_on(save_and_load(&db, &third_swap));

        // THEN the protocol combinations match our expectations
        assert_eq!(loaded_first_swap.alpha, Protocol::Hbit);
        assert_eq!(loaded_first_swap.beta, Protocol::Herc20);

        assert_eq!(loaded_second_swap.alpha, Protocol::Halbit);
        assert_eq!(loaded_second_swap.beta, Protocol::Herc20);

        assert_eq!(loaded_third_swap.alpha, Protocol::Herc20);
        assert_eq!(loaded_third_swap.beta, Protocol::Halbit);
    }
}

async fn save_and_load<A, B>(db: &Sqlite, swap: &CreatedSwap<A, B>) -> SwapContext
where
    A: Clone + IntoInsertable + Send + 'static,
    B: Clone + IntoInsertable + Send + 'static,
    <A as IntoInsertable>::Insertable: 'static,
    <B as IntoInsertable>::Insertable: 'static,
    Sqlite: Insert<<A as IntoInsertable>::Insertable> + Insert<<B as IntoInsertable>::Insertable>,
{
    db.save(swap.clone()).await.unwrap();
    db.load_swap_context(swap.swap_id).await.unwrap()
}

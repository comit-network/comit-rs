use crate::{
    db::{
        tables::{Insert, IntoInsertable},
        CreatedSwap, Load, Save, Sqlite,
    },
    http_api,
    proptest::*,
};
use comit::Protocol;
use proptest::prelude::*;
use tokio::runtime::Runtime;

proptest! {
    /// Test strategy:
    /// To be sure that Load<http_api::Swap<Protocol, Protocol>> works as expected, we insert several(!) swaps into the DB.
    /// With only a single swap, we would not be sure that the custom SQL query actually outputs the correct combination.
    #[test]
    fn given_several_swaps_can_correctly_load_protocol_combinations(
        first_swap in db::created_swap(hbit::created_swap(), herc20::created_swap()),
        second_swap in db::created_swap(halight::created_swap(), herc20::created_swap()),
        third_swap in db::created_swap(herc20::created_swap(), halight::created_swap()),
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

        assert_eq!(loaded_second_swap.alpha, Protocol::Halight);
        assert_eq!(loaded_second_swap.beta, Protocol::Herc20);

        assert_eq!(loaded_third_swap.alpha, Protocol::Herc20);
        assert_eq!(loaded_third_swap.beta, Protocol::Halight);
    }
}

async fn save_and_load<A, B>(
    db: &Sqlite,
    swap: &CreatedSwap<A, B>,
) -> http_api::Swap<Protocol, Protocol>
where
    A: Clone + IntoInsertable + Send + 'static,
    B: Clone + IntoInsertable + Send + 'static,
    <A as IntoInsertable>::Insertable: 'static,
    <B as IntoInsertable>::Insertable: 'static,
    Sqlite: Insert<<A as IntoInsertable>::Insertable> + Insert<<B as IntoInsertable>::Insertable>,
{
    db.save(swap.clone()).await.unwrap();
    db.load(swap.swap_id).await.unwrap()
}

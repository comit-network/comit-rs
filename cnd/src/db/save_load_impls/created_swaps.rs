use crate::{
    db::{
        tables::{Insert, InsertableSwap, IntoInsertable},
        CreatedSwap, Save, Sqlite,
    },
    swap_protocols::Ledger,
};

#[async_trait::async_trait]
impl<TCreatedA, TCreatedB, TInsertableA, TInsertableB> Save<CreatedSwap<TCreatedA, TCreatedB>>
    for Sqlite
where
    TCreatedA: IntoInsertable<Insertable = TInsertableA> + Clone + Send + 'static,
    TCreatedB: IntoInsertable<Insertable = TInsertableB> + Send + 'static,
    TInsertableA: 'static,
    TInsertableB: 'static,
    Sqlite: Insert<TInsertableA> + Insert<TInsertableB>,
{
    async fn save(
        &self,
        CreatedSwap {
            swap_id,
            role,
            peer,
            alpha,
            beta,
            ..
        }: CreatedSwap<TCreatedA, TCreatedB>,
    ) -> anyhow::Result<()> {
        self.do_in_transaction::<_, _, anyhow::Error>(move |conn| {
            let swap_id = self.save_swap(conn, &InsertableSwap::new(swap_id, peer, role))?;

            let insertable_alpha = alpha.into_insertable(swap_id, role, Ledger::Alpha);
            let insertable_beta = beta.into_insertable(swap_id, role, Ledger::Beta);

            self.insert(conn, &insertable_alpha)?;
            self.insert(conn, &insertable_beta)?;

            Ok(())
        })
        .await?;

        Ok(())
    }
}

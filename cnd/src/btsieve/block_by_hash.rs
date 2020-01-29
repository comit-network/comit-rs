macro_rules! impl_block_by_hash {
    () => {
        impl<C> BlockByHash for Cache<C>
        where
            C: BlockByHash<Block = Block, BlockHash = Hash> + Clone,
        {
            type Block = Block;
            type BlockHash = Hash;

            fn block_by_hash(
                &self,
                block_hash: Self::BlockHash,
            ) -> Box<dyn Future<Item = Self::Block, Error = anyhow::Error> + Send + 'static> {
                let connector = self.connector.clone();
                let cache = Arc::clone(&self.block_cache);
                Box::new(Box::pin(block_by_hash(connector, cache, block_hash)).compat())
            }
        }

        async fn block_by_hash<C>(
            connector: C,
            cache: Arc<Mutex<LruCache<Hash, Block>>>,
            block_hash: Hash,
        ) -> anyhow::Result<Block>
        where
            C: BlockByHash<Block = Block, BlockHash = Hash> + Clone,
        {
            if let Some(block) = cache.lock().await.get(&block_hash) {
                tracing::trace!("Found block in cache: {:x}", block_hash);
                return Ok(block.clone());
            }

            let block = connector.block_by_hash(block_hash.clone()).compat().await?;
            tracing::trace!("Fetched block from connector: {:x}", block_hash);

            // We dropped the lock so at this stage the block may have been inserted by
            // another thread, no worries, inserting the same block twice does not hurt.
            let mut guard = cache.lock().await;
            guard.put(block_hash, block.clone());

            Ok(block)
        }
    };
}

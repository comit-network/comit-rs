#[macro_export]
macro_rules! match_request {
    (
        $request_kind:ident,
        $bitcoin_poll_interval:ident,
        $ethereum_poll_interval:ident,
        $lqs_api_client:ident,
        $request:ident,
        $alpha_ledger_events:ident,
        $beta_ledger_events:ident,
        $block:block
    ) => {
        match $request_kind {
            SwapRequestKind::BitcoinEthereumBitcoinQuantityEtherQuantity(request) => {
                let $request = request;
                let $alpha_ledger_events = Box::new(LqsEvents::new(
                    QueryIdCache::wrap(Arc::clone(&$lqs_api_client)),
                    FirstMatch::new(Arc::clone(&$lqs_api_client), $bitcoin_poll_interval),
                ));
                let $beta_ledger_events = Box::new(LqsEvents::new(
                    QueryIdCache::wrap(Arc::clone(&$lqs_api_client)),
                    FirstMatch::new(Arc::clone(&$lqs_api_client), $ethereum_poll_interval),
                ));
                $block
            }
            SwapRequestKind::BitcoinEthereumBitcoinQuantityErc20Quantity(request) => {
                let $request = request;
                let $alpha_ledger_events = Box::new(LqsEvents::new(
                    QueryIdCache::wrap(Arc::clone(&$lqs_api_client)),
                    FirstMatch::new(Arc::clone(&$lqs_api_client), $bitcoin_poll_interval),
                ));
                let $beta_ledger_events = Box::new(LqsEventsForErc20::new(
                    QueryIdCache::wrap(Arc::clone(&$lqs_api_client)),
                    FirstMatch::new(Arc::clone(&$lqs_api_client), $ethereum_poll_interval),
                ));
                $block
            }
            SwapRequestKind::EthereumBitcoinEtherQuantityBitcoinQuantity(request) => {
                let $request = request;
                let $alpha_ledger_events = Box::new(LqsEvents::new(
                    QueryIdCache::wrap(Arc::clone(&$lqs_api_client)),
                    FirstMatch::new(Arc::clone(&$lqs_api_client), $ethereum_poll_interval),
                ));
                let $beta_ledger_events = Box::new(LqsEvents::new(
                    QueryIdCache::wrap(Arc::clone(&$lqs_api_client)),
                    FirstMatch::new(Arc::clone(&$lqs_api_client), $bitcoin_poll_interval),
                ));
                $block
            }
        }
    };
}

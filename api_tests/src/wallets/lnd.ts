import createLnRpc, {
    AutopilotRpc,
    ChainRpc,
    createAutopilotRpc,
    createChainRpc,
    createInvoicesRpc,
    createRouterRpc,
    createSignRpc,
    createWalletRpc,
    createWatchtowerRpc,
    createWtClientRpc,
    InvoicesRpc,
    LnRpc,
    RouterRpc,
    RpcClientConfig,
    SignRpc,
    WalletRpc,
    WatchtowerRpc,
    WtClientRpc,
} from "@radar/lnrpc";

export class Lnd {
    /**
     * Initialize gRPC clients for the main server and all sub-servers
     * @param config The RPC client connection configuration
     */
    public static async init(config: RpcClientConfig): Promise<Lnd> {
        return new Lnd(
            config,
            await createLnRpc(config),
            await createAutopilotRpc(config),
            await createChainRpc(config),
            await createInvoicesRpc(config),
            await createRouterRpc(config),
            await createSignRpc(config),
            await createWalletRpc(config),
            await createWatchtowerRpc(config),
            await createWtClientRpc(config)
        );
    }

    private constructor(
        public config: RpcClientConfig,
        public lnrpc: LnRpc,
        public autopilotrpc: AutopilotRpc,
        public chainrpc: ChainRpc,
        public invoicesrpc: InvoicesRpc,
        public routerrpc: RouterRpc,
        public signrpc: SignRpc,
        public walletrpc: WalletRpc,
        public watchtowerrpc: WatchtowerRpc,
        public wtclientrpc: WtClientRpc
    ) {}
}

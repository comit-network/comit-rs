declare module "ln-service" {
    export interface AuthenticatedLndGrpc {
        autopilot: any;
        chain: any;
        default: any;
        invoices: any;
        router: any;
        signer: any;
        wallet: any;
    }

    export interface UnauthenticatedLndGrpc {
        unlocker: any;
    }

    interface UnauthenticatedLndGrpcParams {
        cert?: string;
        socket?: string;
    }

    export function unauthenticatedLndGrpc(
        params: UnauthenticatedLndGrpcParams
    ): { lnd: UnauthenticatedLndGrpc };

    interface AuthenticatedLndGrpcParams {
        cert?: string;
        macaroon: string;
        socket?: string;
    }

    export function authenticatedLndGrpc(
        params: AuthenticatedLndGrpcParams
    ): { lnd: AuthenticatedLndGrpc };

    interface CreateSeedParams {
        lnd: UnauthenticatedLndGrpc;
        passphrase?: string;
    }

    export function createSeed(
        params: CreateSeedParams
    ): Promise<{ seed: string }>;

    interface CreateWalletParams {
        lnd: UnauthenticatedLndGrpc;
        passphrase?: string;
        password: string;
        seed: string;
    }

    export function createWallet(params: CreateWalletParams): Promise<void>;

    interface DefaultParams {
        lnd: AuthenticatedLndGrpc;
    }

    interface ChainFeature {
        bit: number;
        is_known: boolean;
        is_required: boolean;
        type: string;
    }

    interface WalletInfo {
        active_channels_count: number;
        alias: string;
        chains: string[];
        color: string;
        current_block_hash: string;
        current_block_height: number;
        features: ChainFeature[];
        is_synced_to_chain: boolean;
        latest_block_at: string;
        peers_count: number;
        pending_channels_count: number;
        public_key: string;
    }

    export function getWalletInfo(params: DefaultParams): Promise<WalletInfo>;

    interface CreateChainAddressParams {
        format: "np2wpkh" | "p2wpkh";
        is_unused?: boolean;
        lnd: AuthenticatedLndGrpc;
    }

    export function createChainAddress(
        params: CreateChainAddressParams
    ): Promise<{ address: string }>;

    export function getChainBalance(
        params: DefaultParams
    ): Promise<{ chain_balance: number }>;

    export function getChannelBalance(
        params: DefaultParams
    ): Promise<{ channel_balance: number; pending_balance: number }>;

    interface AddPeerParams {
        is_temporary?: boolean; // Default: false
        lnd: AuthenticatedLndGrpc;
        public_key: string;
        retry_count?: number;
        socket: string; // ip:port
    }

    export function addPeer(params: AddPeerParams): Promise<void>;

    interface Peer {
        bytes_received: number;
        bytes_sent: number;
        features: ChainFeature[];
        is_inbound: boolean;
        is_sync_peer?: boolean;
        ping_time: number;
        public_key: string;
        socket: string;
        tokens_received: number;
        tokens_sent: number;
    }
    export function getPeers(
        params: DefaultParams
    ): Promise<{
        peers: Peer[];
    }>;

    interface Payment {
        id: string;
        is_outgoing: boolean;
        timeout: number;
        tokens: number;
    }

    interface Channel {
        capacity: number;
        commit_transaction_fee: number;
        commit_transaction_weight: number;
        cooperative_close_address?: string;
        id: string;
        is_active: boolean;
        is_closing: boolean;
        is_opening: boolean;
        is_partner_initiated: boolean;
        is_private: boolean;
        is_static_remote_key: boolean;
        local_balance: number;
        local_reserve: number;
        partner_public_key: string;
        pending_payments: Payment[];
        received: number;
        remote_balance: number;
        remote_reserve: number;
        sent: number;
        time_offline: number;
        time_online: number;
        transaction_id: string;
        transaction_vout: number;
        unsettled_balance: number;
    }

    interface GetChannelsParams {
        is_active?: boolean; // Defaults to false
        is_offline?: boolean; // Defaults to false
        is_private?: boolean; // Defaults to false
        is_public?: boolean; // Defaults to false
        lnd: AuthenticatedLndGrpc;
    }

    export function getChannels(
        params: GetChannelsParams
    ): Promise<{ channels: Channel[] }>;

    interface OpenChannelParams {
        chain_fee_tokens_per_vbyte?: number;
        cooperative_close_address?: string;
        give_tokens?: number;
        is_private?: boolean; // Defaults to false
        lnd: AuthenticatedLndGrpc;
        local_tokens: number;
        min_confirmations?: number;
        min_htlc_mtokens?: string;
        partner_public_key: string;
        partner_csv_delay?: number;
        partner_socket?: string;
    }

    export function openChannel(
        params: OpenChannelParams
    ): Promise<{
        transaction_id: string;
        transaction_vout: number;
    }>;

    interface CreateInvoiceParams {
        cltv_delta?: number;
        description?: string;
        expires_at?: string; // ISO 8601
        is_fallback_included?: boolean;
        is_fallback_nested?: boolean;
        is_including_private_channels?: boolean;
        lnd: AuthenticatedLndGrpc;
        log?: any; // Required when WSS is passed
        secret?: string; // hex
        tokens: number;
        wss?: any; // Web Socket Server Object
    }

    interface CreateInvoiceResponse {
        chain_address?: string;
        created_at: string; // ISO 8601 Date
        description: string;
        id: string;
        request: string; // BOLT 11 Encoded Payment Request String
        secret: string; // Hex
        tokens: number;
    }

    export function createInvoice(
        params: CreateInvoiceParams
    ): Promise<CreateInvoiceResponse>;

    interface Route {
        fee: number; // Total Fee
        fee_mtokens: string;
        hops: {
            channel: string;
            channel_capacity: number;
            fee: number;
            fee_mtokens: string;
            forward: number;
            forward_mtokens: string;
            public_key?: string; // hex
            timeout: number; // Block heigh
        }[];
    }

    interface PayParams {
        lnd: AuthenticatedLndGrpc;
        log?: any; // Required if wss is set
        max_fee?: number;
        max_timeout_height?: number;
        outgoing_channel?: string;
        path?: {
            id: string; // Hex
            routes: Route[];
            mtokens: string;
            timeout: number; // Block Height Expiration
            tokens: number;
        };
        pathfinding_timeout?: number; // Time to Spend Finding a Route (ms)
        request?: string; // BOLT 11 Payment Request
        tokens?: number;
        wss?: any; // <Web Socket Server Object
    }

    export function pay(
        params: PayParams
    ): Promise<{
        fee: number;
        fee_mtokens: string;
        hops: {
            channel: string;
            channel_capacity: number;
            fee_mtokens: string; // Hop Forward Fee Millitokens
            forward_mtokens: string; // Hop Forwarded Millitokens
            timeout: number; // Hop CLTV Expiry Block Height
        }[];
        id: string; // Hex
        is_confirmed: boolean;
        is_outgoing: boolean;
        mtokens: string;
        secret: string; // Hex
        safe_fee: number; // Payment Forwarding Fee Rounded Up Tokens
        safe_tokens: number; // Payment Tokens Rounded Up
        tokens: number;
    }>;

    interface GetInvoiceParams {
        id: string; // Hex
        lnd: AuthenticatedLndGrpc;
    }

    interface GetInvoiceResponse {
        chain_address: string;
        confirmed_at?: string; // ISO 8601 Date
        created_at: string; // ISO 8601 Date
        description: string;
        description_hash?: string; // Hex
        expires_at: string; // ISO 8601 Date
        features: ChainFeature[];
        id: string;
        is_canceled?: boolean;
        is_confirmed: boolean;
        is_held?: boolean;
        is_outgoing: boolean;
        is_private: boolean;
        is_push?: boolean;
        payments: {
            confirmed_at?: string; // ISO 8601 Date
            created_at: string; // ISO 8601 Date
            created_height: number; // Block Height
            in_channel: string;
            is_canceled: boolean;
            is_confirmed: boolean;
            is_held: boolean;
            messages: {
                type: string;
                value: string; // Raw Value Hex
            }[];
            mtokens: string; // Incoming Payment Millitokens
            pending_index?: number;
            tokens: number;
        }[];
        received: number;
        received_mtokens: string;
        request?: string; // Bolt 11 Invoice String>
        secret: string; // Hex
        tokens: number;
    }

    export function getInvoice(
        params: GetInvoiceParams
    ): Promise<GetInvoiceResponse>;
}

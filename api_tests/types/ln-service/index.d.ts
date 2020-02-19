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

    export function getWalletInfo(
        params: DefaultParams
    ): Promise<{
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
    }>;

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
        is_active?: boolean; // false
        is_offline?: boolean; // false
        is_private?: boolean; // false
        is_public?: boolean; // false
        lnd: AuthenticatedLndGrpc;
    }

    export function getChannels(
        params: GetChannelsParams
    ): Promise<{ channels: Channel[] }>;
}

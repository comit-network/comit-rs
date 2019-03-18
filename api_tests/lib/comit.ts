/// HTTP API

import { BtsieveForComitNodeConfig } from "./actor";

export interface AcceptPayload {
    beta_ledger_refund_identity: string;
    alpha_ledger_redeem_identity: string;
}

export interface SwapResponse extends HalResource {
    parameters: any;
    role: string;
    state: {
        alpha_ledger: any;
        beta_ledger: any;
        communication: any;
    };
    status: string;
}

export interface Action {
    type: string;
    payload: any;
}

export interface SwapsResponse extends HalResource {
    _embedded: {
        swaps: Swap[];
    };
}

export interface HalResource {
    _links: any;
    _embedded: any;
}

export interface Swap extends HalResource {
    parameters: any;
    protocol: string;
    status: string;
}

//**** Config files ****//

export interface MetaComitNodeConfig {
    // snake_case as it comes from TOML file
    host: string;
    config_dir: string;
}

/// The comit-rs config file
export interface ComitNodeConfig {
    comit: { comit_listen: string; secret_seed: string };
    http_api: { address: string; port: number; logging: boolean };
    btsieve: {
        url: string;
        bitcoin: BtsieveForComitNodeConfig;
        ethereum: BtsieveForComitNodeConfig;
    };
}

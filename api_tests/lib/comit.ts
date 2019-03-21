/// HTTP API

import { BtsieveForComitNodeConfig } from "./actor";

export interface AcceptRequestBody {
    beta_ledger_refund_identity?: string;
    alpha_ledger_redeem_identity?: string;
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

export interface ActionDirective {
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

export interface Asset {
    name: string;
    quantity: string;
    token_contract?: string;
}

export interface Ledger {
    name: string;
    network: string;
}

export interface SwapRequest {
    alpha_ledger: Ledger;
    beta_ledger: Ledger;
    alpha_asset: Asset;
    beta_asset: Asset;
    beta_ledger_redeem_identity?: string;
    alpha_ledger_refund_identity?: string;
    alpha_expiry: number;
    beta_expiry: number;
    peer: string;
}

export enum Method {
    Get,
    Post,
}

export enum ActionKind {
    Accept = "accept",
    Decline = "decline",
    Deploy = "deploy",
    Fund = "fund",
    Redeem = "redeem",
    Refund = "refund",
}

export function getMethod(action: ActionKind): Method {
    switch (action) {
        case ActionKind.Accept:
            return Method.Post;
        case ActionKind.Decline:
            return Method.Post;
        case ActionKind.Deploy:
            return Method.Get;
        case ActionKind.Fund:
            return Method.Get;
        case ActionKind.Redeem:
            return Method.Get;
        case ActionKind.Refund:
            return Method.Get;
        default:
            throw new Error("Method undefined for action " + action);
    }
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

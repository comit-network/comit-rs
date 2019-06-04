/// HTTP API

import { BtsieveForComitNodeConfig } from "./actor";

export type LedgerAction =
    | {
          type: "bitcoin-send-amount-to-address";
          payload: { to: string; amount: string; network: string };
      }
    | {
          type: "bitcoin-broadcast-signed-transaction";
          payload: {
              hex: string;
              network: string;
              min_median_block_time?: number;
          };
      }
    | {
          type: "ethereum-deploy-contract";
          payload: {
              data: string;
              amount: string;
              gas_limit: string;
              network: string;
          };
      }
    | {
          type: "ethereum-call-contract";
          payload: {
              contract_address: string;
              data: string;
              gas_limit: string;
              network: string;
              min_block_timestamp?: number;
          };
      };

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

export enum ActionKind {
    Accept = "accept",
    Decline = "decline",
    Deploy = "deploy",
    Fund = "fund",
    Redeem = "redeem",
    Refund = "refund",
}

//**** Config files ****//

export interface MetaComitNodeConfig {
    // snake_case as it comes from TOML file
    host: string;
    config_file: string;
}

/// The comit-rs config file
export interface ComitNodeConfig {
    comit: { comit_listen: string; secret_seed: string };
    http_api: { address: string; port: number };
    web_gui: { address: string; port: number };
    network: { listen: string[] };
    btsieve: {
        url: string;
        bitcoin: BtsieveForComitNodeConfig;
        ethereum: BtsieveForComitNodeConfig;
    };
}

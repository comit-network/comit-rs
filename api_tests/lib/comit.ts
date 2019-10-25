/// HTTP API

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
              chain_id: number;
          };
      }
    | {
          type: "ethereum-call-contract";
          payload: {
              contract_address: string;
              data: string;
              gas_limit: string;
              chain_id: number;
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
    network?: string;
    chain_id?: number;
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

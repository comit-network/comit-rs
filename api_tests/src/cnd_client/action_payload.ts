/**
 * The payload of the actions returned by cnd.
 *
 * An "action" is something that need to be executed by the COMIT App/User on a specific ledger to execute a swap.
 */
export interface BitcoinSendAmountToAddressPayload {
    to: string;
    amount: string;
    network: string;
}

export interface BitcoinBroadcastSignedTransactionPayload {
    hex: string;
    network: string;
}

export interface EthereumDeployContractPayload {
    data: string;
    amount: string;
    gas_limit: string;
    chain_id: string;
}

export interface EthereumCallContractPayload {
    contract_address: string;
    data: string;
    gas_limit: string;
    chain_id: string;
}

export interface LndSendPaymentPayload {
    self_public_key: string;
    to_public_key: string;
    amount: string;
    secret_hash: string;
    final_cltv_delta: number;
    chain: string;
    network: string;
}

export interface LndAddHoldInvoicePayload {
    self_public_key: string;
    amount: string;
    secret_hash: string;
    expiry: number;
    cltv_expiry: number;
    chain: string;
    network: string;
}

export interface LndSettleInvoicePayload {
    self_public_key: string;
    secret: string;
    chain: string;
    network: string;
}

export type LedgerAction =
    | {
          type: "bitcoin-send-amount-to-address";
          payload: BitcoinSendAmountToAddressPayload;
      }
    | {
          type: "bitcoin-broadcast-signed-transaction";
          payload: BitcoinBroadcastSignedTransactionPayload;
      }
    | {
          type: "ethereum-deploy-contract";
          payload: EthereumDeployContractPayload;
      }
    | {
          type: "ethereum-call-contract";
          payload: EthereumCallContractPayload;
      }
    | {
          type: "lnd-send-payment";
          payload: LndSendPaymentPayload;
      }
    | {
          type: "lnd-add-hold-invoice";
          payload: LndAddHoldInvoicePayload;
      }
    | {
          type: "lnd-settle-invoice";
          payload: LndSettleInvoicePayload;
      }
    | {
          type: string;
          payload: any;
      };

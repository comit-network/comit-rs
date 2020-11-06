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
        type: string;
        payload: any;
    };

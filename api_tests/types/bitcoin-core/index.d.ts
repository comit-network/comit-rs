declare module "bitcoin-core" {
    export interface GetBlockchainInfoResponse {
        mediantime: number;
    }

    export interface VerboseRawTransactionResponse {
        vout: Array<{
            scriptPubKey: {
                addresses: string[];
            };
            value: number;
        }>;
    }

    export type HexRawTransactionResponse = string;

    export type GetRawTransactionResponse =
        | null
        | HexRawTransactionResponse
        | VerboseRawTransactionResponse;

    interface ClientConstructorArgs {
        network: string;
        username: string;
        password: string;
        host: string;
        port: number;
    }

    export default class BitcoinRpcClient {
        public constructor(args: ClientConstructorArgs);

        public generate(num: number): Promise<string[]>;
        public getBlockchainInfo(): Promise<GetBlockchainInfoResponse>;

        public getBlockCount(): Promise<number>;

        public getRawTransaction(
            txId: string,
            verbose?: boolean,
            blockHash?: string
        ): Promise<GetRawTransactionResponse>;

        public sendToAddress(
            address: string,
            amount: number | string
        ): Promise<string>;

        public sendRawTransaction(hexString: string): Promise<string>;
    }
}

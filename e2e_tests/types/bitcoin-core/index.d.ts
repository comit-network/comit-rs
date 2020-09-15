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

    export interface ClientConstructorArgs {
        network: string;
        username: string;
        password: string;
        host: string;
        port: number;
        wallet?: string;
    }

    export default class BitcoinRpcClient {
        public wallet;
        public constructor(args: ClientConstructorArgs);

        public getBlockchainInfo(): Promise<GetBlockchainInfoResponse>;

        public getBlockCount(): Promise<number>;

        public getNewAddress(): Promise<string>;

        public getRawTransaction(
            txId: string,
            verbose?: boolean,
            blockHash?: string
        ): Promise<GetRawTransactionResponse>;

        public sendToAddress(
            address: string,
            amount: number | string
        ): Promise<string>;

        public generateToAddress(
            nblocks: number,
            address: string
        ): Promise<string[]>;

        public sendRawTransaction(hexString: string): Promise<string>;

        public createWallet(wallet: string): Promise<void>;
    }
}

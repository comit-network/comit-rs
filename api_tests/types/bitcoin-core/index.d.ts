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
        wallet: string;
    }

    interface GetDescriptorInfo {
        descriptor: string;
        checksum: string;
        isrange: boolean;
        issolvable: boolean;
        hasprivatekeys: boolean;
    }

    // note: (for us) unneeded fields are not provided in this interface
    interface ImportMultiRequest {
        desc: string;
        timestamp: number | string;
        range: number;
    }

    interface ImportMultiResponse {
        success: boolean;
    }

    interface ImportMultiOptions {
        rescan: boolean;
    }

    interface CreateWalletResponse {
        name: string;
        warning: string;
    }

    export default class BitcoinRpcClient {
        public constructor(args: ClientConstructorArgs);

        public getBalance(): Promise<number>;

        public getBlockchainInfo(): Promise<GetBlockchainInfoResponse>;

        public getBlockCount(): Promise<number>;

        public getNewAddress(): Promise<string>;

        public getDescriptorInfo(
            descriptor: string
        ): Promise<GetDescriptorInfo>;

        public importMulti(
            request: ImportMultiRequest[],
            options: ImportMultiOptions
        ): Promise<ImportMultiResponse[]>;

        public createWallet(name: string): Promise<CreateWalletResponse>;

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
    }
}

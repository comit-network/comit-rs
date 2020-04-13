import { BitcoinWallet } from "comit-sdk";
import { BitcoinNodeConfig } from "../ledgers";
import BitcoinRpcClient from "bitcoin-core";

export class BitcoindWallet implements BitcoinWallet {
    constructor(private readonly client: BitcoinRpcClient) {}

    public static async newInstance(
        network: string,
        hdKey: string,
        config: BitcoinNodeConfig
    ): Promise<BitcoindWallet> {
        const bitcoinRpcClient = new BitcoinRpcClient({
            network,
            host: "localhost",
            port: config.rpcPort,
            username: config.username,
            password: config.password,
            wallet: "miner",
        });

        // everything before is the same for every xprv
        const walletName = hdKey.substr(16, 20);
        const createWalletResponse = await bitcoinRpcClient.createWallet(
            walletName
        );

        const client = new BitcoinRpcClient({
            network,
            host: "localhost",
            port: config.rpcPort,
            username: config.username,
            password: config.password,
            wallet: createWalletResponse.name,
        });

        let descriptor = `wpkh(${hdKey}/0h/0h/*h)`;

        const descriptorInfo = await client.getDescriptorInfo(descriptor);
        descriptor = `${descriptor}#${descriptorInfo.checksum}`;

        const request = {
            desc: descriptor,
            timestamp: 0,
            range: 0,
        };
        // no need to rescan as we mint only after the initialization
        const options = { rescan: false };
        const importMultiResponse = await client.importMulti(
            [request],
            options
        );

        if (!importMultiResponse[0].success) {
            return Promise.reject("Could not import xprv key");
        }

        return new BitcoindWallet(client);
    }

    public async broadcastTransaction(
        transactionHex: string,
        _network: string
    ): Promise<string> {
        return this.client.sendRawTransaction(transactionHex);
    }

    public async getAddress(): Promise<string> {
        return this.client.getNewAddress();
    }

    public async getBalance(): Promise<number> {
        return this.client.getBalance();
    }

    public getFee(): string {
        return "150";
    }

    public async sendToAddress(
        address: string,
        satoshis: number,
        _network: string
    ): Promise<string> {
        const bitcoin = satoshis / 100_000_000;
        return this.client.sendToAddress(address, bitcoin);
    }
}

import * as bcoin from "bcoin";
import BitcoinRpcClient from "bitcoin-core";
import { Asset, BitcoinWallet as BitcoinWalletSdk } from "comit-sdk";
import { toBitcoin, toSatoshi } from "satoshi-bitcoin";
import { BitcoinNodeConfig } from "../../lib/bitcoin";
import { pollUntilMinted, Wallet } from "./index";

export class BitcoinWallet implements Wallet {
    public static async newInstance(config: BitcoinNodeConfig) {
        const hdKey = bcoin.HDPrivateKey.generate().xprivkey(config.network);
        const wallet = await BitcoinWalletSdk.newInstance(
            config.network,
            // config.host == "localhost", which appears to be invalid for bcoin
            `127.0.0.1:${config.p2pPort}`,
            hdKey
        );

        const bitcoinRpcClient = new BitcoinRpcClient({
            network: config.network,
            port: config.rpcPort,
            host: config.host,
            username: config.username,
            password: config.password,
        });

        return new BitcoinWallet(wallet, bitcoinRpcClient);
    }

    public MaximumFee = 100000;

    private constructor(
        public readonly inner: BitcoinWalletSdk,
        private readonly bitcoinRpcClient: any
    ) {}

    public async mint(asset: Asset): Promise<void> {
        if (asset.name !== "bitcoin") {
            throw new Error(
                `Cannot mint asset ${asset.name} with BitcoinWallet`
            );
        }

        const startingBalance = await this.getBalance();

        const minimumExpectedBalance = asset.quantity;
        await this.bitcoinRpcClient.generate(101);
        await this.bitcoinRpcClient.sendToAddress(
            await this.address(),
            toBitcoin(minimumExpectedBalance * 2) // make sure we have at least twice as much
        );

        await pollUntilMinted(this, startingBalance + minimumExpectedBalance);
    }

    public address(): Promise<string> {
        return this.inner.getAddress();
    }

    public async getBalance(): Promise<number> {
        return toSatoshi(await this.inner.getBalance());
    }
}

import * as bcoin from "bcoin";
import BitcoinRpcClient from "bitcoin-core";
import {
    Asset,
    BigNumber,
    InMemoryBitcoinWallet as BitcoinWalletSdk,
} from "comit-sdk";
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
        private readonly bitcoinRpcClient: BitcoinRpcClient
    ) {}

    public async mint(asset: Asset): Promise<void> {
        if (asset.name !== "bitcoin") {
            throw new Error(
                `Cannot mint asset ${asset.name} with BitcoinWallet`
            );
        }

        const startingBalance = new BigNumber(
            await this.getBalanceByAsset(asset)
        );

        const minimumExpectedBalance = new BigNumber(asset.quantity);

        const blockHeight = await this.bitcoinRpcClient.getBlockCount();
        if (blockHeight < 101) {
            throw new Error(
                "unable to mint bitcoin, coinbase transactions are not yet spendable"
            );
        }

        await this.bitcoinRpcClient.sendToAddress(
            await this.address(),
            toBitcoin(minimumExpectedBalance.times(2).toString()) // make sure we have at least twice as much
        );

        await pollUntilMinted(
            this,
            startingBalance.plus(minimumExpectedBalance),
            asset
        );
    }

    public address(): Promise<string> {
        return this.inner.getAddress();
    }

    public async getBalanceByAsset(asset: Asset): Promise<BigNumber> {
        if (asset.name !== "bitcoin") {
            throw new Error(
                `Cannot read balance for asset ${asset.name} with BitcoinWallet`
            );
        }
        return new BigNumber(toSatoshi(await this.inner.getBalance()));
    }

    public async getBlockchainTime(): Promise<number> {
        const blockchainInfo = await this.bitcoinRpcClient.getBlockchainInfo();

        return blockchainInfo.mediantime;
    }

    public fee(): string {
        return this.inner.getFee();
    }

    public async sendToAddress(
        address: string,
        satoshis: number,
        network: string
    ): Promise<string> {
        return this.inner.sendToAddress(address, satoshis, network);
    }
}

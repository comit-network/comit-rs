import * as bcoin from "bcoin";
import BitcoinRpcClient from "bitcoin-core";
import {
    Asset,
    BigNumber,
    InMemoryBitcoinWallet as BitcoinWalletSdk,
} from "comit-sdk";
import { toBitcoin, toSatoshi } from "satoshi-bitcoin";
import { pollUntilMinted, Wallet } from "./index";
import { BitcoinNodeConfig } from "../ledgers/ledger_runner";

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

    public async mintToAddress(
        minimumExpectedBalance: BigNumber,
        toAddress: string
    ): Promise<void> {
        const blockHeight = await this.bitcoinRpcClient.getBlockCount();
        if (blockHeight < 101) {
            throw new Error(
                "unable to mint bitcoin, coinbase transactions are not yet spendable"
            );
        }

        await this.bitcoinRpcClient.sendToAddress(
            toAddress,
            toBitcoin(minimumExpectedBalance.times(2).toString()) // make sure we have at least twice as much
        );
    }

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

        await this.mintToAddress(minimumExpectedBalance, await this.address());

        await pollUntilMinted(
            this,
            startingBalance.plus(minimumExpectedBalance),
            asset
        );
    }

    public async address(): Promise<string> {
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
}

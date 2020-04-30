import crypto from "crypto";
import { bip32, networks } from "bitcoinjs-lib";
import { Logger } from "log4js";
import BitcoinRpcClient from "bitcoin-core";
import {
    Asset,
    BitcoindWallet as BitcoinWalletSdk,
    BitcoindWallet,
} from "comit-sdk";
import { toBitcoin, toSatoshi } from "satoshi-bitcoin";
import { pollUntilMinted, Wallet } from "./index";
import { BitcoinNodeConfig } from "../ledgers";

export class BitcoinWallet implements Wallet {
    public static async newInstance(config: BitcoinNodeConfig, logger: Logger) {
        const hdKey = bip32.fromSeed(crypto.randomBytes(32), networks.regtest);
        const derivationPath = "44h/1h/0h/0/*";
        const walletDescriptor = `wpkh(${hdKey.toBase58()}/${derivationPath})`;

        const walletName = hdKey.fingerprint.toString("hex");
        const wallet = await BitcoindWallet.newInstance({
            url: config.rpcUrl,
            username: config.username,
            password: config.password,
            walletDescriptor,
            walletName,
            rescan: false,
        });

        const rpcClientArgs = {
            network: config.network,
            port: config.rpcPort,
            host: config.host,
            username: config.username,
            password: config.password,
        };

        const minerClient = new BitcoinRpcClient({
            ...rpcClientArgs,
            wallet: config.minerWallet,
        });

        const defaultClient = new BitcoinRpcClient({
            ...rpcClientArgs,
        });

        return new BitcoinWallet(wallet, defaultClient, minerClient, logger);
    }

    public MaximumFee = 100000;

    private constructor(
        public readonly inner: BitcoinWalletSdk,
        private readonly defaultClient: BitcoinRpcClient,
        private readonly minerClient: BitcoinRpcClient,
        private readonly logger: Logger
    ) {}

    public async mintToAddress(
        minimumExpectedBalance: bigint,
        toAddress: string
    ): Promise<void> {
        const blockHeight = await this.defaultClient.getBlockCount();
        if (blockHeight < 101) {
            throw new Error(
                "unable to mint bitcoin, coinbase transactions are not yet spendable"
            );
        }

        // make sure we have at least twice as much
        const amount = toBitcoin(
            (minimumExpectedBalance * BigInt(2)).toString()
        );

        await this.minerClient.sendToAddress(toAddress, amount);

        this.logger.info("Minted", amount, "bitcoin for", toAddress);
    }

    public async mint(asset: Asset): Promise<void> {
        if (asset.name !== "bitcoin") {
            throw new Error(
                `Cannot mint asset ${asset.name} with BitcoinWallet`
            );
        }

        const startingBalance = await this.getBalanceByAsset(asset);

        const minimumExpectedBalance = BigInt(asset.quantity);

        await this.mintToAddress(minimumExpectedBalance, await this.address());

        await pollUntilMinted(
            this,
            startingBalance + minimumExpectedBalance,
            asset
        );
    }

    public async address(): Promise<string> {
        return this.inner.getAddress();
    }

    public async getBalanceByAsset(asset: Asset): Promise<bigint> {
        if (asset.name !== "bitcoin") {
            throw new Error(
                `Cannot read balance for asset ${asset.name} with BitcoinWallet`
            );
        }
        return BigInt(toSatoshi(await this.inner.getBalance()));
    }

    public async getBlockchainTime(): Promise<number> {
        const blockchainInfo = await this.defaultClient.getBlockchainInfo();

        return blockchainInfo.mediantime;
    }

    public async close(): Promise<void> {
        return this.inner.close();
    }
}

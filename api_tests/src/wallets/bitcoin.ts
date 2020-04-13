import { Logger } from "log4js";
import * as bcoin from "bcoin";
import BitcoinRpcClient from "bitcoin-core";
import { Asset, BigNumber } from "comit-sdk";
import { toBitcoin, toSatoshi } from "satoshi-bitcoin";
import { pollUntilMinted, Wallet } from "./index";
import { BitcoinNodeConfig } from "../ledgers";
import { BitcoindWallet } from "./bitcoind_wallet";

export class BitcoinWallet implements Wallet {
    public static async newInstance(config: BitcoinNodeConfig, logger: Logger) {
        const hdKey = bcoin.HDPrivateKey.generate().xprivkey(config.network);

        const minerRpcClient = new BitcoinRpcClient({
            network: config.network,
            port: config.rpcPort,
            host: config.host,
            username: config.username,
            password: config.password,
            wallet: "miner",
        });

        const wallet = await BitcoindWallet.newInstance(
            config.network,
            hdKey,
            config
        );

        return new BitcoinWallet(wallet, minerRpcClient, logger);
    }

    public MaximumFee = 100000;

    private constructor(
        public readonly inner: BitcoindWallet,
        private readonly minerRpcClient: BitcoinRpcClient,
        private readonly logger: Logger
    ) {}

    public async mintToAddress(
        minimumExpectedBalance: BigNumber,
        toAddress: string
    ): Promise<void> {
        const blockHeight = await this.minerRpcClient.getBlockCount();
        if (blockHeight < 101) {
            throw new Error(
                "unable to mint bitcoin, coinbase transactions are not yet spendable"
            );
        }

        // make sure we have at least twice as much
        const amount = toBitcoin(minimumExpectedBalance.times(2).toString());

        await this.minerRpcClient.sendToAddress(toAddress, amount);

        this.logger.info("Minted", amount, "bitcoin for", toAddress);
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
        const blockchainInfo = await this.minerRpcClient.getBlockchainInfo();
        return blockchainInfo.mediantime;
    }
}

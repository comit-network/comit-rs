import { pollUntilMinted, Wallet } from "./index";
import { Asset } from "../asset";
import BigNumber from "bignumber.js";
import { Lnd } from "../lnd";
import { HarnessGlobal } from "../../lib/util";
import { Logger } from "log4js";
import { E2ETestActorConfig } from "../../lib/config";
import { BitcoinWallet } from "./bitcoin";

declare var global: HarnessGlobal;

export class LightningWallet implements Wallet {
    public static async newInstance(
        bitcoinWallet: BitcoinWallet,
        logger: Logger,
        logDir: string,
        actorConfig: E2ETestActorConfig
    ) {
        const bitcoindDataDir = global.bitcoind.getDataDir();

        const lnd = new Lnd(logger, logDir, actorConfig, bitcoindDataDir);
        await lnd.start();

        return new LightningWallet(lnd, bitcoinWallet);
    }

    public MaximumFee = 0;

    private constructor(
        public readonly inner: Lnd,
        private readonly bitcoinWallet: BitcoinWallet
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

        await this.bitcoinWallet.mintToAddress(
            minimumExpectedBalance,
            await this.address()
        );

        await pollUntilMinted(
            this,
            startingBalance.plus(minimumExpectedBalance),
            asset
        );
    }

    public async address(): Promise<string> {
        return this.inner.createChainAddress();
    }

    public async getBalanceByAsset(asset: Asset): Promise<BigNumber> {
        if (asset.name !== "bitcoin") {
            throw new Error(
                `Cannot read balance for asset ${asset.name} with LndWallet`
            );
        }

        const chainBalance = await this.inner.getChainBalance();
        console.log("chainBalance", chainBalance);

        const channelBalance = await this.inner.getChannelBalance();
        console.log("channelBalance", channelBalance);

        return new BigNumber(chainBalance).plus(channelBalance);
    }

    // This functions does not have its place on a Wallet
    public async getBlockchainTime(): Promise<number> {
        throw new Error(
            "getBlockchainTime should not be called for LightningWallet"
        );
    }

    public addPeer(toWallet: LightningWallet) {
        return this.inner.addPeer(toWallet.inner);
    }
}

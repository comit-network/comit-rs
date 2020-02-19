import { pollUntilMinted, Wallet } from "./index";
import { Asset } from "../asset";
import BigNumber from "bignumber.js";
import { Lnd } from "../lnd";
import { Logger } from "log4js";
import { E2ETestActorConfig } from "../../lib/config";
import { BitcoinWallet } from "./bitcoin";
import { sleep } from "../utils";
import { CreateInvoiceResponse } from "ln-service";

export class LightningWallet implements Wallet {
    public static async newInstance(
        bitcoinWallet: BitcoinWallet,
        logger: Logger,
        logDir: string,
        bitcoindDataDir: string,
        actorConfig: E2ETestActorConfig
    ) {
        const lnd = new Lnd(logger, logDir, actorConfig, bitcoindDataDir);
        await lnd.start();

        return new LightningWallet(lnd, logger, bitcoinWallet);
    }

    public MaximumFee = 0;

    private constructor(
        public readonly inner: Lnd,
        private readonly logger: Logger,
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
        const channelBalance = await this.inner.getChannelBalance();

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

    public getPeers() {
        return this.inner.getPeers();
    }

    public getChannels() {
        return this.inner.getChannels();
    }

    public async openChannel(toWallet: LightningWallet, quantity: number) {
        // First, need to check everyone is sync'd to the chain

        const thisIsSynced = (await this.inner.getWalletInfo())
            .is_synced_to_chain;
        const toIsSynced = (await toWallet.inner.getWalletInfo())
            .is_synced_to_chain;

        while (!thisIsSynced || !toIsSynced) {
            this.logger.info(
                `One of the lnd node is not yet synced, waiting. this: ${thisIsSynced}, to: ${toIsSynced}`
            );
            await sleep(500);
        }

        const {
            transaction_id,
            transaction_vout,
        } = await this.inner.openChannel(toWallet.inner, quantity);
        this.logger.debug("Channel opened, waiting for confirmations");

        await this.pollUntilChannelIsOpen(transaction_id, transaction_vout);
    }

    public createInvoice(quantity: number): Promise<CreateInvoiceResponse> {
        return this.inner.createInvoice(quantity);
    }

    public pay(invoice: CreateInvoiceResponse) {
        return this.inner.pay(invoice.request);
    }

    public async assertInvoiceSettled(invoice: CreateInvoiceResponse) {
        const resp = await this.inner.getInvoice(invoice.id);
        if (resp.is_confirmed && resp.tokens === invoice.tokens) {
            return;
        } else {
            throw new Error(
                `Invoices ${invoice.id} is not settled; confirmed: ${resp.is_confirmed}. Tokens: ${resp.tokens}/${invoice.tokens}`
            );
        }
    }

    private async pollUntilChannelIsOpen(
        transactionId: string,
        transactionVout: number
    ): Promise<void> {
        const channels = await this.getChannels();
        for (const channel of channels) {
            this.logger.debug("Found a channel:", channel);
            if (
                channel.transaction_id === transactionId &&
                channel.transaction_vout === transactionVout
            ) {
                return;
            }
        }
        await sleep(500);
        return this.pollUntilChannelIsOpen(transactionId, transactionVout);
    }
}

import { pollUntilMinted, Wallet } from "./index";
import { Asset } from "../asset";
import BigNumber from "bignumber.js";
import { Logger } from "log4js";
import { BitcoinWallet } from "./bitcoin";
import { sleep } from "../utils";
import {
    LightningWallet as LightningWalletSdk,
    Lnd,
    Outpoint,
} from "comit-sdk";
import { AddressType } from "@radar/lnrpc";

export class LightningWallet implements Wallet {
    public static async newInstance(
        bitcoinWallet: BitcoinWallet,
        logger: Logger,
        lnd: Lnd,
        lndp2pHost: string,
        lndp2pPort: number
    ) {
        const inner = new LightningWalletSdk(lnd, lndp2pHost, lndp2pPort);

        return new LightningWallet(inner, logger, bitcoinWallet);
    }

    public MaximumFee = 0;

    private constructor(
        public readonly inner: LightningWalletSdk,
        private readonly logger: Logger,
        private readonly bitcoinWallet: BitcoinWallet
    ) {}

    public async mint(asset: Asset): Promise<void> {
        if (asset.name !== "bitcoin") {
            throw new Error(
                `Cannot mint asset ${asset.name} with LightningWallet`
            );
        }

        const startingBalance = new BigNumber(
            await this.getBalanceByAsset(asset)
        );
        this.logger.debug("starting: ", startingBalance.toString());

        const minimumExpectedBalance = new BigNumber(asset.quantity);
        this.logger.debug("min expected: ", minimumExpectedBalance.toString());

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
        return this.inner.newAddress(AddressType.NESTED_PUBKEY_HASH);
    }

    public async getBalanceByAsset(asset: Asset): Promise<BigNumber> {
        if (asset.name !== "bitcoin") {
            throw new Error(
                `Cannot read balance for asset ${asset.name} with LightningdWallet`
            );
        }

        const walletBalance = await this.inner.confirmedWalletBalance();
        const channelBalance = await this.inner.confirmedChannelBalance();
        return new BigNumber(walletBalance ? walletBalance : 0).plus(
            channelBalance ? channelBalance : 0
        );
    }

    // This function does not have its place on a Wallet
    public async getBlockchainTime(): Promise<number> {
        throw new Error(
            "getBlockchainTime should not be called for LightningWallet"
        );
    }

    public async connectPeer(toWallet: LightningWallet) {
        const pubkey = await toWallet.inner.getPubkey();
        const host = toWallet.inner.getLndP2pSocket();
        return this.inner.lnd.lnrpc.connectPeer({ addr: { pubkey, host } });
    }

    public async listPeers() {
        return this.inner.lnd.lnrpc.listPeers();
    }

    public async getChannels() {
        const listChannelsResponse = await this.inner.lnd.lnrpc.listChannels();
        this.logger.debug(listChannelsResponse);
        return listChannelsResponse.channels;
    }

    // @ts-ignore
    public async openChannel(toWallet: LightningWallet, quantity: number) {
        // First, need to check everyone is sync'd to the chain

        let thisIsSynced = (await this.inner.getInfo()).syncedToChain;
        let toIsSynced = (await toWallet.inner.getInfo()).syncedToChain;

        while (!thisIsSynced || !toIsSynced) {
            this.logger.info(
                `One of the lnd node is not yet synced, waiting. this: ${thisIsSynced}, to: ${toIsSynced}`
            );
            await sleep(500);

            thisIsSynced = (await this.inner.getInfo()).syncedToChain;
            toIsSynced = (await toWallet.inner.getInfo()).syncedToChain;
        }

        const outpoint = await this.inner.openChannel(
            await toWallet.inner.getPubkey(),
            quantity
        );
        this.logger.debug("Channel opened, waiting for confirmations");

        await this.pollUntilChannelIsOpen(outpoint);
    }

    /**
     * Adds an invoice.
     * @param sats
     */
    public async addInvoice(
        sats: string
    ): Promise<{
        rHash: string;
        paymentRequest: string;
    }> {
        return this.inner.addInvoice(sats);
    }

    /**
     * Pay a payment-request
     *
     * @param request A BOLT11-encoded payment request
     */
    public async pay(request: string) {
        return this.inner.sendPaymentWithRequest(request);
    }

    public async lookupInvoice(secretHash: string) {
        return this.inner.lookupInvoice(secretHash);
    }

    private async pollUntilChannelIsOpen(outpoint: Outpoint): Promise<void> {
        const { txId, vout } = outpoint;
        const channels = await this.getChannels();
        if (channels) {
            for (const channel of channels) {
                this.logger.debug(`Looking for channel ${txId}:${vout}`);
                this.logger.debug("Found a channel:", channel);
                if (channel.channelPoint === `${txId}:${vout}`) {
                    return;
                }
            }
        }
        await sleep(500);
        return this.pollUntilChannelIsOpen(outpoint);
    }
}

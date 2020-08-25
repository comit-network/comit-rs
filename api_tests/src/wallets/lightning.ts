import { pollUntilMinted, Wallet } from "./index";
import { Asset } from "../asset";
import { BitcoinWallet } from "./bitcoin";
import { sleep } from "../utils";
import { LightningWallet as LightningWalletSdk, Outpoint } from "comit-sdk";
import { AddressType, Peer } from "@radar/lnrpc";
import { Logger } from "log4js";
import { LightningNodeConfig } from "../ledgers";

export interface LightningWallet extends Wallet {
    readonly inner: LightningWalletSdk;

    address(): Promise<string>;
    pubkey(): Promise<string>;
    connectPeer(toWallet: LightningWallet): Promise<any>;
    listPeers(): Promise<Peer[]>;
    getChannels(): Promise<any>; // any because dependency versions don't match
    openChannel(toWallet: LightningWallet, quantity: number): Promise<void>;
    addInvoice(
        sats: string
    ): Promise<{
        rHash: string;
        paymentRequest: string;
    }>;
}

export class LndWallet implements LightningWallet {
    public static async newInstance(
        bitcoinWallet: BitcoinWallet,
        logger: Logger,
        config: LightningNodeConfig
    ) {
        const inner = await LightningWalletSdk.newInstance(
            config.tlsCertPath,
            config.macaroonPath,
            config.grpcSocket,
            config.p2pSocket
        );

        logger.debug("lnd getinfo:", await inner.lnd.lnrpc.getInfo());

        return new LndWallet(inner, logger, bitcoinWallet);
    }

    public MaximumFee = 0;

    private constructor(
        readonly inner: LightningWalletSdk,
        private readonly logger: Logger,
        private readonly bitcoinWallet: BitcoinWallet
    ) {}

    public async mint(asset: Asset): Promise<void> {
        if (asset.name !== "bitcoin") {
            throw new Error(
                `Cannot mint asset ${asset.name} with LightningWallet`
            );
        }

        const startingBalance = await this.getBalanceByAsset(asset);
        this.logger.debug("starting: ", startingBalance.toString());

        const minimumExpectedBalance = BigInt(asset.quantity);
        this.logger.debug("min expected: ", minimumExpectedBalance.toString());

        await this.bitcoinWallet.mintToAddress(
            minimumExpectedBalance,
            await this.address()
        );

        await pollUntilMinted(
            this,
            startingBalance + minimumExpectedBalance,
            asset
        );
    }

    public async address(): Promise<string> {
        return this.inner.newAddress(AddressType.NESTED_PUBKEY_HASH);
    }

    public async getBalanceByAsset(asset: Asset): Promise<bigint> {
        if (asset.name !== "bitcoin") {
            throw new Error(
                `Cannot read balance for asset ${asset.name} with LightningWallet`
            );
        }

        const walletBalance = await this.inner
            .confirmedWalletBalance()
            .then((balance) => BigInt(balance ? balance : 0));
        const channelBalance = await this.inner
            .confirmedChannelBalance()
            .then((balance) => BigInt(balance ? balance : 0));
        return walletBalance + channelBalance;
    }

    // This function does not have its place on a Wallet
    public async getBlockchainTime(): Promise<number> {
        throw new Error(
            "getBlockchainTime should not be called for LightningWallet"
        );
    }

    public async connectPeer(toWallet: LightningWallet) {
        const pubkey = await toWallet.inner.getPubkey();
        const host = toWallet.inner.p2pSocket;
        return this.inner.lnd.lnrpc.connectPeer({ addr: { pubkey, host } });
    }

    public async listPeers(): Promise<Peer[]> {
        const response = await this.inner.lnd.lnrpc.listPeers();
        return response.peers ? response.peers : [];
    }

    // @ts-ignore: dependency versions don't match ...
    public async getChannels() {
        const listChannelsResponse = await this.inner.lnd.lnrpc.listChannels();

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

    private async pollUntilChannelIsOpen(outpoint: Outpoint): Promise<void> {
        const { txId, vout } = outpoint;
        const channels = await this.getChannels();
        if (channels) {
            for (const channel of channels) {
                this.logger.debug(`Looking for channel ${txId}:${vout}`);
                if (channel.channelPoint === `${txId}:${vout}`) {
                    this.logger.debug("Found a channel:", channel);
                    return;
                }
            }
        }
        await sleep(500);
        return this.pollUntilChannelIsOpen(outpoint);
    }

    public async pubkey(): Promise<string> {
        return this.inner.getPubkey();
    }
}

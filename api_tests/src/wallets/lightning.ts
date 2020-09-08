import { pollUntilMinted } from "./index";
import { BitcoinWallet } from "./bitcoin";
import { sleep } from "../utils";
import {
    AddressType,
    GetInfoResponse,
    Invoice,
    PaymentStatus,
    Peer,
    SendResponse,
} from "@radar/lnrpc";
import { Logger } from "log4js";
import { Lnd } from "./lnd";
import { LightningNodeConfig } from "../environment";

export interface LightningWallet {
    readonly p2pSocket: string;

    newFundingAddress(): Promise<string>;
    getPubkey(): Promise<string>;
    connectPeer(toWallet: LightningWallet): Promise<any>;
    listPeers(): Promise<Peer[]>;
    openChannel(toWallet: LightningWallet, quantity: number): Promise<void>;
    addInvoice(
        sats: string
    ): Promise<{
        rHash: string;
        paymentRequest: string;
    }>;
    isSyncedToChain(): Promise<boolean>;
    sendPayment(
        publicKey: string,
        satAmount: string,
        secretHash: string,
        finalCltvDelta: number
    ): Promise<() => Promise<SendResponse>>;
    addHoldInvoice(
        satAmount: string,
        secretHash: string,
        expiry: number,
        cltvExpiry: number
    ): Promise<string>;
    settleInvoice(secret: string): Promise<void>;

    /**
     * Asserts that the available lnd instance is the same than the one connected to cnd.
     *
     * @param selfPublicKey
     * @param chain
     * @param network
     * @throws Error if the lnd instance details mismatch
     */
    assertLndDetails(
        selfPublicKey: string,
        chain: string,
        network: string
    ): Promise<void>;

    getBalance(): Promise<bigint>;
    mint(satoshis: bigint): Promise<void>;
}

export class LndWallet implements LightningWallet {
    public static async newInstance(
        bitcoinWallet: BitcoinWallet,
        logger: Logger,
        config: LightningNodeConfig
    ) {
        const lnd = await Lnd.init({
            tls: config.tlsCertPath,
            macaroonPath: config.macaroonPath,
            server: config.grpcSocket,
        });

        logger.debug("lnd getinfo:", await lnd.lnrpc.getInfo());

        return new LndWallet(lnd, logger, bitcoinWallet, config.p2pSocket);
    }

    private constructor(
        private readonly lnd: Lnd,
        private readonly logger: Logger,
        private readonly bitcoinWallet: BitcoinWallet,
        public readonly p2pSocket: string
    ) {}

    public async mint(satoshis: bigint): Promise<void> {
        const startingBalance = await this.getBalance();
        this.logger.debug("starting: ", startingBalance.toString());

        const minimumExpectedBalance = satoshis;
        this.logger.debug("min expected: ", minimumExpectedBalance.toString());

        await this.bitcoinWallet.mintToAddress(
            minimumExpectedBalance,
            await this.newFundingAddress()
        );

        await pollUntilMinted(
            async () => this.getBalance(),
            startingBalance + minimumExpectedBalance
        );
    }

    public async newFundingAddress(): Promise<string> {
        return this.lnd.lnrpc
            .newAddress({ type: AddressType.NESTED_PUBKEY_HASH })
            .then((r) => r.address);
    }

    public async getBalance(): Promise<bigint> {
        const walletBalance = await this.lnd.lnrpc
            .walletBalance()
            .then((r) => r.confirmedBalance)
            .then((b) => (b ? b : 0))
            .then(BigInt);
        const channelBalance = await this.lnd.lnrpc
            .channelBalance()
            .then((r) => r.balance)
            .then((b) => (b ? b : 0))
            .then(BigInt);

        return walletBalance + channelBalance;
    }

    public async connectPeer(toWallet: LightningWallet) {
        const pubkey = await toWallet.getPubkey();
        const host = toWallet.p2pSocket;
        try {
            await this.lnd.lnrpc.connectPeer({ addr: { pubkey, host } });
        } catch (e) {
            this.logger.warn("Error while connecting to peer", host);
        }
    }

    public async listPeers(): Promise<Peer[]> {
        const response = await this.lnd.lnrpc.listPeers();
        return response.peers ? response.peers : [];
    }

    public async isSyncedToChain(): Promise<boolean> {
        return this.lnd.lnrpc.getInfo().then((r) => r.syncedToChain);
    }

    public async openChannel(toWallet: LightningWallet, quantity: number) {
        // First, need to check everyone is sync'd to the chain
        while (
            !(await this.isSyncedToChain()) ||
            !(await toWallet.isSyncedToChain())
        ) {
            this.logger.info(`One of the lnd node is not yet synced, waiting.`);
            await sleep(500);
        }

        const request = {
            nodePubkeyString: await toWallet.getPubkey(),
            localFundingAmount: quantity.toString(),
        };

        await this.lnd.lnrpc.openChannelSync(request);
    }

    async sendPayment(
        publicKey: string,
        satAmount: string,
        secretHash: string,
        finalCltvDelta: number
    ): Promise<() => Promise<SendResponse>> {
        const publicKeyBuf = Buffer.from(publicKey, "hex");
        const paymentHash = Buffer.from(secretHash, "hex");

        const sendResponsePromise = this.lnd.lnrpc.sendPaymentSync({
            dest: publicKeyBuf,
            amt: satAmount,
            paymentHash,
            finalCltvDelta,
        });

        let isInFlight = false;

        while (!isInFlight) {
            const payments = await this.lnd.lnrpc
                .listPayments({
                    includeIncomplete: true,
                })
                .then((response) => response.payments);

            if (payments) {
                const payment = payments.find(
                    (payment) => payment.paymentHash === secretHash
                );
                if (payment) {
                    isInFlight = payment.status === PaymentStatus.IN_FLIGHT;
                }
            }

            await sleep(100);
        }

        return async () => sendResponsePromise;
    }

    async addHoldInvoice(
        satAmount: string,
        secretHash: string,
        expiry: number,
        cltvExpiry: number
    ): Promise<string> {
        const satAmountNum = parseInt(satAmount, 10);
        const hash = Buffer.from(secretHash, "hex");
        return (
            await this.lnd.invoicesrpc.addHoldInvoice({
                value: satAmountNum,
                hash,
                cltvExpiry,
                expiry,
            })
        ).paymentRequest;
    }

    async settleInvoice(secret: string): Promise<void> {
        const preimage = Buffer.from(secret, "hex");
        await this.lnd.invoicesrpc.settleInvoice({ preimage });
    }

    public async getPubkey(): Promise<string> {
        return this.lnd.lnrpc.getInfo().then((r) => r.identityPubkey);
    }

    public async getInfo(): Promise<GetInfoResponse> {
        return this.lnd.lnrpc.getInfo();
    }

    public async lookupInvoice(secretHash: string): Promise<Invoice> {
        return this.lnd.lnrpc.lookupInvoice({
            rHashStr: secretHash,
        });
    }

    public async addInvoice(
        satAmount: string
    ): Promise<{ rHash: string; paymentRequest: string }> {
        const { rHash, paymentRequest } = await this.lnd.lnrpc.addInvoice({
            value: satAmount,
        });

        if (typeof rHash === "string") {
            return { rHash, paymentRequest };
        } else {
            return { rHash: rHash.toString("hex"), paymentRequest };
        }
    }

    /**
     * Asserts that the available lnd instance is the same than the one connected to cnd.
     *
     * @param selfPublicKey
     * @param chain
     * @param network
     * @throws Error if the lnd instance details mismatch
     */
    async assertLndDetails(
        selfPublicKey: string,
        chain: string,
        network: string
    ): Promise<void> {
        const getinfo = await this.lnd.lnrpc.getInfo();

        if (getinfo.identityPubkey !== selfPublicKey) {
            throw new Error(
                `lnd self public key does not match cnd expectations. Expected:${selfPublicKey}, actual:${getinfo.identityPubkey}`
            );
        }

        if (getinfo.chains.length !== 1) {
            throw new Error(
                `lnd is connected to several chains, this is unexpected. Chains: ${JSON.stringify(
                    getinfo.chains
                )}`
            );
        }

        const lndChain = getinfo.chains[0];
        if (lndChain.chain !== chain || lndChain.network !== network) {
            throw new Error(
                `lnd chain does not match cnd expectation. Expected:${lndChain}, actual:{ chain: "${chain}", network: "${network}" }`
            );
        }
    }
}

export interface Outpoint {
    txId: string;
    vout: number;
}

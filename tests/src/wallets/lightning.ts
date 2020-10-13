import { sleep } from "../utils";
import createLnRpc, {
    ChannelPoint,
    createInvoicesRpc,
    InvoicesRpc,
    LnRpc,
    PaymentStatus,
    SendResponse,
} from "@radar/lnrpc";
import { LightningNode } from "../environment";
import { Logger } from "log4js";
import pTimeout from "p-timeout";
import pRetry from "p-retry";

export interface LightningChannel {
    getBalance(): Promise<bigint>;
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
    assertLndDetails(
        selfPublicKey: string,
        chain: string,
        network: string
    ): Promise<void>;
}

/**
 * Implements the LightningChannel interface through a reference to an Lnd instance.
 */
export class LndChannel implements LightningChannel {
    public constructor(
        private readonly client: LndClient,
        public readonly chanId: string
    ) {}

    public async getBalance(): Promise<bigint> {
        return this.client.getChannelBalance(this.chanId);
    }

    public async sendPayment(
        publicKey: string,
        satAmount: string,
        secretHash: string,
        finalCltvDelta: number
    ): Promise<() => Promise<SendResponse>> {
        const publicKeyBuf = Buffer.from(publicKey, "hex");
        const paymentHash = Buffer.from(secretHash, "hex");

        const sendResponsePromise = this.client.lnrpc.sendPaymentSync({
            dest: publicKeyBuf,
            amt: satAmount,
            paymentHash,
            finalCltvDelta,
            outgoingChanId: this.chanId,
        });

        let isInFlight = false;

        while (!isInFlight) {
            const payments = await this.client.lnrpc
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

    public async addHoldInvoice(
        satAmount: string,
        secretHash: string,
        expiry: number,
        cltvExpiry: number
    ): Promise<string> {
        const satAmountNum = parseInt(satAmount, 10);
        const hash = Buffer.from(secretHash, "hex");
        return (
            await this.client.invoicesrpc.addHoldInvoice({
                value: satAmountNum,
                hash,
                cltvExpiry,
                expiry,
            })
        ).paymentRequest;
    }

    public async settleInvoice(secret: string): Promise<void> {
        const preimage = Buffer.from(secret, "hex");
        await this.client.invoicesrpc.settleInvoice({ preimage });
    }

    /**
     * Asserts that the available lnd instance is the same than the one connected to cnd.
     *
     * @param selfPublicKey
     * @param chain
     * @param network
     * @throws Error if the lnd instance details mismatch
     */
    public async assertLndDetails(
        selfPublicKey: string,
        chain: string,
        network: string
    ): Promise<void> {
        const getinfo = await this.client.lnrpc.getInfo();

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

/**
 * A client for managing an instance of lnd.
 *
 * It is primary job is to open new channels between two nodes. We make this separation compared to other wallets because creating a channel is interactive and hence requires both parties. As such, the only place to do that is in the actual test after the actors have already been created. This also allows us to run our tests in parallel because each test gets its own channel and can therefore perform swaps and assert balances independently.
 */
export class LndClient {
    public static async newInstance(
        config: LightningNode,
        logger: Logger
    ): Promise<LndClient> {
        const grpcConfig = {
            server: config.grpcSocket,
            tls: config.tlsCertPath,
            macaroonPath: config.macaroonPath,
        };

        return new LndClient(
            await createLnRpc(grpcConfig),
            await createInvoicesRpc(grpcConfig),
            config,
            logger
        );
    }

    private constructor(
        public readonly lnrpc: LnRpc,
        public readonly invoicesrpc: InvoicesRpc,
        public readonly config: LightningNode,
        private readonly logger: Logger
    ) {}

    public async openChannel(to: LndClient, quantity: bigint) {
        await this.connectPeer(to);

        // First, need to check everyone is sync'd to the chain
        while (
            !(await this.isSyncedToChain()) ||
            !(await to.isSyncedToChain())
        ) {
            this.logger.info(`One of the lnd node is not yet synced, waiting.`);
            await sleep(500);
        }

        const request = {
            nodePubkeyString: await to.getPubkey(),
            localFundingAmount: quantity.toString(10),
        };

        // Try to open the channel up to 5 times.
        // We may be unlucky and try to open the channel while
        // the other node is currently syncing a block.
        // Given that we have blocks every second, this occurs unfortunately
        // often enough to have the test suite fail regularly.
        const openChannelResponse = await pRetry(
            () => this.lnrpc.openChannelSync(request),
            { retries: 5 }
        );
        const channelPoint = serializeChannelPoint(openChannelResponse);

        const waitForChannel = async () => {
            let channel = await this.getChannelByPoint(channelPoint);
            while (!channel) {
                await sleep(500);
                channel = await this.getChannelByPoint(channelPoint);
            }
            return channel;
        };

        const channel = await pTimeout(
            waitForChannel(),
            5000,
            "Channel was not created after 5 seconds"
        );

        return new LndChannel(this, channel.chanId);
    }

    public async getPubkey(): Promise<string> {
        return this.lnrpc.getInfo().then((r) => r.identityPubkey);
    }

    private async isSyncedToChain(): Promise<boolean> {
        return this.lnrpc.getInfo().then((r) => r.syncedToChain);
    }

    private async connectPeer(to: LndClient) {
        const pubkey = await to.lnrpc.getInfo().then((r) => r.identityPubkey);
        const host = to.config.p2pSocket;
        try {
            await this.lnrpc.connectPeer({ addr: { pubkey, host } });
        } catch (e) {
            this.logger.warn("Error while connecting to peer", host);
        }
    }

    public async getChannelBalance(id: string): Promise<bigint> {
        const channel = await this.getChannelById(id);

        if (!channel) {
            throw new Error(`Channel with id ${id} does not exist`);
        }

        // as the counterparty, localBalance is undefined ...
        if (!channel.localBalance) {
            return 0n;
        }

        return BigInt(channel.localBalance);
    }

    private async getChannelById(id: string) {
        const channels = await this.lnrpc
            .listChannels()
            .then((r) => r.channels || []);

        return channels.find((c) => c.chanId === id);
    }

    private async getChannelByPoint(point: string) {
        const channels = await this.lnrpc
            .listChannels()
            .then((r) => r.channels || []);

        return channels.find((c) => c.channelPoint === point);
    }
}

function serializeChannelPoint(channel: ChannelPoint) {
    const txId = channel.fundingTxidBytes;
    const txIdReversed = Buffer.from(txId).reverse(); // remember, Bitcoin's txid are reversed!

    const txIdHex = txIdReversed.toString("hex");
    const utxoIndex = channel.outputIndex || 0; // index can be undefined if it is actually 0, wtf lnd ?!

    return `${txIdHex}:${utxoIndex}`;
}

export interface Outpoint {
    txId: string;
    vout: number;
}

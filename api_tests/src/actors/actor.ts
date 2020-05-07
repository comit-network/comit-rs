import {
    Cnd,
    Entity,
    HanEthereumEtherHalightLightningBitcoinRequestBody,
    LedgerAction,
    Swap,
    SwapDetails,
    Transaction,
    Wallets as SdkWallets,
} from "comit-sdk";
import { Logger } from "log4js";
import { E2ETestActorConfig } from "../config";
import { Asset, AssetKind, toKey } from "../asset";
import { CndInstance } from "../cnd/cnd_instance";
import { Ledger, LedgerKind } from "../ledgers/ledger";
import { LedgerConfig, sleep } from "../utils";
import { Actors } from "./index";
import { Wallets } from "../wallets";
import { sha256 } from "js-sha256";
import { InvoiceState } from "@radar/lnrpc";
import { defaultLedgerDescriptionForLedger } from "./defaults";

export type ActorNames = "alice" | "bob" | "charlie";

export class Actor {
    public static defaultActionConfig = {
        maxTimeoutSecs: 20,
        tryIntervalSecs: 1,
    };

    public static async newInstance(
        name: ActorNames,
        ledgerConfig: LedgerConfig,
        cargoTargetDirectory: string,
        cndLogFile: string,
        logger: Logger
    ) {
        const actorConfig = await E2ETestActorConfig.for(name, logger);
        const cndConfigFile = actorConfig.generateCndConfigFile(ledgerConfig);

        const cndInstance = new CndInstance(
            cargoTargetDirectory,
            cndLogFile,
            logger,
            cndConfigFile
        );

        await cndInstance.start();

        logger.info(
            "Created new actor with config %s",
            JSON.stringify(cndConfigFile)
        );

        return new Actor(logger, cndInstance, name);
    }

    public actors: Actors;
    public wallets: Wallets;

    readonly cnd: Cnd;
    public swap: Swap;

    public alphaLedger: Ledger;
    public alphaAsset: Asset;

    public betaLedger: Ledger;
    public betaAsset: Asset;

    public readonly startingBalances: Map<string, bigint>;
    public readonly expectedBalanceChanges: Map<string, bigint>;

    constructor(
        public readonly logger: Logger,
        public readonly cndInstance: CndInstance,
        private readonly name: ActorNames
    ) {
        this.wallets = new Wallets({});
        const socket = cndInstance.getConfigFile().http_api.socket;
        this.cnd = new Cnd(`http://${socket}`);

        this.startingBalances = new Map();
        this.expectedBalanceChanges = new Map();
    }

    public async createSwap(
        createSwapPayload: HanEthereumEtherHalightLightningBitcoinRequestBody
    ) {
        this.alphaLedger = {
            name: LedgerKind.Ethereum,
            chain_id: createSwapPayload.alpha.chain_id,
        };
        this.betaLedger = {
            name: LedgerKind.Lightning,
            network: createSwapPayload.beta.network,
        };
        this.alphaAsset = {
            name: AssetKind.Ether,
            quantity: createSwapPayload.alpha.amount,
            ledger: LedgerKind.Ethereum,
        };
        this.betaAsset = {
            name: AssetKind.Bitcoin,
            quantity: createSwapPayload.beta.amount,
            ledger: LedgerKind.Lightning,
        };

        switch (this.name) {
            case "alice": {
                // Alice purchases beta asset with alpha asset
                await this.setStartingBalance([
                    this.alphaAsset,
                    {
                        ...this.betaAsset,
                        quantity: "0",
                    },
                ]);
                this.expectedBalanceChanges.set(
                    toKey(this.betaAsset),
                    BigInt(this.betaAsset.quantity)
                );
                break;
            }
            case "bob": {
                // Bob purchases alpha asset with beta asset
                await this.setStartingBalance([
                    this.betaAsset,
                    {
                        ...this.alphaAsset,
                        quantity: "0",
                    },
                ]);
                this.expectedBalanceChanges.set(
                    toKey(this.alphaAsset),
                    BigInt(this.alphaAsset.quantity)
                );
                break;
            }
            default: {
                throw new Error(
                    `createSwap does not support the actor ${this.name} yet`
                );
            }
        }

        const location = await this.cnd.createHanEthereumEtherHalightLightningBitcoin(
            createSwapPayload
        );

        this.swap = new Swap(
            this.cnd,
            location,
            new SdkWallets({
                ethereum: this.wallets.ethereum.inner,
                lightning: this.wallets.lightning.inner,
            })
        );
    }

    public async init(config?: {
        maxTimeoutSecs: number;
        tryIntervalSecs: number;
    }) {
        if (!this.swap) {
            throw new Error("Cannot init nonexistent swap");
        }

        const response = await this.swap.tryExecuteSirenAction<LedgerAction>(
            "init",
            config ? config : Actor.defaultActionConfig
        );
        await this.swap.doLedgerAction(response.data);
    }

    public async fund(config?: {
        maxTimeoutSecs: number;
        tryIntervalSecs: number;
    }) {
        if (!this.swap) {
            throw new Error("Cannot fund nonexistent swap");
        }

        const txid = await this.swap.fund(
            config ? config : Actor.defaultActionConfig
        );

        if (txid instanceof Transaction) {
            await txid.status(1);
        }

        this.logger.debug("Funded swap %s in %s", this.swap.self, txid);

        const role = await this.whoAmI();
        switch (role) {
            case "Alice":
                await this.actors.alice.assertAlphaFunded();
                if (this.actors.bob.cndInstance.isRunning()) {
                    await this.actors.bob.assertAlphaFunded();
                }
                break;
            case "Bob":
                if (this.actors.alice.cndInstance.isRunning()) {
                    await this.actors.alice.assertBetaFunded();
                }
                await this.actors.bob.assertBetaFunded();
                break;
        }
    }

    public async redeem() {
        if (!this.swap) {
            throw new Error("Cannot redeem non-existent swap");
        }

        const txid = await this.swap.redeem(Actor.defaultActionConfig);
        this.logger.debug("Redeemed swap %s in %s", this.swap.self, txid);

        const role = await this.whoAmI();
        switch (role) {
            case "Alice":
                await this.actors.alice.assertBetaRedeemed();
                if (this.actors.bob.cndInstance.isRunning()) {
                    await this.actors.bob.assertBetaRedeemed();
                }
                break;
            case "Bob":
                if (this.actors.alice.cndInstance.isRunning()) {
                    await this.actors.alice.assertAlphaRedeemed();
                }
                await this.actors.bob.assertAlphaRedeemed();
                break;
        }
    }

    public async assertSwapped() {
        this.logger.debug("Checking if cnd reports status 'SWAPPED'");

        while (true) {
            await sleep(200);
            const entity = await this.swap.fetchDetails();
            if (entity.properties.status === "SWAPPED") {
                break;
            }
        }

        await this.assertBalances();
    }

    public async assertBalances() {
        // TODO: WTF is this checking for halight?
    }

    public async assertAlphaFunded() {
        // TODO: Actually assert
    }

    public async assertBetaFunded() {
        // TODO: Actually assert
    }

    public async assertAlphaRedeemed() {
        // TODO: Actually assert
    }

    public async assertBetaRedeemed() {
        // TODO: Actually assert
    }

    public async start() {
        await this.cndInstance.start();
    }

    public async stop() {
        this.logger.debug("Stopping actor");
        this.cndInstance.stop();
    }

    public async dumpState() {
        // TODO: Actually dump a split protocol state
    }

    // TODO: Check if this correct or if we can use getName
    public async whoAmI(): Promise<string> {
        const entity = await this.swap.fetchDetails();
        return entity.properties.role;
    }

    public getName() {
        return this.name;
    }

    public async setStartingBalance(assets: Asset[]) {
        for (const asset of assets) {
            if (parseFloat(asset.quantity) === 0) {
                this.startingBalances.set(toKey(asset), BigInt(0));
                continue;
            }

            const ledger = defaultLedgerDescriptionForLedger(asset.ledger);
            const ledgerName = ledger.name;

            this.logger.debug("Minting %s on %s", asset.name, ledgerName);

            await this.wallets.getWalletForLedger(ledgerName).mint(asset);

            const balance = await this.wallets[ledgerName].getBalanceByAsset(
                asset
            );

            this.logger.debug(
                "Starting %s balance: ",
                asset.name,
                balance.toString()
            );
            this.startingBalances.set(
                toKey(asset),
                BigInt(balance.toString(10))
            );
        }
    }

    public cndHttpApiUrl() {
        const socket = this.cndInstance.getConfigFile().http_api.socket;
        return `http://${socket}`;
    }

    public async pollCndUntil(
        location: string,
        // TODO: Use the correct type
        predicate: (body: Entity) => boolean
    ): Promise<Entity> {
        const response = await this.cnd.fetch(location);

        expect(response.status).toEqual(200);

        if (predicate(response.data)) {
            return response.data;
        } else {
            await sleep(500);

            return this.pollCndUntil(location, predicate);
        }
    }

    // TODO: Use the correct type
    public async pollSwapDetails(
        swapUrl: string,
        iteration: number = 0
    ): Promise<SwapDetails> {
        if (iteration > 5) {
            throw new Error(`Could not retrieve Swap ${swapUrl}`);
        }
        iteration++;

        try {
            return (await this.cnd.fetch<SwapDetails>(swapUrl)).data;
        } catch (error) {
            await sleep(1000);
            return this.pollSwapDetails(swapUrl, iteration);
        }
    }

    /// TODO: Remove once all asserts are in place
    public lnCreateSha256Secret(): { secret: string; secretHash: string } {
        const secretBuf = Buffer.alloc(32);
        for (let i = 0; i < secretBuf.length; i++) {
            secretBuf[i] = Math.floor(Math.random() * 255);
        }

        const secretHash = sha256(secretBuf);
        const secret = secretBuf.toString("hex");
        this.logger.debug(`LN: secret: ${secret}, secretHash: ${secretHash}`);
        return { secret, secretHash };
    }

    public async lnCreateHoldInvoice(
        sats: string,
        secretHash: string,
        expiry: number,
        cltvExpiry: number
    ): Promise<void> {
        this.logger.debug("LN: Create Hold Invoice", sats, secretHash, expiry);
        const resp = await this.wallets.lightning.inner.addHoldInvoice(
            sats,
            secretHash,
            expiry,
            cltvExpiry
        );
        this.logger.debug("LN: Create Hold Response:", resp);
    }

    public async lnSendPayment(
        to: Actor,
        satAmount: string,
        secretHash: string,
        finalCltvDelta: number
    ) {
        const toPubkey = await to.wallets.lightning.inner.getPubkey();
        this.logger.debug(
            "LN: Send Payment -",
            "to:",
            toPubkey,
            "; amt:",
            satAmount,
            "; hash:",
            secretHash,
            "; finalCltvDelta: ",
            finalCltvDelta
        );
        return this.wallets.lightning.inner.sendPayment(
            toPubkey,
            satAmount,
            secretHash,
            finalCltvDelta
        );
    }

    /** Settles the invoice once it is `accepted`.
     *
     * When the other party sends the payment, the invoice status changes
     * from `open` to `accepted`. Hence, we check first if the invoice is accepted
     * with `lnAssertInvoiceAccepted`. If it throws, then we sleep 100ms and recursively
     * call `lnSettleInvoice` (this function).
     * If `lnAssertInvoiceAccepted` does not throw then it means the payment has been received
     * and we proceed with the settlement.
     */
    public async lnSettleInvoice(secret: string, secretHash: string) {
        try {
            await this.lnAssertInvoiceAccepted(secretHash);
            this.logger.debug("LN: Settle Invoice", secret, secretHash);
            await this.wallets.lightning.inner.settleInvoice(secret);
        } catch {
            await sleep(100);
            await this.lnSettleInvoice(secret, secretHash);
        }
    }

    public async lnCreateInvoice(sats: string) {
        this.logger.debug(`Creating invoice for ${sats} sats`);
        return this.wallets.lightning.addInvoice(sats);
    }

    public async lnPayInvoiceWithRequest(request: string): Promise<void> {
        this.logger.debug(`Paying invoice with request ${request}`);
        await this.wallets.lightning.pay(request);
    }

    public async lnAssertInvoiceSettled(secretHash: string) {
        const resp = await this.wallets.lightning.lookupInvoice(secretHash);
        if (resp.state !== InvoiceState.SETTLED) {
            throw new Error(
                `Invoice ${secretHash} is not settled, status is ${resp.state}`
            );
        }
    }

    public async lnAssertInvoiceAccepted(secretHash: string) {
        const resp = await this.wallets.lightning.lookupInvoice(secretHash);
        if (resp.state !== InvoiceState.ACCEPTED) {
            throw new Error(
                `Invoice ${secretHash} is not accepted, status is ${resp.state}`
            );
        }
    }
}

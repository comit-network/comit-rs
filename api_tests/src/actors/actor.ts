import { expect } from "chai";
import {
    BigNumber,
    Cnd,
    ComitClient,
    Entity,
    LedgerAction,
    Swap,
    SwapDetails,
    TransactionStatus,
    Transaction,
} from "comit-sdk";
import { Logger } from "log4js";
import { E2ETestActorConfig } from "../config";
import "../setup_chai";
import { Asset, AssetKind, toKey, toKind } from "../asset";
import { CndInstance } from "../cnd/cnd_instance";
import { Ledger, LedgerKind } from "../ledgers/ledger";
import { LedgerConfig, sleep } from "../utils";
import { Wallet, Wallets } from "../wallets";
import { Actors } from "./index";
import { sha256 } from "js-sha256";
import { InvoiceState } from "@radar/lnrpc";
import {
    defaultAssetDescription,
    defaultExpiryTimes,
    defaultLedgerDescriptionForLedger,
    defaultLedgerKindForAsset,
} from "./defaults";

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
        const actorConfig = await E2ETestActorConfig.for(name);
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

    private comitClient: ComitClient;
    readonly cnd: Cnd;
    private swap: Swap;

    private alphaLedger: Ledger;
    private alphaAsset: Asset;

    private betaLedger: Ledger;
    private betaAsset: Asset;

    private readonly startingBalances: Map<string, BigNumber>;
    private readonly expectedBalanceChanges: Map<string, BigNumber>;

    constructor(
        private readonly logger: Logger,
        private readonly cndInstance: CndInstance,
        private readonly name: ActorNames
    ) {
        this.wallets = new Wallets({});
        const socket = cndInstance.getConfigFile().http_api.socket;
        this.cnd = new Cnd(`http://${socket}`);

        this.startingBalances = new Map();
        this.expectedBalanceChanges = new Map();
    }

    public async sendRequest(
        maybeAlpha?: AssetKind | { ledger: LedgerKind; asset: AssetKind },
        maybeBeta?: AssetKind | { ledger: LedgerKind; asset: AssetKind }
    ) {
        this.logger.info("Sending swap request");

        // By default, we will send the swap request to bob
        const to = this.actors.bob;

        let alphaAssetKind: AssetKind;
        let alphaLedgerKind: LedgerKind;
        if (!maybeAlpha) {
            alphaAssetKind = this.defaultAlphaAssetKind();
            alphaLedgerKind = this.defaultAlphaLedgerKind();
        } else if (typeof maybeAlpha === "string") {
            alphaAssetKind = maybeAlpha;
            alphaLedgerKind = defaultLedgerKindForAsset(alphaAssetKind);
        } else {
            alphaAssetKind = maybeAlpha.asset;
            alphaLedgerKind = maybeAlpha.ledger;
        }

        this.alphaLedger = defaultLedgerDescriptionForLedger(alphaLedgerKind);
        this.alphaAsset = defaultAssetDescription(
            alphaAssetKind,
            alphaLedgerKind
        );
        to.alphaLedger = this.alphaLedger;
        to.alphaAsset = this.alphaAsset;

        this.logger.debug(
            "Derived Alpha Ledger %o from %s",
            this.alphaLedger,
            alphaLedgerKind
        );
        this.logger.debug(
            "Derived Alpha Asset %o from %s",
            this.alphaAsset,
            alphaAssetKind
        );

        let betaAssetKind;
        let betaLedgerKind;
        if (!maybeBeta) {
            betaAssetKind = this.defaultBetaAssetKind();
            betaLedgerKind = this.defaultBetaLedgerKind();
        } else if (typeof maybeBeta === "string") {
            betaAssetKind = maybeBeta;
            betaLedgerKind = defaultLedgerKindForAsset(betaAssetKind);
        } else {
            betaAssetKind = maybeBeta.asset;
            betaLedgerKind = maybeBeta.ledger;
        }

        this.betaLedger = defaultLedgerDescriptionForLedger(betaLedgerKind);
        this.betaAsset = defaultAssetDescription(betaAssetKind, betaLedgerKind);
        to.betaLedger = this.betaLedger;
        to.betaAsset = this.betaAsset;

        this.logger.debug(
            "Derived Beta Ledger %o from %s",
            this.betaLedger,
            betaLedgerKind
        );
        this.logger.debug(
            "Derived Beta Asset %o from %s",
            this.betaAsset,
            betaAssetKind
        );

        const listPromises: Promise<void>[] = [
            this.initializeDependencies(),
            to.initializeDependencies(),
        ];
        await Promise.all(listPromises);

        await this.setStartingBalance([
            this.alphaAsset,
            {
                name: this.betaAsset.name,
                ledger: this.betaLedger.name,
                quantity: "0",
            },
        ]);
        await to.setStartingBalance([
            {
                name: to.alphaAsset.name,
                ledger: this.alphaLedger.name,
                quantity: "0",
            },
            to.betaAsset,
        ]);

        this.expectedBalanceChanges.set(
            toKey(this.betaAsset),
            new BigNumber(this.betaAsset.quantity)
        );
        to.expectedBalanceChanges.set(
            toKey(this.alphaAsset),
            new BigNumber(to.alphaAsset.quantity)
        );

        const isLightning =
            this.alphaLedger.name === "lightning" ||
            this.betaLedger.name === "lightning";

        if (isLightning) {
            this.logger.debug("Using lightning routes on cnd REST API");
            return;
        }
        const comitClient: ComitClient = this.getComitClient();

        const payload = {
            alpha_ledger: this.alphaLedger,
            beta_ledger: this.betaLedger,
            alpha_asset: this.alphaAsset,
            beta_asset: this.betaAsset,
            peer: {
                peer_id: await to.cnd.getPeerId(),
                address_hint: await to.cnd
                    .getPeerListenAddresses()
                    .then((addresses) => addresses[0]),
            },
            ...(await this.additionalIdentities(alphaAssetKind, betaAssetKind)),
            ...defaultExpiryTimes(),
        };

        this.swap = await comitClient.sendSwap(payload);
        to.swap = new Swap(to.cnd, this.swap.self, {
            bitcoin: to.wallets.bitcoin.inner,
            ethereum: to.wallets.ethereum.inner,
        });
        this.logger.debug("Created new swap at %s", this.swap.self);

        return this.swap;
    }

    public async accept() {
        if (!this.swap) {
            throw new Error("Cannot accept non-existent swap");
        }

        await this.swap.accept(Actor.defaultActionConfig);
    }

    public async deploy() {
        if (!this.swap) {
            throw new Error("Cannot deploy htlc for nonexistent swap");
        }

        const transaction = await this.swap.deploy(Actor.defaultActionConfig);
        let transactionId;
        if (transaction instanceof Transaction) {
            const status = await transaction.status(1);
            transactionId = transaction.id;
            if (status === TransactionStatus.Failed) {
                throw new Error(`Transaction ${transactionId} failed`);
            }
        } else {
            transactionId = transaction;
        }

        this.logger.debug(
            "Deployed htlc for swap %s in %s",
            this.swap.self,
            transactionId
        );

        const entity = await this.swap.fetchDetails();
        switch (entity.properties.role) {
            case "Alice":
                await this.actors.alice.assertAlphaDeployed();
                if (this.actors.bob.cndInstance.isRunning()) {
                    await this.actors.bob.assertAlphaDeployed();
                }
                break;
            case "Bob":
                if (this.actors.alice.cndInstance.isRunning()) {
                    await this.actors.alice.assertBetaDeployed();
                }
                await this.actors.bob.assertBetaDeployed();
                break;
        }
    }

    public async fund() {
        if (!this.swap) {
            throw new Error("Cannot fund nonexistent swap");
        }

        const txid = await this.swap.fund(Actor.defaultActionConfig);
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

    public async fundLowGas(hexGasLimit: string) {
        const response = await this.swap.tryExecuteSirenAction<LedgerAction>(
            "fund",
            {
                maxTimeoutSecs: 10,
                tryIntervalSecs: 1,
            }
        );
        response.data.payload.gas_limit = hexGasLimit;
        const transaction = await this.swap.doLedgerAction(response.data);
        if (transaction instanceof Transaction) {
            const status = await transaction.status(1);
            if (status !== TransactionStatus.Failed) {
                throw new Error(
                    "Deploy with low gas transaction was successful."
                );
            }
        } else {
            throw new Error(
                "Internal error: Transaction class expected for Ethereum."
            );
        }
    }

    public async overfund() {
        const response = await this.swap.tryExecuteSirenAction<LedgerAction>(
            "fund",
            {
                maxTimeoutSecs: 10,
                tryIntervalSecs: 1,
            }
        );
        const amount = response.data.payload.amount;
        const overfundAmount = amount * 1.01;

        response.data.payload.amount = overfundAmount;

        const txid = await this.swap.doLedgerAction(response.data);
        this.logger.debug(
            "Overfunded swap %s in %s with %d instead of %d",
            this.swap.self,
            txid,
            overfundAmount,
            amount
        );
    }

    public async underfund() {
        const response = await this.swap.tryExecuteSirenAction<LedgerAction>(
            "fund",
            {
                maxTimeoutSecs: 10,
                tryIntervalSecs: 1,
            }
        );
        const amount = response.data.payload.amount;
        const underfundAmount = amount * 0.01;

        response.data.payload.amount = underfundAmount;

        const txid = await this.swap.doLedgerAction(response.data);
        this.logger.debug(
            "Underfunded swap %s in %s with %d instead of %d",
            this.swap.self,
            txid,
            underfundAmount,
            amount
        );
    }

    public async refund() {
        if (!this.swap) {
            throw new Error("Cannot refund non-existent swap");
        }

        const role = await this.whoAmI();
        switch (role) {
            case "Alice":
                await this.waitForAlphaExpiry();
                break;
            case "Bob":
                await this.waitForBetaExpiry();
                break;
        }

        const txid = await this.swap.refund(Actor.defaultActionConfig);
        this.logger.debug("Refunded swap %s in %s", this.swap.self, txid);

        switch (role) {
            case "Alice":
                await this.actors.alice.assertAlphaRefunded();
                if (this.actors.bob.cndInstance.isRunning()) {
                    await this.actors.bob.assertAlphaRefunded();
                }
                break;
            case "Bob":
                if (this.actors.alice.cndInstance.isRunning()) {
                    await this.actors.alice.assertBetaRefunded();
                }
                await this.actors.bob.assertBetaRefunded();
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

    public async redeemWithHighFee() {
        // Hack the bitcoin fee per WU returned by the wallet
        this.wallets.bitcoin.inner.getFee = () => "100000000";

        return this.swap.tryExecuteSirenAction<LedgerAction>("redeem", {
            maxTimeoutSecs: 10,
            tryIntervalSecs: 1,
        });
    }

    public async currentSwapIsAccepted() {
        let swapEntity;

        do {
            swapEntity = await this.swap.fetchDetails();

            await sleep(200);
        } while (
            swapEntity.properties.state.communication.status !== "ACCEPTED"
        );
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

        for (const [
            assetKey,
            expectedBalanceChange,
        ] of this.expectedBalanceChanges.entries()) {
            this.logger.debug(
                "Checking that %s balance changed by %d",
                assetKey,
                expectedBalanceChange
            );

            const { asset, ledger } = toKind(assetKey);

            const wallet = this.wallets[ledger];
            const expectedBalance = new BigNumber(
                this.startingBalances.get(assetKey)
            ).plus(expectedBalanceChange);
            const maximumFee = wallet.MaximumFee;

            const balanceInclFees = expectedBalance.minus(maximumFee);

            const currentWalletBalance = await wallet.getBalanceByAsset(
                defaultAssetDescription(asset, ledger)
            );

            expect(currentWalletBalance).to.be.bignumber.gte(balanceInclFees);

            this.logger.debug(
                "Balance check was positive, current balance is %d",
                currentWalletBalance
            );
        }
    }

    public async assertRefunded() {
        this.logger.debug("Checking if swap @ %s was refunded", this.swap.self);

        for (const [assetKey] of this.startingBalances.entries()) {
            const { asset, ledger } = toKind(assetKey);

            const wallet = this.wallets[ledger];
            const maximumFee = wallet.MaximumFee;

            this.logger.debug(
                "Checking that %s balance changed by max %d (MaximumFee)",
                assetKey,
                maximumFee
            );
            const expectedBalance = new BigNumber(
                this.startingBalances.get(assetKey)
            );
            const currentWalletBalance = await wallet.getBalanceByAsset(
                defaultAssetDescription(asset, ledger)
            );
            const balanceInclFees = expectedBalance.minus(maximumFee);
            expect(currentWalletBalance).to.be.bignumber.gte(balanceInclFees);
        }
    }

    public async assertAlphaDeployed() {
        await this.assertLedgerState("alpha_ledger", "DEPLOYED");
    }

    public async assertBetaDeployed() {
        await this.assertLedgerState("beta_ledger", "DEPLOYED");
    }

    public async assertAlphaFunded() {
        await this.assertLedgerState("alpha_ledger", "FUNDED");
    }

    public async assertBetaFunded() {
        await this.assertLedgerState("beta_ledger", "FUNDED");
    }

    public async assertAlphaRedeemed() {
        await this.assertLedgerState("alpha_ledger", "REDEEMED");
    }

    public async assertBetaRedeemed() {
        await this.assertLedgerState("beta_ledger", "REDEEMED");
    }

    public async assertAlphaRefunded() {
        await this.assertLedgerState("alpha_ledger", "REFUNDED");
    }

    public async assertBetaRefunded() {
        await this.assertLedgerState("beta_ledger", "REFUNDED");
    }

    public async assertAlphaIncorrectlyFunded() {
        await this.assertLedgerState("alpha_ledger", "INCORRECTLY_FUNDED");
    }

    public async assertBetaIncorrectlyFunded() {
        await this.assertLedgerState("beta_ledger", "INCORRECTLY_FUNDED");
    }

    public async assertAlphaNotDeployed() {
        await sleep(3000); // It is meaningless to assert before cnd processes a new block
        await this.assertLedgerState("alpha_ledger", "NOT_DEPLOYED");
    }

    public async assertBetaNotDeployed() {
        await sleep(3000); // It is meaningless to assert before cnd processes a new block
        await this.assertLedgerState("beta_ledger", "NOT_DEPLOYED");
    }

    public async start() {
        await this.cndInstance.start();
    }

    public async stop() {
        this.logger.debug("Stopping actor");
        this.cndInstance.stop();
    }

    public async restart() {
        await this.stop();
        await this.start();
    }

    public async dumpState() {
        this.logger.debug("dumping current state");

        if (this.swap) {
            const swapDetails = await this.swap.fetchDetails();

            this.logger.debug("swap status: %s", swapDetails.properties.status);
            this.logger.debug("swap details: ", JSON.stringify(swapDetails));

            this.logger.debug(
                "alpha ledger wallet balance %d",
                await this.alphaLedgerWallet().getBalanceByAsset(
                    this.alphaAsset
                )
            );
            this.logger.debug(
                "beta ledger wallet balance %d",
                await this.betaLedgerWallet().getBalanceByAsset(this.betaAsset)
            );
        }
    }

    public async whoAmI() {
        const entity = await this.swap.fetchDetails();
        return entity.properties.role;
    }

    public getName() {
        return this.name;
    }

    private async waitForAlphaExpiry() {
        const swapDetails = await this.swap.fetchDetails();

        const expiry = swapDetails.properties.state.communication.alpha_expiry;
        const wallet = this.alphaLedgerWallet();

        await this.waitForExpiry(wallet, expiry);
    }

    private async waitForBetaExpiry() {
        const swapDetails = await this.swap.fetchDetails();

        const expiry = swapDetails.properties.state.communication.beta_expiry;
        const wallet = this.betaLedgerWallet();

        await this.waitForExpiry(wallet, expiry);
    }

    private alphaLedgerWallet() {
        return this.wallets.getWalletForLedger(this.alphaLedger.name);
    }

    private betaLedgerWallet() {
        return this.wallets.getWalletForLedger(this.betaLedger.name);
    }

    private async waitForExpiry(wallet: Wallet, expiry: number) {
        let currentBlockchainTime = await wallet.getBlockchainTime();

        this.logger.debug(
            `Current blockchain time is ${currentBlockchainTime}`
        );

        let diff = expiry - currentBlockchainTime;

        if (diff > 0) {
            this.logger.debug(`Waiting for blockchain time to pass ${expiry}`);

            while (diff > 0) {
                await sleep(1000);

                currentBlockchainTime = await wallet.getBlockchainTime();
                diff = expiry - currentBlockchainTime;

                this.logger.debug(
                    `Current blockchain time is ${currentBlockchainTime}`
                );
            }
        }
    }

    private async assertLedgerState(
        ledger: string,
        status:
            | "NOT_DEPLOYED"
            | "DEPLOYED"
            | "FUNDED"
            | "REDEEMED"
            | "REFUNDED"
            | "INCORRECTLY_FUNDED"
    ) {
        this.logger.debug(
            "Waiting for cnd to see %s in state %s for swap @ %s",
            ledger,
            status,
            this.swap.self
        );

        let swapEntity;

        do {
            swapEntity = await this.swap.fetchDetails();

            await sleep(200);
        } while (swapEntity.properties.state[ledger].status !== status);

        this.logger.debug(
            "cnd saw %s in state %s for swap @ %s",
            ledger,
            status,
            this.swap.self
        );
    }

    private async additionalIdentities(
        alphaAsset: AssetKind,
        betaAsset: AssetKind
    ) {
        if (alphaAsset === "bitcoin" && betaAsset === "ether") {
            return {
                beta_ledger_redeem_identity: this.wallets.ethereum.account(),
            };
        }

        return {};
    }

    private async initializeDependencies() {
        const lightningNeeded =
            this.alphaLedger.name === "lightning" ||
            this.betaLedger.name === "lightning";

        const walletPromises: Promise<void>[] = [];
        for (const ledgerName of [
            this.alphaLedger.name,
            this.betaLedger.name,
        ]) {
            walletPromises.push(
                this.wallets.initializeForLedger(
                    ledgerName,
                    this.logger,
                    this.name
                )
            );
        }

        await Promise.all(walletPromises);

        if (!lightningNeeded) {
            this.comitClient = new ComitClient(this.cnd)
                .withBitcoinWallet(
                    this.wallets.getWalletForLedger("bitcoin").inner
                )
                .withEthereumWallet(
                    this.wallets.getWalletForLedger("ethereum").inner
                );
        }
    }

    private getComitClient(): ComitClient {
        if (!this.comitClient) {
            throw new Error("ComitClient is not initialised");
        }

        return this.comitClient;
    }

    private async setStartingBalance(assets: Asset[]) {
        for (const asset of assets) {
            if (parseFloat(asset.quantity) === 0) {
                this.startingBalances.set(toKey(asset), new BigNumber(0));
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
            this.startingBalances.set(toKey(asset), balance);
        }
    }

    private defaultAlphaAssetKind() {
        const defaultAlphaAssetKind = AssetKind.Bitcoin;
        this.logger.info(
            "AssetKind for alpha asset not specified, defaulting to %s",
            defaultAlphaAssetKind
        );

        return defaultAlphaAssetKind;
    }

    private defaultAlphaLedgerKind() {
        const defaultAlphaLedgerKind = LedgerKind.Bitcoin;
        this.logger.info(
            "LedgerKind for alpha ledger not specified, defaulting to %s",
            defaultAlphaLedgerKind
        );

        return defaultAlphaLedgerKind;
    }

    private defaultBetaAssetKind() {
        const defaultBetaAssetKind = AssetKind.Ether;
        this.logger.info(
            "AssetKind for beta asset not specified, defaulting to %s",
            defaultBetaAssetKind
        );

        return defaultBetaAssetKind;
    }

    private defaultBetaLedgerKind() {
        const defaultBetaLedgerKind = LedgerKind.Ethereum;
        this.logger.info(
            "LedgerKind for beta ledger not specified, defaulting to %s",
            defaultBetaLedgerKind
        );

        return defaultBetaLedgerKind;
    }

    public cndHttpApiUrl() {
        const socket = this.cndInstance.getConfigFile().http_api.socket;
        return `http://${socket}`;
    }

    public async pollCndUntil(
        location: string,
        predicate: (body: Entity) => boolean
    ): Promise<Entity> {
        const response = await this.cnd.fetch(location);

        expect(response).to.have.status(200);

        if (predicate(response.data)) {
            return response.data;
        } else {
            await sleep(500);

            return this.pollCndUntil(location, predicate);
        }
    }

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

    /// This is to be removed once cnd supports lightning
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
        const resp = await this.wallets.lightning.inner.sendPayment(
            toPubkey,
            satAmount,
            secretHash,
            finalCltvDelta
        );
        this.logger.debug("LN: Send Payment Response:", resp);
        return resp;
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

import { Actor } from "./actor";
import { AssetKind, defaultAssetValue, toKey, toKind } from "../asset";
import { LedgerKind } from "../ledgers/ledger";
import {
    defaultExpiryTimes,
    defaultLedgerDescriptionForLedger,
    defaultLedgerKindForAsset,
} from "./rfc003_defaults";
import { Logger } from "log4js";
import { Rfc003Actors } from "./index";
import {
    ComitClient,
    siren,
    LedgerAction,
    Swap,
    SwapDetails,
    Transaction,
    TransactionStatus,
} from "comit-sdk";
import { Wallet } from "../wallets";
import { sleep } from "../utils";

/**
 * Actor class that contains all methods specific to rfc003 and do not apply to the split protocols (han, halight, herc20)
 */
export class Rfc003Actor {
    private readonly logger: Logger;
    public actors: Rfc003Actors;
    private comitClient: ComitClient;

    public static convert(actors: Actor[]): Rfc003Actor[] {
        const rfc003ActorsMap = new Map<string, Rfc003Actor>();

        for (const actor of actors) {
            rfc003ActorsMap.set(actor.name, new Rfc003Actor(actor));
        }

        const rfc003Actors = new Rfc003Actors(rfc003ActorsMap);

        const result: Rfc003Actor[] = [];

        for (const actor of actors) {
            const name = actor.name;
            rfc003ActorsMap.get(name).actors = rfc003Actors;
            result.push(rfc003Actors.getActorByName(name));
        }

        return result;
    }

    private constructor(public actor: Actor) {
        this.logger = actor.logger;
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

        this.actor.alphaLedger = defaultLedgerDescriptionForLedger(
            alphaLedgerKind
        );
        this.actor.alphaAsset = defaultAssetValue(
            alphaAssetKind,
            alphaLedgerKind
        );
        to.actor.alphaLedger = this.actor.alphaLedger;
        to.actor.alphaAsset = this.actor.alphaAsset;

        this.logger.debug(
            "Derived Alpha Ledger %o from %s",
            this.actor.alphaLedger,
            alphaLedgerKind
        );
        this.logger.debug(
            "Derived Alpha Asset %o from %s",
            this.actor.alphaAsset,
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

        this.actor.betaLedger = defaultLedgerDescriptionForLedger(
            betaLedgerKind
        );
        this.actor.betaAsset = defaultAssetValue(betaAssetKind, betaLedgerKind);
        to.actor.betaLedger = this.actor.betaLedger;
        to.actor.betaAsset = this.actor.betaAsset;

        this.logger.debug(
            "Derived Beta Ledger %o from %s",
            this.actor.betaLedger,
            betaLedgerKind
        );
        this.logger.debug(
            "Derived Beta Asset %o from %s",
            this.actor.betaAsset,
            betaAssetKind
        );

        const listPromises: Promise<void>[] = [
            this.initializeDependencies(),
            to.initializeDependencies(),
        ];
        await Promise.all(listPromises);

        await this.actor.setStartingBalance([
            this.actor.alphaAsset,
            {
                name: this.actor.betaAsset.name,
                ledger: this.actor.betaLedger.name,
                quantity: "0",
            },
        ]);
        await to.actor.setStartingBalance([
            {
                name: to.actor.alphaAsset.name,
                ledger: this.actor.alphaLedger.name,
                quantity: "0",
            },
            to.actor.betaAsset,
        ]);

        this.actor.expectedBalanceChanges.set(
            toKey(this.actor.betaAsset),
            BigInt(this.actor.betaAsset.quantity)
        );
        to.actor.expectedBalanceChanges.set(
            toKey(this.actor.alphaAsset),
            BigInt(to.actor.alphaAsset.quantity)
        );

        const isLightning =
            this.actor.alphaLedger.name === "lightning" ||
            this.actor.betaLedger.name === "lightning";

        if (isLightning) {
            this.logger.debug("Using lightning routes on cnd REST API");
            return;
        }

        const comitClient: ComitClient = this.getComitClient();

        const payload = {
            alpha_ledger: this.actor.alphaLedger,
            beta_ledger: this.actor.betaLedger,
            alpha_asset: {
                name: this.actor.alphaAsset.name,
                quantity: this.actor.alphaAsset.quantity,
                token_contract: this.actor.alphaAsset.tokenContract,
            },
            beta_asset: {
                name: this.actor.betaAsset.name,
                quantity: this.actor.betaAsset.quantity,
                token_contract: this.actor.betaAsset.tokenContract,
            },
            peer: {
                peer_id: await to.actor.cnd.getPeerId(),
                address_hint: await to.actor.cnd
                    .getPeerListenAddresses()
                    .then((addresses) => addresses[0]),
            },
            ...(await this.additionalIdentities(alphaAssetKind, betaAssetKind)),
            ...defaultExpiryTimes(),
        };

        this.actor.swap = await comitClient.sendSwap(payload);
        to.actor.swap = new Swap(to.actor.cnd, this.actor.swap.self, {
            bitcoin: to.actor.wallets.bitcoin.inner,
            ethereum: to.actor.wallets.ethereum.inner,
        });
        this.logger.debug("Created new swap at %s", this.actor.swap.self);

        return this.actor.swap;
    }

    public async accept() {
        if (!this.actor.swap) {
            throw new Error("Cannot accept non-existent swap");
        }

        await this.actor.swap.accept(Actor.defaultActionConfig);
    }

    public async deploy() {
        if (!this.actor.swap) {
            throw new Error("Cannot deploy htlc for nonexistent swap");
        }

        const transaction = await this.actor.swap.deploy(
            Actor.defaultActionConfig
        );
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
            this.actor.swap.self,
            transactionId
        );

        const entity = await this.actor.swap.fetchDetails();
        switch (entity.properties.role) {
            case "Alice":
                await this.actors.alice.assertAlphaDeployed();
                if (this.actors.bob.actor.cndInstance.isRunning()) {
                    await this.actors.bob.assertAlphaDeployed();
                }
                break;
            case "Bob":
                if (this.actors.alice.actor.cndInstance.isRunning()) {
                    await this.actors.alice.assertBetaDeployed();
                }
                await this.actors.bob.assertBetaDeployed();
                break;
        }
    }

    public async fund(config?: {
        maxTimeoutSecs: number;
        tryIntervalSecs: number;
    }) {
        if (!this.actor.swap) {
            throw new Error("Cannot fund nonexistent swap");
        }

        const txid = await this.actor.swap.fund(
            config ? config : Actor.defaultActionConfig
        );

        if (txid instanceof Transaction) {
            await txid.status(1);
        }

        this.logger.debug("Funded swap %s in %s", this.actor.swap.self, txid);

        const role = await this.cryptoRole();
        switch (role) {
            case "Alice":
                await this.actors.alice.assertAlphaFunded();
                if (this.actors.bob.actor.cndInstance.isRunning()) {
                    await this.actors.bob.assertAlphaFunded();
                }
                break;
            case "Bob":
                if (this.actors.alice.actor.cndInstance.isRunning()) {
                    await this.actors.alice.assertBetaFunded();
                }
                await this.actors.bob.assertBetaFunded();
                break;
        }
    }

    public async fundLowGas(hexGasLimit: string) {
        const response = await this.actor.swap.tryExecuteSirenAction<
            LedgerAction
        >("fund", {
            maxTimeoutSecs: 10,
            tryIntervalSecs: 1,
        });
        response.data.payload.gas_limit = hexGasLimit;
        const transaction = await this.actor.swap.doLedgerAction(response.data);
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
        const response = await this.actor.swap.tryExecuteSirenAction<
            LedgerAction
        >("fund", {
            maxTimeoutSecs: 10,
            tryIntervalSecs: 1,
        });
        const amount = response.data.payload.amount;
        const overfundAmount = amount * 1.01;

        response.data.payload.amount = overfundAmount;

        const txid = await this.actor.swap.doLedgerAction(response.data);
        this.logger.debug(
            "Overfunded swap %s in %s with %d instead of %d",
            this.actor.swap.self,
            txid,
            overfundAmount,
            amount
        );
    }

    public async underfund() {
        const response = await this.actor.swap.tryExecuteSirenAction<
            LedgerAction
        >("fund", {
            maxTimeoutSecs: 10,
            tryIntervalSecs: 1,
        });
        const amount = response.data.payload.amount;
        const underfundAmount = amount * 0.01;

        response.data.payload.amount = underfundAmount;

        const txid = await this.actor.swap.doLedgerAction(response.data);
        this.logger.debug(
            "Underfunded swap %s in %s with %d instead of %d",
            this.actor.swap.self,
            txid,
            underfundAmount,
            amount
        );
    }

    public async refund() {
        if (!this.actor.swap) {
            throw new Error("Cannot refund non-existent swap");
        }

        const role = await this.cryptoRole();
        switch (role) {
            case "Alice":
                await this.waitForAlphaExpiry();
                break;
            case "Bob":
                await this.waitForBetaExpiry();
                break;
        }

        const txid = await this.actor.swap.refund(Actor.defaultActionConfig);
        this.logger.debug("Refunded swap %s in %s", this.actor.swap.self, txid);

        switch (role) {
            case "Alice":
                await this.actors.alice.assertAlphaRefunded();
                if (this.actors.bob.actor.cndInstance.isRunning()) {
                    await this.actors.bob.assertAlphaRefunded();
                }
                break;
            case "Bob":
                if (this.actors.alice.actor.cndInstance.isRunning()) {
                    await this.actors.alice.assertBetaRefunded();
                }
                await this.actors.bob.assertBetaRefunded();
                break;
        }
    }

    public async redeem() {
        if (!this.actor.swap) {
            throw new Error("Cannot redeem non-existent swap");
        }

        const txid = await this.actor.swap.redeem(Actor.defaultActionConfig);
        this.logger.debug("Redeemed swap %s in %s", this.actor.swap.self, txid);

        const role = await this.cryptoRole();
        switch (role) {
            case "Alice":
                await this.actors.alice.assertBetaRedeemed();
                if (this.actors.bob.actor.cndInstance.isRunning()) {
                    await this.actors.bob.assertBetaRedeemed();
                }
                break;
            case "Bob":
                if (this.actors.alice.actor.cndInstance.isRunning()) {
                    await this.actors.alice.assertAlphaRedeemed();
                }
                await this.actors.bob.assertAlphaRedeemed();
                break;
        }
    }

    public async redeemWithHighFee() {
        // Hack the bitcoin fee per WU returned by the wallet
        this.actor.wallets.bitcoin.inner.getFee = () => "100000000";

        return this.actor.swap.tryExecuteSirenAction<LedgerAction>("redeem", {
            maxTimeoutSecs: 10,
            tryIntervalSecs: 1,
        });
    }

    public async currentSwapIsAccepted() {
        let swapEntity;

        do {
            swapEntity = await this.actor.swap.fetchDetails();

            await sleep(200);
        } while (
            swapEntity.properties.state.communication.status !== "ACCEPTED"
        );
    }

    public async assertSwapped() {
        this.logger.debug("Checking if cnd reports status 'SWAPPED'");

        while (true) {
            await sleep(200);
            const entity = await this.actor.swap.fetchDetails();
            if (entity.properties.status === "SWAPPED") {
                break;
            }
        }

        await this.assertBalances();
    }

    public async assertBalances() {
        for (const [
            assetKey,
            expectedBalanceChange,
        ] of this.actor.expectedBalanceChanges.entries()) {
            this.logger.debug(
                "Checking that %s balance changed by %d",
                assetKey,
                expectedBalanceChange
            );

            const { asset, ledger } = toKind(assetKey);

            const wallet = this.actor.wallets[ledger];
            const expectedBalance =
                this.actor.startingBalances.get(assetKey) +
                expectedBalanceChange;
            const maximumFee = BigInt(wallet.MaximumFee);

            const balanceInclFees = expectedBalance - maximumFee;

            const currentWalletBalance = await wallet.getBalanceByAsset(
                defaultAssetValue(asset, ledger)
            );
            expect(currentWalletBalance).toBeGreaterThanOrEqual(
                balanceInclFees
            );

            this.logger.debug(
                "Balance check was positive, current balance is %d",
                currentWalletBalance
            );
        }
    }

    public async assertRefunded() {
        this.logger.debug(
            "Checking if swap @ %s was refunded",
            this.actor.swap.self
        );

        for (const [assetKey] of this.actor.startingBalances.entries()) {
            const { asset, ledger } = toKind(assetKey);

            const wallet = this.actor.wallets[ledger];
            const maximumFee = BigInt(wallet.MaximumFee);

            this.logger.debug(
                "Checking that %s balance changed by max %d (MaximumFee)",
                assetKey,
                maximumFee
            );
            const expectedBalance = this.actor.startingBalances.get(assetKey);
            const currentWalletBalance = await wallet.getBalanceByAsset(
                defaultAssetValue(asset, ledger)
            );
            const balanceInclFees = expectedBalance - maximumFee;
            expect(currentWalletBalance).toBeGreaterThanOrEqual(
                // @ts-ignore: Jest supports bigint, types to be fixed updated with
                // https://github.com/DefinitelyTyped/DefinitelyTyped/pull/44368
                balanceInclFees
            );
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
        await this.assertLedgerState("alpha_ledger", "NOT_DEPLOYED");
    }

    public async assertBetaNotDeployed() {
        await this.assertLedgerState("beta_ledger", "NOT_DEPLOYED");
    }

    public async start() {
        await this.actor.cndInstance.start();
    }

    public async stop() {
        this.logger.debug("Stopping actor");
        this.actor.cndInstance.stop();
    }

    public async dumpState() {
        this.logger.debug("dumping current state");

        if (this.actor.swap) {
            const swapDetails = await this.actor.swap.fetchDetails();

            this.logger.debug("swap status: %s", swapDetails.properties.status);
            this.logger.debug("swap details: ", JSON.stringify(swapDetails));

            this.logger.debug(
                "alpha ledger wallet balance %d",
                await this.actor.alphaLedgerWallet.getBalanceByAsset(
                    this.actor.alphaAsset
                )
            );
            this.logger.debug(
                "beta ledger wallet balance %d",
                await this.actor.betaLedgerWallet.getBalanceByAsset(
                    this.actor.betaAsset
                )
            );
        }
    }

    public async cryptoRole(): Promise<"Alice" | "Bob"> {
        return this.actor.swap
            .fetchDetails()
            .then((entity) => entity.properties.role);
    }

    get name() {
        return this.actor.name;
    }

    private async waitForAlphaExpiry() {
        const swapDetails = await this.actor.swap.fetchDetails();

        const expiry = swapDetails.properties.state.communication.alpha_expiry;
        const wallet = this.actor.alphaLedgerWallet;

        await this.waitForExpiry(wallet, expiry);
    }

    private async waitForBetaExpiry() {
        const swapDetails = await this.actor.swap.fetchDetails();

        const expiry = swapDetails.properties.state.communication.beta_expiry;
        const wallet = this.actor.betaLedgerWallet;

        await this.waitForExpiry(wallet, expiry);
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
        ledger: "alpha_ledger" | "beta_ledger",
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
            this.actor.swap.self
        );

        let swapEntity;

        do {
            swapEntity = await this.actor.swap.fetchDetails();

            await sleep(200);
        } while (swapEntity.properties.state[ledger].status !== status);

        this.logger.debug(
            "cnd saw %s in state %s for swap @ %s",
            ledger,
            status,
            this.actor.swap.self
        );
    }

    private async additionalIdentities(
        alphaAsset: AssetKind,
        betaAsset: AssetKind
    ) {
        if (alphaAsset === "bitcoin" && betaAsset === "ether") {
            return {
                beta_ledger_redeem_identity: this.actor.wallets.ethereum.account(),
            };
        }

        return {};
    }

    private async initializeDependencies() {
        const walletPromises: Promise<void>[] = [];
        for (const ledgerName of [
            this.actor.alphaLedger.name,
            this.actor.betaLedger.name,
        ]) {
            walletPromises.push(
                this.actor.wallets.initializeForLedger(
                    ledgerName,
                    this.logger,
                    this.actor.name
                )
            );
        }

        await Promise.all(walletPromises);

        this.comitClient = new ComitClient(this.actor.cnd)
            .withBitcoinWallet(
                this.actor.wallets.getWalletForLedger("bitcoin").inner
            )
            .withEthereumWallet(
                this.actor.wallets.getWalletForLedger("ethereum").inner
            );
    }

    private getComitClient(): ComitClient {
        if (!this.comitClient) {
            throw new Error("ComitClient is not initialised");
        }

        return this.comitClient;
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
        const socket = this.actor.cndInstance.getConfigFile().http_api.socket;
        return `http://${socket}`;
    }

    public async pollCndUntil(
        location: string,
        predicate: (body: siren.Entity) => boolean
    ): Promise<siren.Entity> {
        const response = await this.actor.cnd.fetch(location);

        expect(response.status).toEqual(200);

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
            return (await this.actor.cnd.fetch<SwapDetails>(swapUrl)).data;
        } catch (error) {
            await sleep(1000);
            return this.pollSwapDetails(swapUrl, iteration);
        }
    }
}

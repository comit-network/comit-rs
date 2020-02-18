import { expect } from "chai";
import { BigNumber, Cnd, ComitClient, Swap } from "comit-sdk";
import { parseEther } from "ethers/utils";
import getPort from "get-port";
import { Logger } from "log4js";
import { E2ETestActorConfig } from "../../lib/config";
import { LedgerConfig } from "../../lib/ledger_runner";
import "../../lib/setup_chai";
import { Asset, AssetKind } from "../asset";
import { CndInstance } from "../cnd_instance";
import { Lnd } from "../lnd";
import { Ledger, LedgerKind } from "../ledger";
import { sleep } from "../utils";
import { Wallet, Wallets } from "../wallets";
import { Actors } from "./index";
import { HarnessGlobal } from "../../lib/util";
import { Entity } from "../../gen/siren";
import { SwapDetails } from "comit-sdk/dist/src/cnd";

declare var global: HarnessGlobal;

export class Actor {
    public static defaultActionConfig = {
        maxTimeoutSecs: 20,
        tryIntervalSecs: 1,
    };

    public static async newInstance(
        loggerFactory: (name: string) => Logger,
        name: string,
        ledgerConfig: LedgerConfig,
        projectRoot: string,
        logRoot: string
    ) {
        const actorConfig = new E2ETestActorConfig(
            await getPort(),
            await getPort(),
            name,
            await getPort(),
            await getPort()
        );

        const cndInstance = new CndInstance(
            projectRoot,
            logRoot,
            actorConfig,
            ledgerConfig
        );

        await cndInstance.start();

        const logger = loggerFactory(name);
        logger.level = "debug";

        logger.info(
            "Created new actor with config %s",
            JSON.stringify(actorConfig.generateCndConfigFile(ledgerConfig))
        );

        return new Actor(actorConfig, logger, logRoot, cndInstance);
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

    private readonly startingBalances: Map<AssetKind, BigNumber>;
    private readonly expectedBalanceChanges: Map<AssetKind, BigNumber>;

    public lnd: Lnd;

    private constructor(
        private readonly config: E2ETestActorConfig,
        private readonly logger: Logger,
        private readonly logRoot: string,
        private readonly cndInstance: CndInstance
    ) {
        this.wallets = new Wallets({});
        const { address, port } = cndInstance.getConfigFile().http_api.socket;
        this.cnd = new Cnd(`http://${address}:${port}`);

        this.startingBalances = new Map();
        this.expectedBalanceChanges = new Map();
    }

    public async sendRequest(
        maybeAlphaAssetKind?: AssetKind,
        maybeBetaAssetKind?: AssetKind
    ) {
        const alphaAssetKind = maybeAlphaAssetKind
            ? maybeAlphaAssetKind
            : this.defaultAlphaAssetKind();
        const betaAssetKind = maybeBetaAssetKind
            ? maybeBetaAssetKind
            : this.defaultBetaAssetKind();

        // By default, we will send the swap request to bob
        const to = this.actors.bob;

        this.logger.info("Sending swap request");

        this.alphaLedger = defaultLedgerDescriptionForAsset(alphaAssetKind);
        this.alphaAsset = defaultAssetDescriptionForAsset(alphaAssetKind);
        to.alphaLedger = this.alphaLedger;
        to.alphaAsset = this.alphaAsset;

        this.logger.debug(
            "Derived %o from asset %s",
            this.alphaLedger,
            alphaAssetKind
        );
        this.logger.debug(
            "Derived %o from asset %s",
            this.alphaAsset,
            alphaAssetKind
        );

        this.betaLedger = defaultLedgerDescriptionForAsset(betaAssetKind);
        this.betaAsset = defaultAssetDescriptionForAsset(betaAssetKind);
        to.betaLedger = this.betaLedger;
        to.betaAsset = this.betaAsset;

        this.logger.debug(
            "Derived %o from asset %s",
            this.betaLedger,
            betaAssetKind
        );
        this.logger.debug(
            "Derived %o from asset %s",
            this.betaAsset,
            betaAssetKind
        );

        await this.initializeDependencies();
        await to.initializeDependencies();

        await this.setStartingBalance([
            this.alphaAsset,
            { name: this.betaAsset.name, quantity: "0" },
        ]);
        await to.setStartingBalance([
            { name: to.alphaAsset.name, quantity: "0" },
            to.betaAsset,
        ]);

        this.expectedBalanceChanges.set(
            betaAssetKind,
            new BigNumber(this.betaAsset.quantity)
        );
        to.expectedBalanceChanges.set(
            alphaAssetKind,
            new BigNumber(to.alphaAsset.quantity)
        );

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
                    .then(addresses => addresses[0]),
            },
            ...(await this.additionalIdentities(alphaAssetKind, betaAssetKind)),
            ...defaultExpiryTimes(),
        };

        this.swap = await comitClient.sendSwap(payload);
        to.swap = new Swap(
            to.wallets.bitcoin.inner,
            to.wallets.ethereum.inner,
            to.cnd,
            this.swap.self
        );
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

        const txid = await this.swap.deploy(Actor.defaultActionConfig);
        this.logger.debug(
            "Deployed htlc for swap %s in %s",
            this.swap.self,
            txid
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
        const response = await this.swap.tryExecuteAction("fund", {
            maxTimeoutSecs: 10,
            tryIntervalSecs: 1,
        });
        response.data.payload.gas_limit = hexGasLimit;
        const txid = await this.swap.doLedgerAction(response.data);
        this.logger.debug(
            "Deployed with low gas swap %s in %s",
            this.swap.self,
            txid
        );

        const status = await this.wallets.ethereum.getTransactionStatus(txid);
        if (status !== 0) {
            throw new Error("Deploy with low gas transaction was successful.");
        }
    }

    public async overfund() {
        const response = await this.swap.tryExecuteAction("fund", {
            maxTimeoutSecs: 10,
            tryIntervalSecs: 1,
        });
        const amount = response.data.payload.amount;
        response.data.payload.amount = amount * 1.01;

        const txid = await this.swap.doLedgerAction(response.data);
        this.logger.debug("Funded swap %s in %s", this.swap.self, txid);
    }

    public async underfund() {
        const response = await this.swap.tryExecuteAction("fund", {
            maxTimeoutSecs: 10,
            tryIntervalSecs: 1,
        });
        const amount = response.data.payload.amount;
        response.data.payload.amount = amount * 0.01;

        const txid = await this.swap.doLedgerAction(response.data);
        this.logger.debug("Funded swap %s in %s", this.swap.self, txid);
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

    /**
     * This method uses the cnd HTTP API routes introduced to support lightning.
     * It should ultimately replace [sendRequest] once the previous route format
     * is deprecated.
     * @param alphaLedgerKind
     * @param alphaAssetKind
     * @param betaLedgerKind
     * @param betaAssetKind
     */
    public async sendRequestOnLightningRoute(
        alphaLedgerKind: LedgerKind,
        alphaAssetKind: AssetKind,
        betaLedgerKind: LedgerKind,
        betaAssetKind: AssetKind
    ) {
        // By default, we will send the swap request to bob
        const to = this.actors.bob;

        this.logger.info("[WIP] Sending lighting swap request");

        this.alphaLedger = defaultLedgerDescriptionForLedger(alphaLedgerKind);
        this.alphaAsset = defaultAssetDescriptionForAsset(alphaAssetKind);
        to.alphaLedger = this.alphaLedger;
        to.alphaAsset = this.alphaAsset;

        this.logger.debug(
            "Alpha Ledger: %o; Alpha Asset: %o",
            this.alphaLedger,
            this.alphaAsset
        );

        this.betaLedger = defaultLedgerDescriptionForLedger(betaLedgerKind);
        this.betaAsset = defaultAssetDescriptionForAsset(betaAssetKind);
        to.betaLedger = this.betaLedger;
        to.betaAsset = this.betaAsset;

        this.logger.debug(
            "Beta Ledger: %o; Beta Asset: %o",
            this.betaLedger,
            this.betaAsset
        );

        await this.initializeDependencies();
        await to.initializeDependencies();

        await this.setStartingBalance([
            this.alphaAsset,
            { name: this.betaAsset.name, quantity: "0" },
        ]);
        await to.setStartingBalance([
            { name: to.alphaAsset.name, quantity: "0" },
            to.betaAsset,
        ]);

        this.expectedBalanceChanges.set(
            betaAssetKind,
            new BigNumber(this.betaAsset.quantity)
        );
        to.expectedBalanceChanges.set(
            alphaAssetKind,
            new BigNumber(to.alphaAsset.quantity)
        );

        if (
            this.alphaLedger.name === "lightning" ||
            this.betaLedger.name === "lightning"
        ) {
            await this.wallets
                .getWalletForLedger("lightning")
                .addPeer(to.wallets.getWalletForLedger("lightning"));
        }

        this.expectedBalanceChanges.set(
            betaAssetKind,
            new BigNumber(this.betaAsset.quantity)
        );
        to.expectedBalanceChanges.set(
            alphaAssetKind,
            new BigNumber(to.alphaAsset.quantity)
        );

        // Here is where we would send the swap request to cnd HTTP API
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
            assetKind,
            expectedBalanceChange,
        ] of this.expectedBalanceChanges.entries()) {
            this.logger.debug(
                "Checking that %s balance changed by %d",
                assetKind,
                expectedBalanceChange
            );

            const wallet = this.wallets[
                defaultLedgerDescriptionForAsset(assetKind).name
            ];
            const expectedBalance = new BigNumber(
                this.startingBalances.get(assetKind)
            ).plus(expectedBalanceChange);
            const maximumFee = wallet.MaximumFee;

            const balanceInclFees = expectedBalance.minus(maximumFee);

            const currentWalletBalance = await wallet.getBalanceByAsset(
                defaultAssetDescriptionForAsset(assetKind)
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

        for (const [assetKind] of this.startingBalances.entries()) {
            const wallet = this.wallets[
                defaultLedgerDescriptionForAsset(assetKind).name
            ];
            const maximumFee = wallet.MaximumFee;

            this.logger.debug(
                "Checking that %s balance changed by max %d (MaximumFee)",
                assetKind,
                maximumFee
            );
            const expectedBalance = new BigNumber(
                this.startingBalances.get(assetKind)
            );
            const currentWalletBalance = await wallet.getBalanceByAsset(
                defaultAssetDescriptionForAsset(assetKind)
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

    public stop() {
        this.cndInstance.stop();
        if (this.lnd && this.lnd.isRunning()) {
            this.lnd.stop();
        }
    }

    public async restart() {
        this.stop();
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
        let lightning = false;
        for (const ledgerName of [
            this.alphaLedger.name,
            this.betaLedger.name,
        ]) {
            if (ledgerName === "lightning") {
                lightning = true;
            }
            await this.wallets.initializeForLedger(
                ledgerName,
                this.logger,
                this.logRoot,
                this.config
            );
        }

        // Once `ComitClient` can be built with lightning, this hack should
        // be removed and ComitClient built with the wallets available
        if (!lightning) {
            this.comitClient = new ComitClient(
                this.wallets.getWalletForLedger("bitcoin").inner,
                this.wallets.getWalletForLedger("ethereum").inner,
                this.cnd
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
                this.startingBalances.set(asset.name, new BigNumber(0));
                continue;
            }

            const ledger = defaultLedgerDescriptionForAsset(asset.name);
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
            this.startingBalances.set(asset.name, balance);
        }
    }

    private defaultAlphaAssetKind() {
        const defaultAlphaAssetKind = AssetKind.Bitcoin;
        this.logger.info(
            "AssetKind for alpha ledger not specified, defaulting to %s",
            defaultAlphaAssetKind
        );

        return defaultAlphaAssetKind;
    }

    private defaultBetaAssetKind() {
        const defaultBetaAssetKind = AssetKind.Ether;
        this.logger.info(
            "AssetKind for beta ledger not specified, defaulting to %s",
            defaultBetaAssetKind
        );

        return defaultBetaAssetKind;
    }

    public cndHttpApiUrl() {
        const cndSocket = this.cndInstance.getConfigFile().http_api.socket;
        return `http://${cndSocket.address}:${cndSocket.port}`;
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
            return await this.pollSwapDetails(swapUrl, iteration);
        }
    }
}

/**
 * WIP as the cnd REST API routes for lightning are not yet defined.
 * @param ledger
 * @returns The ledger formatted as needed for the request body to cnd HTTP API on the lightning route.
 */
function defaultLedgerDescriptionForLedger(ledger: LedgerKind): Ledger {
    switch (ledger) {
        case LedgerKind.Lightning: {
            return {
                name: LedgerKind.Lightning,
            };
        }
        case LedgerKind.Bitcoin: {
            return {
                name: LedgerKind.Bitcoin,
                network: "regtest",
            };
        }
        case LedgerKind.Ethereum: {
            return {
                name: LedgerKind.Ethereum,
                chain_id: 17,
            };
        }
    }
}

function defaultLedgerDescriptionForAsset(asset: AssetKind): Ledger {
    switch (asset) {
        case AssetKind.Bitcoin: {
            return {
                name: LedgerKind.Bitcoin,
                network: "regtest",
            };
        }
        case AssetKind.Ether: {
            return {
                name: LedgerKind.Ethereum,
                chain_id: 17,
            };
        }
        case AssetKind.Erc20: {
            return {
                name: LedgerKind.Ethereum,
                chain_id: 17,
            };
        }
    }
}

function defaultAssetDescriptionForAsset(asset: AssetKind): Asset {
    switch (asset) {
        case AssetKind.Bitcoin: {
            return {
                name: AssetKind.Bitcoin,
                quantity: "100000000",
            };
        }
        case AssetKind.Ether: {
            return {
                name: AssetKind.Ether,
                quantity: parseEther("10").toString(),
            };
        }
        case AssetKind.Erc20: {
            return {
                name: AssetKind.Erc20,
                quantity: parseEther("100").toString(),
                token_contract: global.tokenContract,
            };
        }
    }
}

function defaultExpiryTimes() {
    const alphaExpiry = Math.round(Date.now() / 1000) + 8;
    const betaExpiry = Math.round(Date.now() / 1000) + 3;

    return {
        alpha_expiry: alphaExpiry,
        beta_expiry: betaExpiry,
    };
}

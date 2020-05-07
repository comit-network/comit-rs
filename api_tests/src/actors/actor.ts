import {
    Cnd,
    HanEthereumEtherHalightLightningBitcoinRequestBody,
    LedgerAction,
    Swap,
    Transaction,
    Wallets as SdkWallets,
} from "comit-sdk";
import { Logger } from "log4js";
import { E2ETestActorConfig } from "../config";
import {
    Asset,
    assetAsKey,
    AssetKind,
    defaultAssetValue,
    toKey,
    toKind,
} from "../asset";
import { CndInstance } from "../cnd/cnd_instance";
import { Ledger, LedgerKind } from "../ledgers/ledger";
import { LedgerConfig, sleep } from "../utils";
import { Actors } from "./index";
import { Wallets } from "../wallets";
import { defaultLedgerDescriptionForLedger } from "./defaults";
import { EscrowStatus, LedgerState, SwapResponse } from "../swap_response";

export type ActorName = "alice" | "bob" | "charlie";

export class Actor {
    public static defaultActionConfig = {
        maxTimeoutSecs: 20,
        tryIntervalSecs: 1,
    };

    public static async newInstance(
        name: ActorName,
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

    public readonly startingBalances: Map<assetAsKey, bigint>;
    public readonly expectedBalanceChanges: Map<assetAsKey, bigint>;

    constructor(
        public readonly logger: Logger,
        public readonly cndInstance: CndInstance,
        public readonly name: ActorName
    ) {
        this.wallets = new Wallets({});
        const socket = cndInstance.getConfigFile().http_api.socket;
        this.cnd = new Cnd(`http://${socket}`);

        this.startingBalances = new Map();
        this.expectedBalanceChanges = new Map();
    }

    /**
     * Interactions with cnd REST API
     */

    /**
     * Create a Swap
     * @param createSwapPayload
     */
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

    public cndHttpApiUrl() {
        const socket = this.cndInstance.getConfigFile().http_api.socket;
        return `http://${socket}`;
    }

    public async pollCndUntil(
        location: string,
        predicate: (body: SwapResponse) => boolean
    ): Promise<SwapResponse> {
        const response = await this.cnd.fetch<SwapResponse>(location);

        expect(response.status).toEqual(200);

        if (predicate(response.data)) {
            return response.data;
        } else {
            await sleep(500);

            return this.pollCndUntil(location, predicate);
        }
    }

    public async pollSwapResponse(
        swapUrl: string,
        iteration: number = 0
    ): Promise<SwapResponse> {
        if (iteration > 5) {
            throw new Error(`Could not retrieve Swap ${swapUrl}`);
        }
        iteration++;

        try {
            return this.cnd
                .fetch<SwapResponse>(swapUrl)
                .then((response) => response.data);
        } catch (error) {
            await sleep(1000);
            return this.pollSwapResponse(swapUrl, iteration);
        }
    }

    /**
     * Wait for and execute the init action
     * @param config Timeout parameters
     */
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

    /**
     * Wait for and execute the fund action
     * @param config Timeout parameters
     */
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

        const role = await this.cryptoRole();
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

    /**
     * Wait for and execute the redeem action
     */
    public async redeem() {
        if (!this.swap) {
            throw new Error("Cannot redeem non-existent swap");
        }

        const txid = await this.swap.redeem(Actor.defaultActionConfig);
        this.logger.debug("Redeemed swap %s in %s", this.swap.self, txid);

        const role = await this.cryptoRole();
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

    public async getSwapResponse(): Promise<SwapResponse> {
        return this.cnd
            .fetch<SwapResponse>(this.swap.self)
            .then((response) => response.data);
    }

    public async cryptoRole(): Promise<"Alice" | "Bob"> {
        return this.getSwapResponse().then(
            (swapResponse) => swapResponse.properties.role
        );
    }

    /**
     * Assertions against cnd API Only
     */

    public async assertAlphaFunded(): Promise<void> {
        await this.assertLedgerStatus("alpha", EscrowStatus.Funded);
    }

    public async assertBetaFunded() {
        await this.assertLedgerStatus("beta", EscrowStatus.Funded);
    }

    public async assertAlphaRedeemed() {
        await this.assertLedgerStatus("alpha", EscrowStatus.Redeemed);
    }

    public async assertBetaRedeemed() {
        await this.assertLedgerStatus("beta", EscrowStatus.Redeemed);
    }

    private async assertLedgerStatus(
        ledgerRel: "alpha" | "beta",
        status: EscrowStatus
    ): Promise<void> {
        await this.pollCndUntil(this.swap.self, (swapResponse) => {
            for (const entity of swapResponse.entities) {
                const ledgerState = entity as LedgerState;
                if (
                    ledgerState.class.includes("state") &&
                    ledgerState.rel.includes(ledgerRel)
                ) {
                    return ledgerState.properties.status === status;
                }
            }
        });
    }

    /**
     * Assertions against Ledgers
     */

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
        for (const [
            assetAsKey,
            expectedBalanceChange,
        ] of this.expectedBalanceChanges.entries()) {
            this.logger.debug(
                "Checking that %s balance changed by %d",
                assetAsKey,
                expectedBalanceChange
            );

            const { asset, ledger } = toKind(assetAsKey);

            const wallet = this.wallets[ledger];
            const expectedBalance =
                this.startingBalances.get(assetAsKey) + expectedBalanceChange;
            const maximumFee = BigInt(wallet.MaximumFee);

            const balanceInclFees = expectedBalance - maximumFee;

            const currentWalletBalance = await wallet.getBalanceByAsset(
                defaultAssetValue(asset, ledger)
            );
            expect(currentWalletBalance).toBeGreaterThanOrEqual(
                // @ts-ignore: Jest supports bigint, types to be fixed updated with
                // https://github.com/DefinitelyTyped/DefinitelyTyped/pull/44368
                balanceInclFees
            );

            this.logger.debug(
                "Balance check was positive, current balance is %d",
                currentWalletBalance
            );
        }
    }

    /**
     * Manage cnd instance
     */

    public async start() {
        await this.cndInstance.start();
    }

    public async stop() {
        this.logger.debug("Stopping actor");
        this.cndInstance.stop();
    }

    public async dumpState() {
        this.logger.debug("dumping current state");

        if (this.swap) {
            const swapResponse = await this.getSwapResponse();

            this.logger.debug(
                "swap status: %s",
                swapResponse.properties.status
            );
            this.logger.debug("swap details: ", JSON.stringify(swapResponse));

            this.logger.debug(
                "alpha ledger wallet balance %d",
                await this.alphaLedgerWallet.getBalanceByAsset(this.alphaAsset)
            );
            this.logger.debug(
                "beta ledger wallet balance %d",
                await this.betaLedgerWallet.getBalanceByAsset(this.betaAsset)
            );
        }
    }

    /**
     * Wallet Management
     */

    /**
     * Mine and set starting balances
     * @param assets
     */
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

    get alphaLedgerWallet() {
        return this.wallets.getWalletForLedger(this.alphaLedger.name);
    }

    get betaLedgerWallet() {
        return this.wallets.getWalletForLedger(this.betaLedger.name);
    }
}

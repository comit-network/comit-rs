import {
    Cnd,
    LedgerParameters,
    Step,
    Swap,
    Wallets as SdkWallets,
} from "comit-sdk";
import {
    EscrowStatus,
    HalbitHerc20Payload,
    HbitHerc20Payload,
    Herc20HalbitPayload,
    Herc20HbitPayload,
    LedgerState,
    OpenOrdersEntity,
    OrderEntity,
    Position,
    SwapResponse,
    SwapStatus,
} from "../payload";
import { Logger } from "log4js";
import { CndConfigFile, E2ETestActorConfig } from "../config";
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
import { HarnessGlobal, LedgerConfig, sleep } from "../utils";
import { Actors } from "./index";
import {
    newBitcoinStubWallet,
    newEthereumStubWallet,
    newLightningStubWallet,
    Wallets,
} from "../wallets";
import { defaultLedgerDescriptionForLedger } from "./defaults";
import pTimeout from "p-timeout";
import { Entity, Link } from "comit-sdk/dist/src/cnd/siren";
import { BitcoindWallet, BitcoinWallet } from "../wallets/bitcoin";
import { EthereumWallet, Web3EthereumWallet } from "../wallets/ethereum";
import { LightningWallet } from "../wallets/lightning";
import { merge } from "lodash";

declare var global: HarnessGlobal;

export type ActorName = "alice" | "bob" | "carol";

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
        logger: Logger,
        configOverrides: Partial<CndConfigFile>,
        ethereumLockDir: string,
        lndWallets: { alice?: LightningWallet; bob?: LightningWallet }
    ) {
        const actorConfig = await E2ETestActorConfig.for(name, logger);
        const generatedConfig = actorConfig.generateCndConfigFile(ledgerConfig);
        const finalConfig = merge(generatedConfig, configOverrides);

        const cndInstance = new CndInstance(
            cargoTargetDirectory,
            cndLogFile,
            logger,
            finalConfig
        );

        await cndInstance.start();

        logger.info(
            "Created new actor with config %s",
            JSON.stringify(finalConfig)
        );

        const wallets = new Wallets({
            bitcoin: await newBitcoinWallet(ledgerConfig, logger),
            ethereum: await newEthereumWallet(
                ledgerConfig,
                ethereumLockDir,
                logger
            ),
            lightning: newLightningWallet(lndWallets, name, logger),
        });

        return new Actor(logger, cndInstance, wallets, name);
    }

    public actors: Actors;

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
        public readonly wallets: Wallets,
        public readonly name: ActorName
    ) {
        const socket = cndInstance.getConfigFile().http_api.socket;
        this.cnd = new Cnd(`http://${socket}`);

        this.startingBalances = new Map();
        this.expectedBalanceChanges = new Map();
    }

    public async connect(other: Actor) {
        const addr = await other.cnd.getPeerListenAddresses();
        // @ts-ignore
        await this.cnd.client.post("dial", { addresses: addr });

        const otherPeerId = await other.cnd.getPeerId();
        await this.pollUntilConnectedTo(otherPeerId);
    }

    /**
     * Create a herc20<->halbit Swap
     * @param create
     */
    public async createHerc20HalbitSwap(create: Herc20HalbitPayload) {
        this.alphaLedger = {
            name: LedgerKind.Ethereum,
            chain_id: create.alpha.chain_id,
        };
        this.betaLedger = {
            name: LedgerKind.Lightning,
            network: create.beta.network,
        };
        this.alphaAsset = {
            name: AssetKind.Erc20,
            quantity: create.alpha.amount,
            ledger: LedgerKind.Ethereum,
            tokenContract: create.alpha.token_contract,
        };
        this.betaAsset = {
            name: AssetKind.Bitcoin,
            quantity: create.beta.amount,
            ledger: LedgerKind.Lightning,
        };

        await this.setStartingBalances();

        const location = await this.createHerc20Halbit(create);

        this.swap = new Swap(
            this.cnd,
            location,
            new SdkWallets({
                ethereum: this.wallets.ethereum.inner,
                lightning: this.wallets.lightning.inner,
            })
        );
    }

    /**
     * Create a halbit<->herc20 Swap
     * @param create
     */
    public async createHalbitHerc20Swap(create: HalbitHerc20Payload) {
        this.alphaLedger = {
            name: LedgerKind.Lightning,
            network: create.alpha.network,
        };
        this.betaLedger = {
            name: LedgerKind.Ethereum,
            chain_id: create.beta.chain_id,
        };
        this.alphaAsset = {
            name: AssetKind.Bitcoin,
            quantity: create.alpha.amount,
            ledger: LedgerKind.Lightning,
        };
        this.betaAsset = {
            name: AssetKind.Erc20,
            quantity: create.beta.amount,
            ledger: LedgerKind.Ethereum,
            tokenContract: create.beta.token_contract,
        };

        await this.setStartingBalances();

        const location = await this.createHalbitHerc20(create);

        this.swap = new Swap(
            this.cnd,
            location,
            new SdkWallets({
                ethereum: this.wallets.ethereum.inner,
                lightning: this.wallets.lightning.inner,
            })
        );
    }

    /**
     * Create a herc20-hbit Swap
     * @param create
     */
    public async createHerc20HbitSwap(create: Herc20HbitPayload) {
        this.alphaLedger = {
            name: LedgerKind.Ethereum,
            chain_id: create.alpha.chain_id,
        };
        this.betaLedger = {
            name: LedgerKind.Bitcoin,
            network: create.beta.network,
        };
        this.alphaAsset = {
            name: AssetKind.Erc20,
            quantity: create.alpha.amount,
            ledger: LedgerKind.Ethereum,
            tokenContract: create.alpha.token_contract,
        };
        this.betaAsset = {
            name: AssetKind.Bitcoin,
            quantity: create.beta.amount,
            ledger: LedgerKind.Bitcoin,
        };

        await this.setStartingBalances();

        const location = await this.createHerc20Hbit(create);

        this.swap = new Swap(
            this.cnd,
            location,
            new SdkWallets({
                ethereum: this.wallets.ethereum.inner,
                bitcoin: this.wallets.bitcoin.inner,
            })
        );
    }

    /**
     * Create a hbit-herc20 Swap
     * @param create
     */
    public async createHbitHerc20Swap(create: HbitHerc20Payload) {
        this.alphaLedger = {
            name: LedgerKind.Bitcoin,
            network: create.alpha.network,
        };
        this.betaLedger = {
            name: LedgerKind.Ethereum,
            chain_id: create.beta.chain_id,
        };
        this.alphaAsset = {
            name: AssetKind.Bitcoin,
            quantity: create.alpha.amount,
            ledger: LedgerKind.Bitcoin,
        };
        this.betaAsset = {
            name: AssetKind.Erc20,
            quantity: create.beta.amount,
            ledger: LedgerKind.Ethereum,
            tokenContract: create.beta.token_contract,
        };

        await this.setStartingBalances();

        const location = await this.createHbitHerc20(create);

        this.swap = new Swap(
            this.cnd,
            location,
            new SdkWallets({
                bitcoin: this.wallets.bitcoin.inner,
                ethereum: this.wallets.ethereum.inner,
            })
        );
    }

    /**
     * Makes a BtcDai sell order (herc20-hbit Swap)
     */
    public async makeBtcDaiOrder(
        position: Position,
        quantity: number,
        price: number
    ): Promise<string> {
        const sats = Number(quantity) * 100_000_000;
        const btcAsset = {
            ledger: LedgerKind.Bitcoin,
            name: AssetKind.Bitcoin,
            quantity: sats.toString(10),
        };

        const daiPerBtc = BigInt(price);
        const weiPerDai = BigInt("1000000000000000000");
        const satsPerBtc = BigInt("100000000");
        const weiPerSat = (daiPerBtc * weiPerDai) / satsPerBtc;
        const dai = BigInt(sats) * weiPerSat;

        const daiAsset = {
            ledger: LedgerKind.Ethereum,
            name: AssetKind.Erc20,
            quantity: dai.toString(10),
            tokenContract: global.tokenContract,
        };

        switch (position) {
            case Position.Buy: {
                switch (this.name) {
                    case "alice": {
                        this.alphaAsset = daiAsset;
                        this.betaAsset = btcAsset;
                        this.alphaLedger = {
                            name: LedgerKind.Ethereum,
                        };
                        this.betaLedger = {
                            name: LedgerKind.Bitcoin,
                        };
                        break;
                    }
                    case "bob": {
                        this.alphaAsset = btcAsset;
                        this.alphaLedger = {
                            name: LedgerKind.Bitcoin,
                        };
                        this.betaAsset = daiAsset;
                        this.betaLedger = {
                            name: LedgerKind.Ethereum,
                        };
                        break;
                    }
                }
                break;
            }
            case Position.Sell: {
                switch (this.name) {
                    case "alice": {
                        this.alphaAsset = btcAsset;
                        this.betaAsset = daiAsset;
                        this.alphaLedger = {
                            name: LedgerKind.Bitcoin,
                        };
                        this.betaLedger = {
                            name: LedgerKind.Ethereum,
                        };
                        break;
                    }
                    case "bob": {
                        this.alphaAsset = daiAsset;
                        this.betaAsset = btcAsset;
                        this.alphaLedger = {
                            name: LedgerKind.Ethereum,
                        };
                        this.betaLedger = {
                            name: LedgerKind.Bitcoin,
                        };
                        break;
                    }
                }
                break;
            }
        }

        await this.setStartingBalances();

        // @ts-ignore
        const response = await this.cnd.client.post("/orders/BTC-DAI", {
            position,
            quantity: sats.toString(10),
            price: weiPerSat.toString(10),
            swap: {
                role: this.name, // TODO: don't abuse name for role,
                bitcoin_address: await this.wallets.bitcoin.address(),
                ethereum_address: this.wallets.ethereum.account(),
            },
        });

        return response.headers.location;
    }

    public async fetchOrder(href: string): Promise<OrderEntity> {
        const response = await this.cnd.fetch<OrderEntity>(href);

        return response.data;
    }

    public async listOpenOrders(): Promise<OpenOrdersEntity> {
        const response = await this.cnd.fetch<OpenOrdersEntity>("/orders");

        return response.data;
    }

    /**
     * Wait until a swap is created on bobs end
     */
    public async assertSwapCreatedFromOrder(orderUrl: string) {
        if (this.name === "bob") {
            // Since Alice has taken the swap, the order created by Bob should have an associated swap in the navigational link
            const bobGetOrderResponse = await this.cnd.fetch<Entity>(orderUrl);

            expect(bobGetOrderResponse.status).toEqual(200);
            const linkToBobSwap = bobGetOrderResponse.data.links.find(
                (link: Link) => link.rel.includes("swap")
            );
            expect(linkToBobSwap).toBeDefined();

            // The link the Bobs swap should return 200
            // "GET /swaps/934dd090-f8eb-4244-9aba-78e23d3f79eb HTTP/1.1"
            const bobSwapResponse = await this.cnd.fetch<Entity>(
                linkToBobSwap.href
            );

            expect(bobSwapResponse.status).toEqual(200);

            this.swap = new Swap(
                this.cnd,
                linkToBobSwap.href,
                new SdkWallets({
                    ethereum: this.wallets.ethereum.inner,
                    bitcoin: this.wallets.bitcoin.inner,
                })
            );
        } else {
            throw new Error(
                `assertSwapCreated does not support the actor ${this.name} yet`
            );
        }
    }

    private async setStartingBalances() {
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
    }

    public cndHttpApiUrl() {
        const socket = this.cndInstance.getConfigFile().http_api.socket;
        return `http://${socket}`;
    }

    public async pollCndUntil<T = SwapResponse>(
        location: string,
        predicate: (body: T) => boolean
    ): Promise<T> {
        const response = await this.cnd.fetch<T>(location);

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

    public async assertAndExecuteNextAction(expectedActionName: string) {
        if (!this.swap) {
            throw new Error("Cannot do anything on non-existent swap");
        }

        const { action, transaction } = await pTimeout(
            (async () => {
                while (true) {
                    const action = await this.swap.nextAction();

                    if (action && action.name === expectedActionName) {
                        const transaction = await action.execute();
                        return { action, transaction };
                    }

                    await sleep(
                        Actor.defaultActionConfig.tryIntervalSecs * 1000
                    );
                }
            })(),
            Actor.defaultActionConfig.maxTimeoutSecs * 1000,
            `action '${expectedActionName}' not found`
        );

        this.logger.debug(
            "%s done on swap %s in %s",
            action.name,
            this.swap.self,
            transaction
        );
        switch (action.name) {
            case "deploy":
                await this.assertDeployed();
                break;
            case "fund":
                await this.assertFunded();
                break;
            case "redeem":
                await this.assertRedeemed();
                break;
            case "refund":
                await this.assertRefunded();
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

    public async assertAlphaDeployed() {
        await this.assertLedgerStatus("alpha", EscrowStatus.Deployed);
        await this.assertLedgerEventPresence("alpha", Step.Deploy);
    }

    public async assertBetaDeployed() {
        await this.assertLedgerStatus("beta", EscrowStatus.Deployed);
        await this.assertLedgerEventPresence("beta", Step.Deploy);
    }

    public async assertAlphaFunded(): Promise<void> {
        await this.assertLedgerStatus("alpha", EscrowStatus.Funded);
        await this.assertLedgerEventPresence("alpha", Step.Fund);
    }

    public async assertBetaFunded() {
        await this.assertLedgerStatus("beta", EscrowStatus.Funded);
        await this.assertLedgerEventPresence("beta", Step.Fund);
    }

    public async assertAlphaRedeemed() {
        await this.assertLedgerStatus("alpha", EscrowStatus.Redeemed);
        await this.assertLedgerEventPresence("alpha", Step.Redeem);
    }

    public async assertBetaRedeemed() {
        await this.assertLedgerStatus("beta", EscrowStatus.Redeemed);
        await this.assertLedgerEventPresence("beta", Step.Redeem);
    }

    public async assertAlphaRefunded() {
        await this.assertLedgerStatus("alpha", EscrowStatus.Refunded);
        await this.assertLedgerEventPresence("alpha", Step.Refund);
    }

    public async assertBetaRefunded() {
        await this.assertLedgerStatus("beta", EscrowStatus.Refunded);
        await this.assertLedgerEventPresence("beta", Step.Refund);
    }

    private async assertDeployed() {
        const role = await this.cryptoRole();
        switch (role) {
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

    private async assertFunded() {
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

    private async assertRedeemed() {
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

    private async assertRefunded() {
        const role = await this.cryptoRole();
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

    private async assertLedgerEventPresence(
        ledgerRel: "alpha" | "beta",
        step: Step
    ): Promise<void> {
        await this.pollCndUntil(this.swap.self, (swapResponse) => {
            let protocol;
            for (const entity of swapResponse.entities) {
                const ledgerParameters = entity as LedgerParameters;
                if (
                    ledgerParameters.class.includes("parameters") &&
                    ledgerParameters.rel.includes(ledgerRel)
                ) {
                    protocol = ledgerParameters.properties.protocol;
                    break;
                }
            }

            // No events are set for halbit
            if (protocol === "halbit") {
                return true;
            }

            for (const entity of swapResponse.entities) {
                const ledgerState = entity as LedgerState;
                if (
                    ledgerState.class.includes("state") &&
                    ledgerState.rel.includes(ledgerRel)
                ) {
                    return !!ledgerState.properties.events[step];
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
            if (entity.properties.status === SwapStatus.Swapped) {
                break;
            }
        }

        await this.assertBalancesAfterSwap();
    }

    public async assertBalancesAfterSwap() {
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
                balanceInclFees
            );

            this.logger.debug(
                "Balance check was positive, current balance is %d",
                currentWalletBalance
            );
        }
    }

    public async assertBalancesAfterRefund() {
        this.logger.debug("Checking if swap @ %s was refunded", this.swap.self);

        for (const [assetKey] of this.startingBalances.entries()) {
            const { asset, ledger } = toKind(assetKey);

            const wallet = this.wallets[ledger];
            const maximumFee = BigInt(wallet.MaximumFee);

            this.logger.debug(
                "Checking that %s balance changed by max %d (MaximumFee)",
                assetKey,
                maximumFee
            );
            const expectedBalance = this.startingBalances.get(assetKey);
            const currentWalletBalance = await wallet.getBalanceByAsset(
                defaultAssetValue(asset, ledger)
            );
            const balanceInclFees = expectedBalance - maximumFee;
            expect(currentWalletBalance).toBeGreaterThanOrEqual(
                balanceInclFees
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

    public async restart() {
        await this.stop();
        await this.start();
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

    public async createHerc20Halbit(
        body: Herc20HalbitPayload
    ): Promise<string> {
        // @ts-ignore: client is private.
        const response = await this.cnd.client.post(
            "swaps/herc20/halbit",
            body
        );

        return response.headers.location;
    }

    public async createHalbitHerc20(
        body: HalbitHerc20Payload
    ): Promise<string> {
        // @ts-ignore: client is private.
        const response = await this.cnd.client.post(
            "swaps/halbit/herc20",
            body
        );

        return response.headers.location;
    }

    public async createHerc20Hbit(body: Herc20HbitPayload): Promise<string> {
        // @ts-ignore: client is private.
        const response = await this.cnd.client.post("swaps/herc20/hbit", body);

        return response.headers.location;
    }

    public async createHbitHerc20(body: HbitHerc20Payload): Promise<string> {
        // @ts-ignore: client is private.
        const response = await this.cnd.client.post("swaps/hbit/herc20", body);

        return response.headers.location;
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

    public async waitForSwap(): Promise<void> {
        const response = await this.pollCndUntil<Entity>(
            "/swaps",
            (body) => body.entities.length > 0
        );

        this.swap = new Swap(
            this.cnd,
            response.entities[0].href,
            new SdkWallets({
                ethereum: this.wallets.ethereum.inner,
                bitcoin: this.wallets.bitcoin.inner,
            })
        );
    }

    get alphaLedgerWallet() {
        return this.wallets.getWalletForLedger(this.alphaLedger.name);
    }

    get betaLedgerWallet() {
        return this.wallets.getWalletForLedger(this.betaLedger.name);
    }

    private async pollUntilConnectedTo(peer: string) {
        interface Peers {
            peers: Peer[];
        }

        interface Peer {
            id: string;
            // these are multi-addresses
            endpoints: string[];
        }

        await this.pollCndUntil<Peers>(
            "/peers",
            (peers) =>
                peers.peers.findIndex((candidate) => candidate.id === peer) !==
                -1
        );
    }
}

async function newBitcoinWallet(
    ledgerConfig: LedgerConfig,
    logger: Logger
): Promise<BitcoinWallet> {
    const bitcoinConfig = ledgerConfig.bitcoin;
    return bitcoinConfig
        ? BitcoindWallet.newInstance(bitcoinConfig, logger)
        : Promise.resolve(newBitcoinStubWallet(logger));
}

async function newEthereumWallet(
    ledgerConfig: LedgerConfig,
    ethereumLockDir: string,
    logger: Logger
): Promise<EthereumWallet> {
    const ethereumConfig = ledgerConfig.ethereum;
    return ethereumConfig
        ? Web3EthereumWallet.new_instance(
              ethereumConfig.dev_account_key,
              ethereumConfig.rpc_url,
              logger,
              ethereumLockDir,
              ethereumConfig.chain_id
          )
        : Promise.resolve(newEthereumStubWallet(logger));
}

function newLightningWallet(
    lightningWallets: { alice?: LightningWallet; bob?: LightningWallet },
    actor: ActorName,
    logger: Logger
): LightningWallet {
    switch (actor) {
        case "alice": {
            return lightningWallets.alice || newLightningStubWallet(logger);
        }
        case "bob": {
            return lightningWallets.bob || newLightningStubWallet(logger);
        }
        default: {
            throw new Error(
                `Cannot initialize Lightning wallet for actor: '${actor}'`
            );
        }
    }
}

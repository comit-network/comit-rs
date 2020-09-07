import {
    ActionKind,
    HalbitHerc20Payload,
    HbitHerc20Payload,
    Herc20HalbitPayload,
    Herc20HbitPayload,
    OpenOrdersEntity,
    OrderEntity,
    Position,
    SwapEntity,
    SwapEventKind,
} from "../cnd_client/payload";
import { Logger } from "log4js";
import {
    Asset,
    assetAsKey,
    AssetKind,
    defaultAssetValue,
    toKey,
    toKind,
} from "../asset";
import { CndInstance } from "../environment/cnd_instance";
import { Ledger, LedgerKind } from "../ledger";
import { sleep } from "../utils";
import { Role } from "./index";
import { Wallets } from "../wallets";
import { defaultLedgerDescriptionForLedger } from "./defaults";
import pTimeout from "p-timeout";
import { AxiosResponse } from "axios";
import CndClient from "../cnd_client";
import { Swap } from "../swap";
import { Entity } from "../cnd_client/siren";
import { HarnessGlobal } from "../environment";

declare var global: HarnessGlobal;

/**
 * An actor that uses cnd to perform to participate in the COMIT network.
 *
 * Although in reality instance of cnd can handle multiple swaps in different roles at the same time, the test framework limits an instance to one specific role.
 */
export class CndActor {
    readonly cnd: CndClient;
    public swap: Swap;

    public alphaLedger: Ledger;
    public alphaAsset: Asset;

    public betaLedger: Ledger;
    public betaAsset: Asset;

    public readonly startingBalances: Map<assetAsKey, bigint>;
    public readonly expectedBalanceChanges: Map<assetAsKey, bigint>;

    public constructor(
        public readonly logger: Logger,
        public readonly cndInstance: CndInstance,
        public readonly wallets: Wallets,
        public readonly role: Role
    ) {
        const socket = cndInstance.getConfigFile().http_api.socket;
        this.cnd = new CndClient(`http://${socket}`);

        this.startingBalances = new Map();
        this.expectedBalanceChanges = new Map();
    }

    public async connect(other: CndActor) {
        await this.cnd.dial(other.cnd);

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

        const location = await this.cnd.createHerc20Halbit(create);

        this.swap = new Swap(
            this.cnd,
            location,
            new Wallets({
                ethereum: this.wallets.ethereum,
                lightning: this.wallets.lightning,
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

        const location = await this.cnd.createHalbitHerc20(create);

        this.swap = new Swap(
            this.cnd,
            location,
            new Wallets({
                ethereum: this.wallets.ethereum,
                lightning: this.wallets.lightning,
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

        const location = await this.cnd.createHerc20Hbit(create);

        this.swap = new Swap(
            this.cnd,
            location,
            new Wallets({
                ethereum: this.wallets.ethereum,
                bitcoin: this.wallets.bitcoin,
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

        const location = await this.cnd.createHbitHerc20(create);

        this.swap = new Swap(
            this.cnd,
            location,
            new Wallets({
                bitcoin: this.wallets.bitcoin,
                ethereum: this.wallets.ethereum,
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
                switch (this.role) {
                    case "Alice": {
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
                    case "Bob": {
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
                switch (this.role) {
                    case "Alice": {
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
                    case "Bob": {
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

        return this.cnd.createBtcDaiOrder({
            position,
            quantity: sats.toString(10),
            price: weiPerSat.toString(10),
            swap: {
                role: this.role,
                bitcoin_address: await this.wallets.bitcoin.getAddress(),
                ethereum_address: this.wallets.ethereum.getAccount(),
            },
        });
    }

    public async fetchOrder(href: string): Promise<OrderEntity> {
        const response = await this.cnd.fetch<OrderEntity>(href);

        return response.data;
    }

    public async listOpenOrders(): Promise<OpenOrdersEntity> {
        const response = await this.cnd.fetch<OpenOrdersEntity>("/orders");

        return response.data;
    }

    public async executeSirenAction<T>(
        entity: Entity,
        actionName: string
    ): Promise<AxiosResponse<T>> {
        const action = entity.actions.find(
            (action) => action.name === actionName
        );

        if (!action) {
            throw new Error(`Action ${actionName} is not present`);
        }

        return this.cnd.executeSirenAction(action);
    }

    private async setStartingBalances() {
        switch (this.role) {
            case "Alice": {
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
            case "Bob": {
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
        }
    }

    public cndHttpApiUrl() {
        const socket = this.cndInstance.getConfigFile().http_api.socket;
        return `http://${socket}`;
    }

    public async pollCndUntil<T>(
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

    public async assertAndExecuteNextAction(expectedActionName: ActionKind) {
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

                    await sleep(1000);
                }
            })(),
            20 * 1000,
            `action '${expectedActionName}' not found`
        );

        this.logger.debug(
            "%s done on swap %s in %s",
            action.name,
            this.swap.self,
            transaction
        );

        const swapProperties = await this.getSwapResponse().then(
            (e) => e.properties
        );
        const event = nextExpectedEvent(
            swapProperties.role,
            expectedActionName,
            swapProperties.alpha.protocol,
            swapProperties.beta.protocol
        );

        if (event === null) {
            return;
        }

        await pTimeout(
            (async () => {
                while (true) {
                    const swapEntity = await this.getSwapResponse();

                    if (
                        swapEntity.properties.events
                            .map((e) => e.name)
                            .includes(event)
                    ) {
                        return;
                    }

                    await sleep(500);
                }
            })(),
            30_000,
            `event '${event}' expected but never found`
        );
    }

    public async getSwapResponse(): Promise<SwapEntity> {
        return this.cnd
            .fetch<SwapEntity>(this.swap.self)
            .then((response) => response.data);
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
            new Wallets({
                ethereum: this.wallets.ethereum,
                bitcoin: this.wallets.bitcoin,
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

/**
 * Computes the event that we are expecting to see.
 */
function nextExpectedEvent(
    role: "Alice" | "Bob",
    action: ActionKind,
    alphaProtocol: "hbit" | "halbit" | "herc20",
    betaProtocol: "hbit" | "halbit" | "herc20"
): SwapEventKind {
    switch (action) {
        case "init": {
            return null;
        }
        // "deploy" can only mean we are waiting for "herc20_deployed"
        case "deploy": {
            return "herc20_deployed";
        }

        // Alice is always funding and refunding on the alpha ledger, likewise Bob on the beta ledger
        case "fund":
        case "refund": {
            switch (role) {
                case "Alice": {
                    // @ts-ignore: Sad that TypeScript can't infer that.
                    return `${alphaProtocol}_${action}ed`;
                }
                case "Bob": {
                    // @ts-ignore: Sad that TypeScript can't infer that.
                    return `${betaProtocol}_${action}ed`;
                }
                default:
                    throw new Error(
                        `Who is ${role}? We expected either Alice or Bob!`
                    );
            }
        }
        // Alice is always redeeming on the beta ledger, likewise Bob on the alpha ledger
        case "redeem": {
            switch (role) {
                case "Alice": {
                    // @ts-ignore: Sad that TypeScript can't infer that.
                    return `${betaProtocol}_${action}ed`;
                }
                case "Bob": {
                    // @ts-ignore: Sad that TypeScript can't infer that.
                    return `${alphaProtocol}_${action}ed`;
                }
                default:
                    throw new Error(
                        `Who is ${role}? We expected either Alice or Bob!`
                    );
            }
        }
    }
}

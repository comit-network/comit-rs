import chai, { expect } from "chai";
import chaiAsPromised from "chai-as-promised";
import { Cnd, ComitClient, Swap } from "comit-sdk";
import { parseEther } from "ethers/utils";
import { Logger } from "log4js";
import { Asset, AssetKind } from "../asset";
import { Ledger, LedgerKind } from "../ledger";
import { Wallets } from "../wallets";
import { Actors } from "./index";

chai.use(chaiAsPromised);

export class Actor {
    public static defaultActionConfig = {
        timeout: 5000,
        tryInterval: 100,
    };

    public actors: Actors;
    public wallets: Wallets;

    private comitClient: ComitClient;
    private readonly cnd: Cnd;
    private swap: Swap;
    private readonly startingBalances: Map<AssetKind, number>;
    private readonly expectedBalanceChanges: Map<AssetKind, number>;
    private readonly logger: Logger;

    constructor(loggerFactory: () => Logger, cndEndpoint: string) {
        this.logger = loggerFactory();
        this.logger.level = "debug";

        this.wallets = new Wallets({});
        this.cnd = new Cnd(cndEndpoint);

        this.startingBalances = new Map();
        this.expectedBalanceChanges = new Map();
        this.logger.info("Created new actor at %s", cndEndpoint);
    }

    public async sendRequest(
        alphaAssetKind: AssetKind,
        betaAssetKind: AssetKind
    ) {
        // By default, we will send the swap request to bob
        const to = this.actors.bob;

        this.logger.info("Sending swap request");

        const alphaLedger = defaultLedgerDescriptionForAsset(alphaAssetKind);
        const alphaAsset = defaultAssetDescriptionForAsset(alphaAssetKind);

        this.logger.debug(
            "Derived %o from asset %s",
            alphaLedger,
            alphaAssetKind
        );
        this.logger.debug(
            "Derived %o from asset %s",
            alphaAsset,
            alphaAssetKind
        );

        const betaLedger = defaultLedgerDescriptionForAsset(betaAssetKind);
        const betaAsset = defaultAssetDescriptionForAsset(betaAssetKind);

        this.logger.debug(
            "Derived %o from asset %s",
            betaLedger,
            betaAssetKind
        );
        this.logger.debug("Derived %o from asset %s", betaAsset, betaAssetKind);

        await this.initializeDependencies([alphaLedger.name, betaLedger.name]);
        await to.initializeDependencies([alphaLedger.name, betaLedger.name]);

        await this.setStartingBalance([
            alphaAsset,
            { name: betaAsset.name, quantity: 0 },
        ]);
        await to.setStartingBalance([
            { name: alphaAsset.name, quantity: 0 },
            betaAsset,
        ]);

        this.expectedBalanceChanges.set(betaAssetKind, betaAsset.quantity);
        to.expectedBalanceChanges.set(alphaAssetKind, alphaAsset.quantity);

        const comitClient: ComitClient = this.getComitClient();

        const payload = {
            alpha_ledger: alphaLedger,
            beta_ledger: betaLedger,
            alpha_asset: alphaAsset,
            beta_asset: betaAsset,
            alpha_expiry: defaultExpiryTimes().alpha_expiry,
            beta_expiry: defaultExpiryTimes().beta_expiry,
            peer: {
                peer_id: await to.cnd.getPeerId(),
                address_hint: await to.cnd
                    .getPeerListenAddresses()
                    .then(addresses => addresses[0]),
            },
            ...(await this.additionalIdentities(alphaAssetKind, betaAssetKind)),
        };

        this.swap = await comitClient.sendSwap(payload);
        to.swap = new Swap(
            to.wallets.bitcoin.inner,
            to.wallets.ethereum.inner,
            to.cnd,
            this.swap.self
        );
        this.logger.debug("Created new swap at %s", this.swap.self);
    }

    public async accept() {
        if (!this.swap) {
            throw new Error("Cannot accept inexistent swap");
        }

        this.swap.accept(Actor.defaultActionConfig);
    }

    public async fund() {
        if (!this.swap) {
            throw new Error("Cannot fund inexistent swap");
        }

        this.logger.debug("Funding as part of swap @ %s", this.swap.self);
        await this.swap.fund(Actor.defaultActionConfig);
    }

    public async redeem() {
        if (!this.swap) {
            throw new Error("Cannot redeem inexistent swap");
        }

        this.logger.debug("Redeeming as part of swap @ %s", this.swap.self);
        await this.swap.redeem(Actor.defaultActionConfig);
    }

    public async assertSwapped() {
        this.logger.debug("Checking if swap @ %s is done", this.swap.self);

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
            const expectedBalance =
                this.startingBalances.get(assetKind) + expectedBalanceChange;
            const maximumFee = wallet.MaximumFee;

            await expect(wallet.getBalance()).to.eventually.be.at.least(
                expectedBalance - maximumFee
            );
        }
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

    private async initializeDependencies<K extends LedgerKind>(
        ledgerNames: K[]
    ) {
        for (const ledgerName of ledgerNames) {
            await this.wallets.initializeForLedger(ledgerName);
        }

        this.comitClient = new ComitClient(
            this.wallets.getWalletForLedger("bitcoin").inner,
            this.wallets.getWalletForLedger("ethereum").inner,
            this.cnd
        );
    }

    private getComitClient(): ComitClient {
        if (!this.comitClient) {
            throw new Error("ComitClient is not initialised");
        }

        return this.comitClient;
    }

    private async setStartingBalance(assets: Asset[]) {
        for (const asset of assets) {
            if (asset.quantity === 0) {
                this.startingBalances.set(asset.name, 0);
                continue;
            }

            const ledger = defaultLedgerDescriptionForAsset(asset.name).name;

            this.logger.debug("Minting %s on %s", asset.name, ledger);
            await this.wallets.getWalletForLedger(ledger).mint(asset);

            const balance = await this.wallets[
                defaultLedgerDescriptionForAsset(asset.name).name
            ].getBalance();

            this.logger.debug("Starting %s balance: ", asset.name, balance);
            this.startingBalances.set(asset.name, balance);
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
    }
}

function defaultExpiryTimes() {
    const alphaExpiry = new Date("2080-06-11T23:00:00Z").getTime() / 1000;
    const betaExpiry = new Date("2080-06-11T13:00:00Z").getTime() / 1000;

    return {
        alpha_expiry: alphaExpiry,
        beta_expiry: betaExpiry,
    };
}

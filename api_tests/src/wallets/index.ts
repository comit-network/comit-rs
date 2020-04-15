import { Asset, BigNumber } from "comit-sdk";
import { HarnessGlobal, sleep } from "../utils";
import { BitcoinWallet } from "./bitcoin";
import { EthereumWallet } from "./ethereum";
import { LightningWallet } from "./lightning";
import { Logger } from "log4js";
import { ActorNames } from "../actors/actor";

declare var global: HarnessGlobal;

interface AllWallets {
    bitcoin?: BitcoinWallet;
    ethereum?: EthereumWallet;
    lightning?: LightningWallet;
}

export interface Wallet {
    MaximumFee: number;
    mint(asset: Asset): Promise<void>;
    getBalanceByAsset(asset: Asset): Promise<BigNumber>;
    getBlockchainTime(): Promise<number>;
}

export class Wallets {
    constructor(private readonly wallets: AllWallets) {}

    get bitcoin(): BitcoinWallet {
        return this.getWalletForLedger("bitcoin");
    }

    get ethereum(): EthereumWallet {
        return this.getWalletForLedger("ethereum");
    }

    get lightning(): LightningWallet {
        return this.getWalletForLedger("lightning");
    }

    public getWalletForLedger<K extends keyof AllWallets>(
        name: K
    ): AllWallets[K] {
        const wallet = this.wallets[name];

        if (!wallet) {
            throw new Error(`Wallet for ${name} is not initialised`);
        }

        return wallet;
    }

    public async initializeForLedger<K extends keyof AllWallets>(
        name: K,
        logger: Logger,
        actor?: ActorNames
    ) {
        switch (name) {
            case "ethereum":
                this.wallets.ethereum = new EthereumWallet(
                    global.ledgerConfigs.ethereum.rpc_url,
                    logger,
                    global.parityLockDir
                );
                break;
            case "bitcoin":
                this.wallets.bitcoin = await BitcoinWallet.newInstance(
                    global.ledgerConfigs.bitcoin,
                    logger
                );
                break;
            case "lightning":
                switch (actor) {
                    case "alice": {
                        this.wallets.lightning = global.lndWallets.alice;
                        break;
                    }
                    case "bob": {
                        this.wallets.lightning = global.lndWallets.bob;
                        break;
                    }
                    default: {
                        throw new Error(
                            `Cannot initialize Lightning wallet for actor: '${actor}'`
                        );
                    }
                }
        }
    }

    public async close(): Promise<void[]> {
        const tasks = [];

        if (this.wallets.lightning) {
            tasks.push(this.wallets.lightning.close());
        }

        if (this.wallets.bitcoin) {
            tasks.push(this.wallets.bitcoin.close());
        }

        return Promise.all(tasks);
    }
}

export async function pollUntilMinted(
    wallet: Wallet,
    minimumBalance: BigNumber,
    asset: Asset
): Promise<void> {
    const currentBalance = await wallet.getBalanceByAsset(asset);
    if (currentBalance.gte(minimumBalance)) {
        return;
    } else {
        await sleep(500);

        return pollUntilMinted(wallet, minimumBalance, asset);
    }
}

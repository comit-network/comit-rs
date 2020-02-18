import { BigNumber } from "comit-sdk";
import { HarnessGlobal } from "../../lib/util";
import { Asset } from "comit-sdk";
import { sleep } from "../utils";
import { BitcoinWallet } from "./bitcoin";
import { EthereumWallet } from "./ethereum";
import { LightningWallet } from "./lightning";
import { E2ETestActorConfig } from "../../lib/config";
import { Logger } from "log4js";

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
        logDir: string,
        actorConfig: E2ETestActorConfig
    ) {
        switch (name) {
            case "ethereum":
                this.wallets.ethereum = new EthereumWallet(
                    global.ledgerConfigs.ethereum
                );
                break;
            case "bitcoin":
                if (!this.wallets.bitcoin) {
                    this.wallets.bitcoin = await BitcoinWallet.newInstance(
                        global.ledgerConfigs.bitcoin
                    );
                }
                break;
            case "lightning":
                if (!this.wallets.bitcoin) {
                    this.wallets.bitcoin = await BitcoinWallet.newInstance(
                        global.ledgerConfigs.bitcoin
                    );
                }
                this.wallets.lightning = await LightningWallet.newInstance(
                    this.wallets.bitcoin,
                    logger,
                    logDir,
                    actorConfig
                );
                break;
        }
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

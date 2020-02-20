import { BigNumber } from "comit-sdk";
import { Asset } from "comit-sdk";
import { HarnessGlobal, sleep } from "../utils";
import { BitcoinWallet } from "./bitcoin";
import { EthereumWallet } from "./ethereum";
import { LightningWallet } from "./lightning";
import { Logger } from "log4js";
import { E2ETestActorConfig } from "../config";

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
                this.wallets.bitcoin = await BitcoinWallet.newInstance(
                    global.ledgerConfigs.bitcoin
                );
                break;
            case "lightning":
                this.wallets.lightning = await LightningWallet.newInstance(
                    await BitcoinWallet.newInstance(
                        global.ledgerConfigs.bitcoin
                    ),
                    logger,
                    logDir,
                    global.ledgerConfigs.bitcoin.dataDir,
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

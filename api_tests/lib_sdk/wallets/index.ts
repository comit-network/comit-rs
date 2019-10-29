import { HarnessGlobal } from "../../lib/util";
import { Asset } from "../asset";
import { sleep } from "../utils";
import { BitcoinWallet } from "./bitcoin";
import { EthereumWallet } from "./ethereum";

declare var global: HarnessGlobal;

interface AllWallets {
    bitcoin?: BitcoinWallet;
    ethereum?: EthereumWallet;
}

export interface Wallet {
    MaximumFee: number;
    mint(asset: Asset): Promise<void>;
    getBalance(): Promise<number>;
}

export class Wallets {
    constructor(private readonly wallets: AllWallets) {}

    get bitcoin(): BitcoinWallet {
        return this.getWalletForLedger("bitcoin");
    }

    get ethereum(): EthereumWallet {
        return this.getWalletForLedger("ethereum");
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

    public async initializeForLedger<K extends keyof AllWallets>(name: K) {
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
        }
    }
}

export async function pollUntilMinted(
    wallet: Wallet,
    minimumBalance: string
): Promise<void> {
    const currentBalance = await wallet.getBalance();

    if (currentBalance.toString() >= minimumBalance) {
        return;
    } else {
        await sleep(500);

        return pollUntilMinted(wallet, minimumBalance);
    }
}

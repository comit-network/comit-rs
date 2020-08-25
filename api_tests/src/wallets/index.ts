import { sleep } from "../utils";
import { BitcoinWallet } from "./bitcoin";
import { EthereumWallet } from "./ethereum";
import { Asset } from "../asset";
import { LightningWallet } from "./lightning";
import { Logger } from "log4js";

interface AllWallets {
    bitcoin?: BitcoinWallet;
    ethereum?: EthereumWallet;
    lightning?: LightningWallet;
}

export interface Wallet {
    MaximumFee: number;
    mint(asset: Asset): Promise<void>;
    getBalanceByAsset(asset: Asset): Promise<bigint>;
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
}

export async function pollUntilMinted(
    wallet: Wallet,
    minimumBalance: BigInt,
    asset: Asset
): Promise<void> {
    const currentBalance = await wallet.getBalanceByAsset(asset);
    if (currentBalance >= minimumBalance) {
        return;
    } else {
        await sleep(500);

        return pollUntilMinted(wallet, minimumBalance, asset);
    }
}

export function newBitcoinStubWallet(logger: Logger): BitcoinWallet {
    return newStubWallet(
        {
            address: () =>
                Promise.resolve("bcrt1qq7pflkfujg6dq25n73n66yjkvppq6h9caklrhz"),
        },
        logger
    );
}

export function newEthereumStubWallet(logger: Logger): EthereumWallet {
    return newStubWallet(
        {
            account: () => "0x00a329c0648769a73afac7f9381e08fb43dbea72",
        },
        logger
    );
}

export function newLightningStubWallet(logger: Logger): LightningWallet {
    return newStubWallet(
        {
            pubkey: () =>
                Promise.resolve(
                    "02ed138aaed50d2d597f6fe8d30759fd3949fe73fdf961322713f1c19e10036a06"
                ),
        },
        logger
    );
}

function newStubWallet<W extends Wallet, T extends Partial<W>>(
    stubs: T,
    logger: Logger
): W {
    const stubWallet: Partial<W> = {
        ...stubs,
        mint: (_: Asset) => {
            logger.warn("StubWallet doesn't mint anything");
        },
        getBalanceByAsset: async (asset: Asset) => {
            logger.warn(
                "StubWallet always returns 0 balance for asset",
                asset.name
            );

            return Promise.resolve(0);
        },
    };

    return (stubWallet as unknown) as W;
}

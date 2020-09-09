import { BitcoinWallet } from "./bitcoin";
import { EthereumWallet } from "./ethereum";
import { LightningWallet } from "./lightning";

export interface AllWallets {
    bitcoin?: BitcoinWallet;
    ethereum?: EthereumWallet;
    lightning?: LightningWallet;
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

export function newBitcoinStubWallet(): BitcoinWallet {
    return newStubWallet({
        MaximumFee: BigInt(0),
        getAddress: () =>
            Promise.resolve("bcrt1qq7pflkfujg6dq25n73n66yjkvppq6h9caklrhz"),
        getBalance: () => Promise.resolve(BigInt(0)),
        mint: (_satoshis: bigint) => Promise.resolve(),
    });
}

export function newEthereumStubWallet(): EthereumWallet {
    return newStubWallet({
        getAccount: () => "0x00a329c0648769a73afac7f9381e08fb43dbea72",
        getErc20Balance: (
            _contractAddress: string,
            _decimals?: number
        ): Promise<bigint> => Promise.resolve(BigInt(0)),
        mintErc20: (_quantity: bigint, _tokenContract: string) =>
            Promise.resolve(),
    });
}

export function newLightningStubWallet(): LightningWallet {
    return newStubWallet({
        getPubkey: () =>
            Promise.resolve(
                "02ed138aaed50d2d597f6fe8d30759fd3949fe73fdf961322713f1c19e10036a06"
            ),
        getBalance: () => Promise.resolve(BigInt(0)),
    });
}

function newStubWallet<W, T extends Partial<W>>(stubs: T): W {
    return (stubs as unknown) as W;
}

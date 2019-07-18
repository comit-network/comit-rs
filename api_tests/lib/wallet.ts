import { BitcoinNodeConfig, BitcoinWallet } from "./bitcoin";
import { EthereumNodeConfig, EthereumWallet } from "./ethereum";

export interface WalletConfig {
    ethereumNodeConfig?: EthereumNodeConfig;
    bitcoinNodeConfig?: BitcoinNodeConfig;

    addressForIncomingBitcoinPayments?: string;
}

export class Wallet {
    public owner: string;
    private readonly ethWallet: EthereumWallet;
    private readonly btcWallet: BitcoinWallet;

    constructor(owner: string, config: WalletConfig) {
        this.owner = owner;

        if (config.ethereumNodeConfig) {
            this.ethWallet = new EthereumWallet(config.ethereumNodeConfig);
        }

        if (config.bitcoinNodeConfig) {
            this.btcWallet = new BitcoinWallet(
                config.bitcoinNodeConfig,
                config.addressForIncomingBitcoinPayments
            );
        }
    }

    public eth() {
        return this.ethWallet;
    }

    public btc() {
        return this.btcWallet;
    }
}

import { BitcoinNodeConfig, BitcoinWallet } from "./bitcoin";
import { EthereumNodeConfig, EthereumWallet } from "./ethereum";

export interface WalletConfig {
    ethereumNodeConfig?: EthereumNodeConfig;
    bitcoinNodeConfig?: BitcoinNodeConfig;

    addressForIncomingBitcoinPayments?: string;
}

export class Wallet {
    public owner: string;
    public _ethWallet: EthereumWallet;
    public _btcWallet: BitcoinWallet;

    constructor(owner: string, config: WalletConfig) {
        this.owner = owner;

        if (config.ethereumNodeConfig) {
            this._ethWallet = new EthereumWallet(config.ethereumNodeConfig);
        }

        if (config.bitcoinNodeConfig) {
            this._btcWallet = new BitcoinWallet(
                config.bitcoinNodeConfig,
                config.addressForIncomingBitcoinPayments
            );
        }
    }

    public eth() {
        return this._ethWallet;
    }

    public btc() {
        return this._btcWallet;
    }
}

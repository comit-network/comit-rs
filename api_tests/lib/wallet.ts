import { BitcoinWallet, BitcoinNodeConfig } from "./bitcoin";
import { EthereumNodeConfig, EthereumWallet } from "./ethereum";

export interface WalletConfig {
    ethereumNodeConfig?: EthereumNodeConfig;
    bitcoinNodeConfig?: BitcoinNodeConfig;

    addressForIncomingBitcoinPayments?: string;
}

export class Wallet {
    owner: string;
    _ethWallet: EthereumWallet;
    _btcWallet: BitcoinWallet;

    constructor(owner: string, config: WalletConfig) {
        this.owner = owner;
        this._ethWallet = new EthereumWallet(config.ethereumNodeConfig);
        this._btcWallet = new BitcoinWallet(
            config.bitcoinNodeConfig,
            config.addressForIncomingBitcoinPayments
        );
    }

    eth() {
        return this._ethWallet;
    }

    btc() {
        return this._btcWallet;
    }
}

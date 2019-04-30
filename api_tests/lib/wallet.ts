import { BitcoinWallet, BtcConfig } from "./bitcoin";
import { EthConfig, EthereumWallet } from "./ethereum";

export interface WalletConfig {
    ethConfig?: EthConfig;
    btcConfig?: BtcConfig;
}

export class Wallet {
    owner: string;
    _config: WalletConfig;
    _ethWallet: EthereumWallet;
    _btcWallet: BitcoinWallet;

    constructor(owner: string, config: WalletConfig) {
        this.owner = owner;
        this._config = config;
    }

    eth() {
        if (!this._ethWallet) {
            this._ethWallet = new EthereumWallet(this._config.ethConfig);
        }
        return this._ethWallet;
    }

    btc() {
        if (!this._btcWallet) {
            this._btcWallet = new BitcoinWallet(this._config.btcConfig);
        }
        return this._btcWallet;
    }
}

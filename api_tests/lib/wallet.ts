import { createBitcoinWallet, BitcoinWallet } from "./bitcoin";
import { createEthereumWallet, EthereumWallet, EthConfig } from "./ethereum";

export interface WalletConfig {
    ethConfig: EthConfig;
}

class Wallet {
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
            this._ethWallet = createEthereumWallet(this._config.ethConfig);
        }
        return this._ethWallet;
    }

    btc() {
        if (!this._btcWallet) {
            this._btcWallet = createBitcoinWallet();
        }
        return this._btcWallet;
    }
}

export function create(owner: string, config: WalletConfig) {
    return new Wallet(owner, config);
}

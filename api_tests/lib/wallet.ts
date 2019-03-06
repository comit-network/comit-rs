import { createBitcoinWallet, BitcoinWallet } from "./bitcoin";
import { createEthereumWallet, EthereumWallet, IEthConfig } from "./ethereum";

export interface IWalletConfig {
    ethConfig: IEthConfig;
}

class Wallet {
    owner: string;
    _config: IWalletConfig;
    _ethWallet: EthereumWallet;
    _btcWallet: BitcoinWallet;

    constructor(owner: string, config: IWalletConfig) {
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

export function create(owner: string, config: IWalletConfig) {
    return new Wallet(owner, config);
}

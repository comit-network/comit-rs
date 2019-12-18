import { BitcoinWallet } from "./bitcoin";
import { EthereumWallet } from "./ethereum";
import { LedgerConfig } from "./ledger_runner";

export interface WalletConfig {
    ledgerConfig: LedgerConfig;
    addressForIncomingBitcoinPayments?: string;
}

export class Wallet {
    public owner: string;
    private readonly ethWallet: EthereumWallet;
    private readonly btcWallet: BitcoinWallet;

    constructor(owner: string, config: WalletConfig) {
        this.owner = owner;

        if (config.ledgerConfig.ethereum) {
            this.ethWallet = new EthereumWallet(config.ledgerConfig.ethereum);
        }

        if (config.ledgerConfig.bitcoin) {
            this.btcWallet = new BitcoinWallet(
                config.ledgerConfig.bitcoin,
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

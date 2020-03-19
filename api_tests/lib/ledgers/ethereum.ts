import { EthereumWallet } from "../wallets/ethereum";
import LedgerInstance from "./ledger_instance";
import { Logger } from "log4js";

/**
 * An instance of the Ethereum ledger for use in the e2e tests.
 *
 * This class is compatible with anything that implements {@link EthereumInstance}.
 *
 * Some of the e2e tests need an ERC20 token deployed to work properly.
 * This class takes care of deploying such contract after the Ethereum
 * blockchain is up and running.
 *
 * This class serves as an abstraction layer on top of Ethereum, regardless
 * of which implementation is used (Docker container, parity, geth, etc).
 */
export default class EthereumLedger implements LedgerInstance {
    public static async start(instance: EthereumInstance, logger: Logger) {
        await instance.start();

        const rpcUrl = instance.rpcUrl;

        logger.info("Ethereum node started at", rpcUrl);

        const erc20Wallet = new EthereumWallet(rpcUrl, logger);
        const erc20TokenContract = await erc20Wallet.deployErc20TokenContract();

        logger.info("ERC20 token contract deployed at", erc20TokenContract);

        return new EthereumLedger(instance, erc20TokenContract);
    }

    constructor(
        private readonly instance: EthereumInstance,
        private readonly erc20TokenContract: string
    ) {}

    public async stop(): Promise<void> {
        await this.instance.stop();
    }

    public get config(): EthereumNodeConfig {
        return {
            rpc_url: this.instance.rpcUrl,
            tokenContract: this.erc20TokenContract,
        };
    }
}

export interface EthereumInstance {
    rpcUrl: string;

    start(): Promise<void>;
    stop(): Promise<void>;
}

export interface EthereumNodeConfig {
    rpc_url: string;
    tokenContract: string;
}

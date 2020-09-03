import * as tmp from "tmp";
import { LedgerConfig } from "./utils";
import getPort from "get-port";
import {
    LightningNodeConfig,
    BitcoinNodeConfig,
    EthereumNodeConfig,
} from "./ledgers";
import { Logger } from "log4js";
import { ActorName } from "./actors";

export interface CndConfigFile {
    http_api: HttpApi;
    data?: { dir: string };
    network: { listen: string[] };
    logging: { level: string };
    bitcoin?: BitcoinConfig;
    ethereum?: EthereumConfig;
    lightning?: LightningConfig;
}

export interface HttpApi {
    socket: string;
}

export class E2ETestActorConfig {
    public readonly data: string;

    public static async for(name: ActorName, logger: Logger) {
        return new E2ETestActorConfig(
            await getPort(),
            await getPort(),
            name,
            logger
        );
    }

    constructor(
        public readonly httpApiPort: number,
        public readonly comitPort: number,
        public readonly name: ActorName,
        private readonly logger: Logger
    ) {
        this.httpApiPort = httpApiPort;
        this.comitPort = comitPort;

        const tmpobj = tmp.dirSync();
        tmpobj.removeCallback(); // Manual cleanup

        this.data = tmpobj.name;
    }

    public generateCndConfigFile(ledgerConfig: LedgerConfig): CndConfigFile {
        return {
            http_api: {
                socket: `0.0.0.0:${this.httpApiPort}`,
            },
            data: {
                dir: this.data,
            },
            network: {
                listen: [`/ip4/0.0.0.0/tcp/${this.comitPort}`],
            },
            logging: {
                level: "Trace",
            },
            ...this.createLedgerConnectors(ledgerConfig),
        };
    }

    private createLedgerConnectors(ledgerConfig: LedgerConfig): LedgerConfigs {
        const config: LedgerConfigs = {};

        if (ledgerConfig.bitcoin) {
            config.bitcoin = bitcoinConnector(ledgerConfig.bitcoin);
        }

        if (ledgerConfig.ethereum) {
            config.ethereum = ethereumConnector(ledgerConfig.ethereum);
        }

        switch (this.name) {
            case "Alice": {
                if (ledgerConfig.aliceLnd) {
                    config.lightning = lightningConnector(
                        ledgerConfig.aliceLnd
                    );
                }
                break;
            }
            case "Bob": {
                if (ledgerConfig.bobLnd) {
                    config.lightning = lightningConnector(ledgerConfig.bobLnd);
                }
                break;
            }
            case "Carol":
                {
                    this.logger.warn(
                        "generating lnd config for carol is not supported at this stage"
                    );
                }
                break;
        }

        return config;
    }
}

interface LedgerConfigs {
    bitcoin?: BitcoinConfig;
    ethereum?: EthereumConfig;
    lightning?: LightningConfig;
}

interface Geth {
    node_url: string;
}

interface EthereumConfig {
    chain_id: number;
    geth: Geth;
    tokens: Tokens;
}

interface Tokens {
    dai: string;
}

interface Bitcoind {
    node_url: string;
}

interface BitcoinConfig {
    network: string;
    bitcoind: Bitcoind;
}

interface Lnd {
    rest_api_url: string;
    dir: string;
}

interface LightningConfig {
    network: string;
    lnd: Lnd;
}

function bitcoinConnector(nodeConfig: BitcoinNodeConfig): BitcoinConfig {
    return {
        bitcoind: {
            node_url: nodeConfig.rpcUrl,
        },
        network: nodeConfig.network,
    };
}

function ethereumConnector(nodeConfig: EthereumNodeConfig): EthereumConfig {
    return {
        chain_id: nodeConfig.chain_id,
        geth: {
            node_url: nodeConfig.rpc_url,
        },
        tokens: {
            dai: nodeConfig.tokenContract,
        },
    };
}

function lightningConnector(nodeConfig: LightningNodeConfig): LightningConfig {
    return {
        network: "regtest",
        lnd: {
            rest_api_url: `https://localhost:${nodeConfig.restPort}`,
            dir: nodeConfig.dataDir,
        },
    };
}

import * as tmp from "tmp";
import {
    LedgerConfig,
    BitcoinNodeConfig,
    EthereumNodeConfig,
} from "./ledgers/ledger_runner";

export interface CndConfigFile {
    http_api: HttpApi;
    data?: { dir: string };
    network: { listen: string[] };
    logging: { level: string };
}

export interface HttpApi {
    socket: string;
}

export class E2ETestActorConfig {
    public readonly data: string;

    constructor(
        public readonly httpApiPort: number,
        public readonly comitPort: number,
        public readonly name: string
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
            ...createLedgerConnectors(ledgerConfig),
        };
    }
}

interface LedgerConnectors {
    bitcoin?: BitcoinConnector;
    ethereum?: EthereumConnector;
}

interface Parity {
    node_url: string;
}

interface EthereumConnector {
    chain_id: number;
    parity: Parity;
}

interface Bitcoind {
    node_url: string;
}

interface BitcoinConnector {
    network: string;
    bitcoind: Bitcoind;
}

function createLedgerConnectors(ledgerConfig: LedgerConfig): LedgerConnectors {
    const config: LedgerConnectors = {};

    if (ledgerConfig.bitcoin) {
        config.bitcoin = bitcoinConnector(ledgerConfig.bitcoin);
    }

    if (ledgerConfig.ethereum) {
        config.ethereum = ethereumConnector(ledgerConfig.ethereum);
    }

    return config;
}

function bitcoinConnector(nodeConfig: BitcoinNodeConfig): BitcoinConnector {
    return {
        bitcoind: {
            node_url: `http://${nodeConfig.host}:${nodeConfig.rpcPort}`,
        },
        network: nodeConfig.network,
    };
}

function ethereumConnector(nodeConfig: EthereumNodeConfig): EthereumConnector {
    return {
        chain_id: 17,
        parity: {
            node_url: nodeConfig.rpc_url,
        },
    };
}

import * as tmp from "tmp";
import { BitcoinNodeConfig } from "./ledgers/bitcoin";
import { LedgerConfig } from "./ledgers/ledger_runner";
import { EthereumNodeConfig } from "./ledgers/ethereum";

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
        public readonly name: string,
        public readonly lndP2pPort: number,
        public readonly lndRpcPort: number
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

export const ALICE_CONFIG = new E2ETestActorConfig(
    8000,
    9938,
    "alice",
    59736,
    50009
);
export const BOB_CONFIG = new E2ETestActorConfig(
    8010,
    9939,
    "bob",
    59737,
    50010
);
export const CHARLIE_CONFIG = new E2ETestActorConfig(
    8020,
    8021,
    "charlie",
    59738,
    50011
);

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

export const CND_CONFIGS: {
    [actor: string]: E2ETestActorConfig | undefined;
} = {
    alice: ALICE_CONFIG,
    bob: BOB_CONFIG,
    charlie: CHARLIE_CONFIG,
};

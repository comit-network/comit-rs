import { ChildProcess, spawn } from "child_process";
import * as path from "path";
import { promises as asyncFs } from "fs";
import getPort from "get-port";
import { Logger } from "log4js";
import waitForLogMessage from "./wait_for_log_message";
import findCacheDir from "find-cache-dir";
import download from "download";
import { platform } from "os";
import { crashListener } from "./crash_listener";
import { BitcoinNodeConfig, LedgerInstance } from "./index";
import { existsAsync } from "./async_fs";

export class BitcoindInstance implements LedgerInstance {
    private process: ChildProcess;
    private username: string;
    private password: string;

    public static async new(
        dataDir: string,
        pidFile: string,
        logger: Logger
    ): Promise<BitcoindInstance> {
        return new BitcoindInstance(
            dataDir,
            pidFile,
            logger,
            await getPort({ port: 18444 }),
            await getPort({ port: 18443 }),
            await getPort({ port: 28332 }),
            await getPort({ port: 28333 })
        );
    }

    constructor(
        private readonly dataDir: string,
        private readonly pidFile: string,
        private readonly logger: Logger,
        public readonly p2pPort: number,
        public readonly rpcPort: number,
        public readonly zmqPubRawBlockPort: number,
        public readonly zmqPubRawTxPort: number
    ) {}

    public async start() {
        const bin = await this.findBinary("0.20.0");

        this.logger.info("Using binary", bin);

        await this.createConfigFile(this.dataDir);

        this.process = spawn(bin, [`-datadir=${this.dataDir}`], {
            cwd: this.dataDir,
            stdio: "ignore",
        });

        this.process.once(
            "exit",
            crashListener(this.process.pid, "bitcoind", this.logPath())
        );

        await waitForLogMessage(this.logPath(), "init message: Done loading");

        const result = await asyncFs.readFile(
            path.join(this.dataDir, "regtest", ".cookie"),
            "utf8"
        );
        const [username, password] = result.split(":");

        this.username = username;
        this.password = password;

        this.logger.info("bitcoind started with PID", this.process.pid);

        await asyncFs.writeFile(this.pidFile, this.process.pid.toString(), {
            encoding: "utf-8",
        });
    }

    public get config(): BitcoinNodeConfig {
        return {
            network: "regtest",
            host: "localhost",
            rpcPort: this.rpcPort,
            p2pPort: this.p2pPort,
            username: this.username,
            password: this.password,
            dataDir: this.getDataDir(),
            rpcUrl: `http://localhost:${this.rpcPort}`,
        };
    }

    private async findBinary(version: string): Promise<string> {
        const envOverride = process.env.BITCOIND_BIN;

        if (envOverride) {
            this.logger.info(
                "Overriding bitcoind bin with BITCOIND_BIN: ",
                envOverride
            );

            return envOverride;
        }

        const archiveName = `bitcoin-core-${version}`;

        const cacheDir = findCacheDir({
            name: archiveName,
            create: true,
            thunk: true,
        });

        // This path depends on the directory structure inside the archive
        const binaryPath = cacheDir(`bitcoin-${version}`, "bin", "bitcoind");

        try {
            await existsAsync(binaryPath);
            return binaryPath;
        } catch (e) {
            // Continue and download the file
        }

        const url = downloadUrlFor(version);

        this.logger.info(
            "Binary for version ",
            version,
            " not found at ",
            binaryPath,
            ", downloading from ",
            url
        );

        const destination = cacheDir("");
        await download(url, destination, {
            decompress: true,
            extract: true,
            filename: archiveName,
        });

        this.logger.info("Download completed");

        return binaryPath;
    }

    private logPath() {
        return path.join(this.dataDir, "regtest", "debug.log");
    }

    public getDataDir() {
        return this.dataDir;
    }

    private async createConfigFile(dataDir: string) {
        const output = `regtest=1
server=1
printtoconsole=1
rpcallowip=0.0.0.0/0
debug=1
rest=1
acceptnonstdtxn=0
zmqpubrawblock=tcp://127.0.0.1:${this.zmqPubRawBlockPort}
zmqpubrawtx=tcp://127.0.0.1:${this.zmqPubRawTxPort}
fallbackfee=0.0002
txindex=1

[regtest]
bind=0.0.0.0:${this.p2pPort}
rpcbind=0.0.0.0:${this.rpcPort}
`;
        const config = path.join(dataDir, "bitcoin.conf");
        await asyncFs.writeFile(config, output);
    }
}

function downloadUrlFor(version: string) {
    switch (platform()) {
        case "darwin":
            return `https://bitcoincore.org/bin/bitcoin-core-${version}/bitcoin-${version}-osx64.tar.gz`;
        case "linux":
            return `https://bitcoincore.org/bin/bitcoin-core-${version}/bitcoin-${version}-x86_64-linux-gnu.tar.gz`;
        default:
            throw new Error(`Unsupported platform ${platform()}`);
    }
}

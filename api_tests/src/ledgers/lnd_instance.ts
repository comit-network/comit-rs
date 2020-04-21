import { ChildProcess, spawn } from "child_process";
import { existsAsync, waitUntilFileExists, writeFileAsync } from "../utils";
import * as path from "path";
import getPort from "get-port";
import waitForLogMessage from "../wait_for_log_message";
import { Lnd } from "comit-sdk";
import { Logger } from "log4js";
import { LightningNodeConfig, LedgerInstance } from "./index";
import findCacheDir from "find-cache-dir";
import download from "download";
import { platform } from "os";

export class LndInstance implements LedgerInstance {
    private process: ChildProcess;

    public static async new(
        dataDir: string,
        logger: Logger,
        bitcoindDataDir: string,
        pidFile: string
    ) {
        return new LndInstance(
            dataDir,
            logger,
            bitcoindDataDir,
            pidFile,
            await getPort(),
            await getPort(),
            await getPort()
        );
    }

    private constructor(
        private readonly dataDir: string,
        private readonly logger: Logger,
        private readonly bitcoindDataDir: string,
        private readonly pidFile: string,
        private readonly lndP2pPort: number,
        private readonly lndRpcPort: number,
        private readonly lndRestPort: number
    ) {}

    public async start() {
        await this.createConfigFile();

        await this.execBinary();

        const logFile = this.logPath();

        this.logger.debug("Waiting for lnd password RPC server");
        await waitForLogMessage(logFile, "RPCS: password RPC server listening");

        await this.initWallet();

        this.logger.debug("Waiting for lnd unlocked RPC server");
        await waitForLogMessage(logFile, "RPCS: RPC server listening");

        this.logger.debug(
            "Waiting for admin macaroon file to exist:",
            this.adminMacaroonPath()
        );
        await waitUntilFileExists(this.adminMacaroonPath());

        this.logger.debug("Waiting for lightning server to start");
        await waitForLogMessage(logFile, "[INF] BTCN: Server listening on ");

        this.logger.debug("lnd started with PID", this.process.pid);

        await writeFileAsync(this.pidFile, this.process.pid, {
            encoding: "utf-8",
        });
    }

    private async execBinary() {
        const bin = await this.findBinary("v0.9.1-beta");

        this.logger.debug(`Using binary ${bin}`);
        this.process = spawn(bin, ["--lnddir", this.dataDir], {
            stdio: ["ignore", "ignore", "ignore"], // stdin, stdout, stderr.  These are all logged already.
        });

        this.process.on("exit", (code: number, signal: number) => {
            this.logger.info(
                "lnd exited with code",
                code,
                "after signal",
                signal
            );
        });
    }

    private async initWallet() {
        const config = {
            server: this.grpcSocket,
            tls: this.tlsCertPath(),
        };
        this.logger.debug("Instantiating lnd connection:", config);
        const lnd = await Lnd.init(config);

        const { cipherSeedMnemonic } = await lnd.lnrpc.genSeed({
            seedEntropy: Buffer.alloc(16, this.lndP2pPort),
        });
        const walletPassword = Buffer.from("password", "utf8");
        this.logger.debug(
            "Initialize wallet",
            cipherSeedMnemonic,
            walletPassword
        );
        await lnd.lnrpc.initWallet({ cipherSeedMnemonic, walletPassword });
        this.logger.debug("Lnd wallet initialized!");
    }

    public isRunning() {
        return this.process != null;
    }

    public logPath() {
        return path.join(this.dataDir, "logs", "bitcoin", "regtest", "lnd.log");
    }

    public tlsCertPath() {
        return path.join(this.dataDir, "tls.cert");
    }

    public adminMacaroonPath() {
        return path.join(
            this.dataDir,
            "data",
            "chain",
            "bitcoin",
            "regtest",
            "admin.macaroon"
        );
    }

    get grpcSocket() {
        return `${this.grpcHost}:${this.grpcPort}`;
    }

    get grpcHost() {
        return "127.0.0.1";
    }

    get grpcPort() {
        return this.lndRpcPort;
    }

    get p2pSocket() {
        return `${this.p2pHost}:${this.p2pPort}`;
    }

    get p2pHost() {
        return "127.0.0.1";
    }

    get p2pPort() {
        return this.lndP2pPort;
    }

    get restPort() {
        return this.lndRestPort;
    }

    get config(): LightningNodeConfig {
        return {
            p2pSocket: this.p2pSocket,
            grpcSocket: this.grpcSocket,
            tlsCertPath: this.tlsCertPath(),
            macaroonPath: this.adminMacaroonPath(),
            restPort: this.restPort,
            dataDir: this.dataDir,
        };
    }

    private async createConfigFile() {
        const output = `[Application Options]
debuglevel=trace

; peer to peer port
listen=127.0.0.1:${this.lndP2pPort}

; gRPC
rpclisten=127.0.0.1:${this.lndRpcPort}

; REST interface
restlisten=127.0.0.1:${this.lndRestPort}

; Do not seek out peers on the network
nobootstrap=true

; Only wait 1 confirmation to open a channel
bitcoin.defaultchanconfs=1

[Bitcoin]

bitcoin.active=true
bitcoin.regtest=true
bitcoin.node=bitcoind

[Bitcoind]

bitcoind.dir=${this.bitcoindDataDir}
`;
        const config = path.join(this.dataDir, "lnd.conf");
        await writeFileAsync(config, output);
    }

    private async findBinary(version: string): Promise<string> {
        const envOverride = process.env.LND_BIN;

        if (envOverride) {
            this.logger.info("Overriding lnd bin with LND_BIN: ", envOverride);

            return envOverride;
        }

        const archiveName = `lnd-${version}`;

        const cacheDir = findCacheDir({
            name: archiveName,
            create: true,
            thunk: true,
        });

        // This path depends on the directory structure inside the archive
        const binaryPath = cacheDir(`lnd-${getArch()}-${version}`, "lnd");

        if (await existsAsync(binaryPath)) {
            return binaryPath;
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
}

function getArch(): string {
    switch (platform()) {
        case "darwin":
            return "darwin-amd64";
        case "linux":
            return "linux-amd64";
        default:
            throw new Error(`Unsupported platform ${platform()}`);
    }
}

function downloadUrlFor(version: string) {
    return `https://github.com/lightningnetwork/lnd/releases/download/${version}/lnd-${getArch()}-${version}.tar.gz`;
}

import { ChildProcess, spawn } from "child_process";
import { waitUntilFileExists, writeFileAsync } from "../utils";
import * as path from "path";
import getPort from "get-port";
import { LogReader } from "./log_reader";
import { Lnd } from "comit-sdk";
import whereis from "@wcjiang/whereis";
import { LightningInstance, LightningNodeConfig } from "./lightning";
import { Logger } from "log4js";

export class LndInstance implements LightningInstance {
    private process: ChildProcess;
    public lnd: Lnd;
    private publicKey?: string;

    public static async new(
        dataDir: string,
        name: string,
        logger: Logger,
        bitcoindDataDir: string
    ) {
        return new LndInstance(
            dataDir,
            name,
            logger,
            bitcoindDataDir,
            await getPort(),
            await getPort(),
            await getPort()
        );
    }

    private constructor(
        private readonly dataDir: string,
        private readonly name: string,
        private readonly logger: Logger,
        private readonly bitcoindDataDir: string,
        private readonly lndP2pPort: number,
        private readonly lndRpcPort: number,
        private readonly lndRestPort: number
    ) {}

    public async start() {
        await this.createConfigFile();

        await this.execBinary();

        this.logger.debug("Waiting for lnd log file to exist:", this.logPath());
        await waitUntilFileExists(this.logPath());

        this.logger.debug("Waiting for lnd password RPC server");
        await this.logReader().waitForLogMessage(
            "RPCS: password RPC server listening"
        );

        await this.initWallet();

        this.logger.debug("Waiting for lnd unlocked RPC server");
        await this.logReader().waitForLogMessage("RPCS: RPC server listening");

        this.logger.debug(
            "Waiting for admin macaroon file to exist:",
            this.adminMacaroonPath()
        );
        await waitUntilFileExists(this.adminMacaroonPath());

        this.logger.debug("Waiting for lnd to catch up with blocks");
        await this.logReader().waitForLogMessage(
            "LNWL: Done catching up block hashes"
        );

        await this.initAuthenticatedLndConnection();

        this.publicKey = (await this.lnd.lnrpc.getInfo()).identityPubkey;
        this.logger.info("lnd is ready:", this.publicKey);

        this.logger.debug("lnd started with PID", this.process.pid);
    }

    private async execBinary() {
        const bin = process.env.LND_BIN
            ? process.env.LND_BIN
            : await whereis("lnd");
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
            seedEntropy: Buffer.alloc(16, this.name),
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

    private async initAuthenticatedLndConnection() {
        const config = {
            server: this.grpcSocket,
            tls: this.tlsCertPath(),
            macaroonPath: this.adminMacaroonPath(),
        };

        this.lnd = await Lnd.init(config);
    }

    public async stop() {
        this.logger.debug("Stopping lnd instance");
        this.process.kill("SIGTERM");
        this.process = null;
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
            lnd: this.lnd,
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

    private logReader() {
        return new LogReader(this.logPath());
    }
}

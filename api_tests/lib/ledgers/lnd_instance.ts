import { ChildProcess, spawn } from "child_process";
import { mkdirAsync, waitUntilFileExists, writeFileAsync } from "../utils";
import * as path from "path";
import getPort from "get-port";
import { LogReader } from "./log_reader";
import { Lnd } from "comit-sdk";

export class LndInstance {
    private process: ChildProcess;
    private lndDir: string;
    public lnd: Lnd;
    // private publicKey?: string;

    public static async new(
        testLogDir: string,
        name: string,
        bitcoindDataDir: string
    ) {
        const lndP2pPort = await getPort();
        const lndRpcPort = await getPort();

        return new LndInstance(
            testLogDir,
            name,
            bitcoindDataDir,
            lndP2pPort,
            lndRpcPort
        );
    }

    private constructor(
        private readonly testLogDir: string,
        private readonly name: string,
        private readonly bitcoindDataDir: string,
        private readonly lndP2pPort: number,
        private readonly lndRpcPort: number
    ) {}

    public async start() {
        this.lndDir = path.join(this.testLogDir, "lnd-" + this.name);
        await mkdirAsync(this.lndDir, "755");
        await this.createConfigFile();

        this.execBinary();

        // this.logger.debug("Waiting for lnd log file to exist:", this.logPath());
        await waitUntilFileExists(this.logPath());

        // this.logger.debug("Waiting for lnd password RPC server");
        await this.logReader().waitForLogMessage(
            "RPCS: password RPC server listening"
        );

        await this.initWallet();

        // this.logger.debug("Waiting for lnd unlocked RPC server");
        await this.logReader().waitForLogMessage("RPCS: RPC server listening");

        // this.logger.debug(
        //     "Waiting for admin macaroon file to exist:",
        //     this.adminMacaroonPath()
        // );
        await waitUntilFileExists(this.adminMacaroonPath());

        // this.logger.debug("Waiting for lnd to catch up with blocks");
        await this.logReader().waitForLogMessage(
            "LNWL: Done catching up block hashes"
        );

        await this.initAuthenticatedLndConnection();

        // this.publicKey = (await this.lnd.lnrpc.getInfo()).identityPubkey;
        // this.logger.info("lnd is ready:", this.publicKey);

        return this;
    }

    private execBinary() {
        const bin = process.env.LND_BIN ? process.env.LND_BIN : "lnd";
        // this.logger.debug(`Using binary ${bin}`);
        this.process = spawn(bin, ["--lnddir", this.lndDir], {
            stdio: ["ignore", "ignore", "ignore"], // stdin, stdout, stderr.  These are all logged already.
        });
        // this.logger.debug(`Process spawned LND with PID ${this.process.pid}`);

        // this.process.on("exit", (code: number, signal: number) => {
        //     this.logger.debug(`lnd exited with ${code || `signal ${signal}`}`);
        // });
    }

    private async initWallet() {
        const config = {
            server: this.grpcSocket,
            tls: this.tlsCertPath(),
        };
        // this.logger.debug("Instantiating lnd connection:", config);
        const lnd = await Lnd.init(config);

        // this.logger.debug("Calling genSeed");
        const { cipherSeedMnemonic } = await lnd.lnrpc.genSeed({
            seedEntropy: this.name,
        });
        const walletPassword = Buffer.from("password", "utf8");
        // this.logger.debug(
        //     "Initialize wallet",
        //     cipherSeedMnemonic,
        //     walletPassword
        // );
        await lnd.lnrpc.initWallet({ cipherSeedMnemonic, walletPassword });
        // this.logger.debug("Wallet initialized!");
    }

    private async initAuthenticatedLndConnection() {
        const config = {
            server: this.grpcSocket,
            tls: this.tlsCertPath(),
            macaroonPath: this.adminMacaroonPath(),
        };
        // this.logger.debug("Instantiating lnd connection:", config);
        this.lnd = await Lnd.init(config);
    }

    public stop() {
        // this.logger.debug("Stopping lnd instance");
        this.process.kill("SIGTERM");
        this.process = null;
    }

    public isRunning() {
        return this.process != null;
    }

    public logPath() {
        return path.join(this.lndDir, "logs", "bitcoin", "regtest", "lnd.log");
    }

    public tlsCertPath() {
        return path.join(this.lndDir, "tls.cert");
    }

    public adminMacaroonPath() {
        return path.join(
            this.lndDir,
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

    private async createConfigFile() {
        // We don't use REST but want a random port so we don't get used port errors.
        const restPort = await getPort();
        const output = `[Application Options]
debuglevel=trace

; peer to peer port
listen=127.0.0.1:${this.lndP2pPort}

; gRPC
rpclisten=127.0.0.1:${this.lndRpcPort}

; REST interface
restlisten=127.0.0.1:${restPort}

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
        const config = path.join(this.lndDir, "lnd.conf");
        await writeFileAsync(config, output);
    }

    private logReader() {
        return new LogReader(this.logPath());
    }
}

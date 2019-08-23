import { expect, request, use } from "chai";
import chaiHttp = require("chai-http");
import { TransactionReceipt } from "ethers/providers";
import { Transaction } from "ethers/utils";
import * as fs from "fs";
import * as toml from "toml";
import URI from "urijs";
import { TestConfig } from "./actor";
import { sleep } from "./util";

use(chaiHttp);

export interface IdMatchResponse<M> {
    query: any;
    matches: M[];
}

export interface IdMatch {
    id: string;
}

export interface EthereumMatch {
    transaction: Transaction;
    receipt: TransactionReceipt;
}

export interface MetaBtsieveConfig {
    host: string;
    config_file: string;
    env: { [key: string]: string };
}

export class Btsieve {
    private readonly host: string;
    private readonly port: number;

    constructor(name: string, testConfig: TestConfig, root: string) {
        const metaBtsieveConfig = testConfig.btsieve;
        if (!metaBtsieveConfig) {
            throw new Error("btsieve configuration is needed");
        }

        this.host = metaBtsieveConfig[name].host;
        const btsieveConfig = toml.parse(
            fs.readFileSync(
                `${root}/${metaBtsieveConfig[name].config_file}`,
                "utf8"
            )
        );
        this.port = btsieveConfig.http_api.port_bind;
    }

    public url() {
        return `http://${this.host}:${this.port}`;
    }

    public absoluteLocation(relativeLocation: string) {
        return new URI(relativeLocation)
            .protocol("http")
            .host(this.host)
            .port(this.port.toString())
            .toString();
    }

    public async pollUntilMatches<M>(
        queryUrl: string
    ): Promise<IdMatchResponse<M>> {
        const res = await request(queryUrl).get("");

        expect(res).to.have.status(200);

        if (res.body.matches.length !== 0) {
            return res.body;
        } else {
            await sleep(200);
            return this.pollUntilMatches(queryUrl);
        }
    }
}

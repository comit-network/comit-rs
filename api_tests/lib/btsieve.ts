import { expect, request, use } from "chai";
import chaiHttp = require("chai-http");
import * as fs from "fs";
import * as toml from "toml";
import URI from "urijs";
import { Transaction, TransactionReceipt } from "web3-core";
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
    public host: string;
    public port: number;

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

    public absoluteLocation(relative_location: string) {
        return new URI(relative_location)
            .protocol("http")
            .host(this.host)
            .port(this.port.toString())
            .toString();
    }

    public async pollUntilMatches<M>(
        query_url: string
    ): Promise<IdMatchResponse<M>> {
        const res = await request(query_url).get("");

        expect(res).to.have.status(200);

        if (res.body.matches.length !== 0) {
            return res.body;
        } else {
            await sleep(200);
            return this.pollUntilMatches(query_url);
        }
    }
}

import { use, request, expect } from "chai";
import { Transaction, TransactionReceipt } from "web3-core";
import * as toml from "toml";
import * as fs from "fs";
import { TestConfig } from "./actor";
import chaiHttp = require("chai-http");
import { sleep } from "./util";
import URI from "urijs";

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
    config_dir: string;
    env: { [key: string]: string };
}

export class Btsieve {
    host: string;
    port: number;

    constructor(name: string, testConfig: TestConfig, root: string) {
        const metaBtsieveConfig = testConfig.btsieve;
        if (!metaBtsieveConfig) {
            throw new Error("btsieve configuration is needed");
        }

        this.host = metaBtsieveConfig[name].host;
        let btsieveConfig = toml.parse(
            fs.readFileSync(
                root +
                    "/" +
                    metaBtsieveConfig[name].config_dir +
                    "/default.toml",
                "utf8"
            )
        );
        this.port = btsieveConfig.http_api.port_bind;
    }

    url() {
        return "http://" + this.host + ":" + this.port;
    }

    absoluteLocation(relative_location: string) {
        return new URI(relative_location)
            .protocol("http")
            .host(this.host)
            .port(this.port.toString())
            .toString();
    }

    async pollUntilMatches<M>(query_url: string): Promise<IdMatchResponse<M>> {
        let res = await request(query_url).get("");

        expect(res).to.have.status(200);

        if (res.body.matches.length !== 0) {
            return res.body;
        } else {
            await sleep(200);
            return this.pollUntilMatches(query_url);
        }
    }
}

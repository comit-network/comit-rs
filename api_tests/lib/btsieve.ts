import { parse } from "@iarna/toml";
import { expect, request, use } from "chai";
import chaiHttp = require("chai-http");
import { TransactionReceipt } from "ethers/providers";
import { Transaction } from "ethers/utils";
import * as fs from "fs";
import URI from "urijs";
import { BTSIEVE_BASE_CONFIG } from "./config";
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

export class Btsieve {
    public readonly expectedVersion: string;
    private readonly port: number;

    constructor(root: string) {
        const config = BTSIEVE_BASE_CONFIG;
        this.port = config.http_api.port_bind;

        const cndCargoToml: any = parse(
            fs.readFileSync(`${root}/cnd/Cargo.toml`, "utf8")
        );
        this.expectedVersion = cndCargoToml.package.version;
    }

    public url() {
        return `http://127.0.0.1:${this.port}`;
    }

    public absoluteLocation(relativeLocation: string) {
        return new URI(relativeLocation)
            .protocol("http")
            .host("127.0.0.1")
            .port(this.port.toString())
            .toString();
    }

    public async pollUntilMatches<M>(
        queryUrl: string
    ): Promise<IdMatchResponse<M>> {
        const res = await request(queryUrl)
            .get("")
            .set("Expected-Version", this.expectedVersion);

        expect(res).to.have.status(200);

        if (res.body.matches.length !== 0) {
            return res.body;
        } else {
            await sleep(200);
            return this.pollUntilMatches(queryUrl);
        }
    }
}

import * as chai from "chai";

import chaiHttp = require("chai-http");
chai.use(chaiHttp);

import { Transaction } from "web3-core";
import { TransactionReceipt } from "web3-core";

export interface IdMatchResponse {
    query: any;
    matches: { id: string }[];
}

export interface EthereumTransactionResponse {
    query: any;
    matches: EthereumMatch[];
}

export interface EthereumMatch {
    transaction: Transaction;
    receipt: TransactionReceipt;
}

export interface BtsieveConfig {
    env: { [key: string]: string };
}

export class Btsieve {
    host: string;
    port: number;

    constructor(host: string, port: number) {
        this.host = host;
        this.port = port;
    }

    url() {
        return "http://" + this.host + ":" + this.port;
    }

    absoluteLocation(relative_location: string) {
        return this.url() + relative_location;
    }

    pollUntilMatches(query_url: string) {
        return new Promise((final_res, rej) => {
            chai.request(query_url)
                .get("")
                .end((err, res) => {
                    if (err) {
                        return rej(err);
                    }
                    res.should.have.status(200);
                    if (res.body.matches.length !== 0) {
                        final_res(res.body);
                    } else {
                        setTimeout(async () => {
                            const result = await this.pollUntilMatches(
                                query_url
                            );
                            final_res(result);
                        }, 200);
                    }
                });
        });
    }
}

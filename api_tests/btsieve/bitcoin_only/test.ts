import * as bitcoin from "../../lib/bitcoin";
import { Wallet } from "../../lib/wallet";
import * as chai from "chai";
import chaiHttp = require("chai-http");
import * as ethereum from "../../lib/ethereum";
import { HarnessGlobal, sleep } from "../../lib/util";
import {
    IdMatchResponse,
    EthereumTransactionResponse,
    Btsieve,
} from "../../lib/btsieve";

const should = chai.should();
chai.use(chaiHttp);

declare var global: HarnessGlobal;

const btsieve = new Btsieve("main", global.config, global.project_root);

const tobyWallet = new Wallet("toby", {
    ethConfig: global.ledgers_config.ethereum,
    btcConfig: global.ledgers_config.bitcoin,
});

setTimeout(async function() {
    describe("Test btsieve API - bitcoin", () => {
        let token_contract_address: string;
        before(async function() {
            this.timeout(5000);
            await bitcoin.ensureSegwit();
            await tobyWallet.btc().fund(5);
        });

        describe("Bitcoin", () => {
            describe("Transactions", () => {
                it("btsieve should respond not found when getting a non-existent bitcoin transaction query", async function() {
                    return chai
                        .request(btsieve.url())
                        .get("/queries/bitcoin/regtest/transactions/1")
                        .then(res => {
                            res.should.have.status(404);
                        });
                });

                const to_address =
                    "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap";
                let location: string;

                it("btsieve should respond not found when creating a bitcoin transaction query for an invalid network", async function() {
                    return chai
                        .request(btsieve.url())
                        .post("/queries/bitcoin/banananet/transactions")
                        .send({
                            to_address: to_address,
                        })
                        .then(res => {
                            res.should.have.status(404);
                        });
                });

                it("btsieve should respond with location when creating a valid bitcoin transaction query", async function() {
                    return chai
                        .request(btsieve.url())
                        .post("/queries/bitcoin/regtest/transactions")
                        .send({
                            to_address: to_address,
                        })
                        .then(res => {
                            res.should.have.status(201);
                            location = res.header.location;
                            location.should.not.be.empty;
                        });
                });

                it("btsieve should respond with no match when querying an existing bitcoin transaction query", async function() {
                    return chai
                        .request(btsieve.absoluteLocation(location))
                        .get("")
                        .then(res => {
                            res.should.have.status(200);
                            res.body.query.to_address.should.equal(to_address);
                            res.body.matches.should.be.empty;
                        });
                });

                it("btsieve should respond with transaction match when requesting on the `to_address` bitcoin transaction query", async function() {
                    this.slow(1000);
                    return tobyWallet
                        .btc()
                        .sendToAddress(to_address, 100000000)
                        .then(() => {
                            return bitcoin.generate(1).then(() => {
                                return btsieve
                                    .pollUntilMatches(
                                        btsieve.absoluteLocation(location)
                                    )
                                    .then((body: IdMatchResponse) => {
                                        body.query.to_address.should.equal(
                                            to_address
                                        );
                                        body.matches.should.have.lengthOf(1);
                                        body.matches[0].id.should.be.a(
                                            "string"
                                        );
                                    });
                            });
                        });
                });

                it("btsieve should respond with full transaction details when requesting on the `to_address` bitcoin transaction query with `return_as=transaction`", async function() {
                    return bitcoin.generate(1).then(() => {
                        return chai
                            .request(btsieve.absoluteLocation(location))
                            .get("?return_as=transaction")
                            .then(res => {
                                res.body.query.to_address.should.equal(
                                    to_address
                                );
                                res.body.matches.should.have.lengthOf(1);
                                res.body.matches[0].transaction.output.should.have.lengthOf(
                                    2
                                );
                                res.body.matches[0].transaction.output[0].should.be.a(
                                    "object"
                                );
                            });
                    });
                });

                it("btsieve should respond with no content when deleting an existing bitcoin transaction query", async function() {
                    return chai
                        .request(btsieve.absoluteLocation(location))
                        .del("")
                        .then(res => {
                            res.should.have.status(204);
                        });
                });
            });

            describe("Blocks", () => {
                it("btsieve should respond not found when getting a non-existent bitcoin block query", async function() {
                    return chai
                        .request(btsieve.url())
                        .get("/queries/bitcoin/regtest/blocks/1")
                        .then(res => {
                            res.should.have.status(404);
                        });
                });

                it("btsieve should respond not found when creating a bitcoin block query for an invalid network", async function() {
                    return chai
                        .request(btsieve.url())
                        .post("/queries/bitcoin/banananet/blocks")
                        .send({
                            min_height: min_height,
                        })
                        .then(res => {
                            res.should.have.status(404);
                        });
                });

                const min_height = 600;
                let location: string;
                it("btsieve should respond with location when creating a valid bitcoin block query", async function() {
                    return chai
                        .request(btsieve.url())
                        .post("/queries/bitcoin/regtest/blocks")
                        .send({
                            min_height: min_height,
                        })
                        .then(res => {
                            res.should.have.status(201);
                            location = res.header.location;
                            location.should.not.be.empty;
                        });
                });

                it("btsieve should respond with no match when querying an existing bitcoin block query", async function() {
                    return chai
                        .request(btsieve.absoluteLocation(location))
                        .get("")
                        .then(res => {
                            res.should.have.status(200);
                            res.body.query.min_height.should.equal(min_height);
                            res.body.matches.should.be.empty;
                        });
                });

                it("btsieve should respond with no block match (yet) when requesting on the min_height 600 bitcoin block query", async function() {
                    this.slow(500);
                    return bitcoin.generate(50).then(() => {
                        return chai
                            .request(btsieve.absoluteLocation(location))
                            .get("")
                            .then(res => {
                                res.should.have.status(200);
                                res.body.query.min_height.should.equal(
                                    min_height
                                );
                                res.body.matches.should.be.empty;
                            });
                    });
                });

                it("btsieve should respond with block match when requesting on the min_height 600 bitcoin block query", async function() {
                    this.slow(2000);
                    this.timeout(3000);
                    return bitcoin.generate(200).then(() => {
                        return btsieve
                            .pollUntilMatches(
                                btsieve.absoluteLocation(location)
                            )
                            .then((body: IdMatchResponse) => {
                                body.query.min_height.should.equal(min_height);
                                body.matches.length.should.greaterThan(1);
                            });
                    });
                });

                it("btsieve should respond with no content when deleting an existing bitcoin block query", async function() {
                    return chai
                        .request(btsieve.absoluteLocation(location))
                        .del("")
                        .then(res => {
                            res.should.have.status(204);
                        });
                });
            });
        });
    });

    run();
}, 0);

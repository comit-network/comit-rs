import * as chai from "chai";
import {HarnessGlobal, sleep} from "../lib/util";
import {Btsieve,} from "../lib/btsieve";
import {LedgerRunner} from "../lib/ledgerRunner";
import chaiHttp = require("chai-http");

const should = chai.should();
chai.use(chaiHttp);

declare var global: HarnessGlobal;

const btsieve = new Btsieve("localhost", 8080);

setTimeout(async function () {
    describe("Test btsieve API", () => {

        before(async function () {
            this.timeout(5000);
        });


        describe("Ethereum", () => {
            describe("Transactions", () => {

                const to_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
                let location: string;
                it("btsieve should respond with location when creating a valid ethereum transaction query", async function () {
                    return chai
                        .request(btsieve.url())
                        .post("/queries/ethereum/regtest/transactions")
                        .send({
                            to_address: to_address,
                        })
                        .then(res => {
                            res.should.have.status(201);
                            location = res.header.location;
                            location.should.not.be.empty;
                        });
                });

                it("btsieve should respond with no match when querying an existing ethereum transaction query", async function () {
                    return chai
                        .request(location)
                        .get("")
                        .then(res => {
                            res.should.have.status(200);
                        });
                });

                it("YOU ARE TERMINATED!!!", async function () {
                    this.timeout(60000);
                    console.log("Ledgers are terminated");
                    LedgerRunner.pauseLedger("ethereum");
                    return;
                });

                it("btsieve should respond with error when ethereum is offline", async function () {
                    return chai
                        .request(location)
                        .get("")
                        .then(res => {
                            res.should.have.status(503);
                        });
                });

            });
        });
    });

    run();
}, 0);

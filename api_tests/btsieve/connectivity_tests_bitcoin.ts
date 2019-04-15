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

        describe("Bitcoin", () => {
            describe("Transactions", () => {
                let location: string;

                it("btsieve should respond with location when creating a valid bitcoin transaction query", async function () {
                    return chai
                        .request(btsieve.url())
                        .post("/queries/bitcoin/regtest/transactions")
                        .send({
                            to_address: "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap",
                        })
                        .then(res => {
                            res.should.have.status(201);
                            location = res.header.location;
                            location.should.not.be.empty;
                        });
                });

                it("btsieve should respond with no match when querying an existing bitcoin transaction query", async function () {
                    return chai
                        .request(location)
                        .get("")
                        .then(res => {
                            res.should.have.status(200);
                            res.body.matches.should.be.empty;
                        });
                });

                it("YOU ARE TERMINATED!!!", async function () {
                    this.timeout(60000);
                    console.log("Ledgers are being terminated...");
                    LedgerRunner.pauseLedger("btc");
                    // return sleep(30000)
                    //     .then(() => {
                    //         console.log("Am I dead?");
                    //     });
                });

                it("btsieve should respond with error when bitcoin is offline", async function () {
                    this.timeout(30000);
                    console.log(location);
                    return chai
                        .request(location)
                        .get("")
                        .then(res => {
                            res.should.have.status(503);
                        });
                });

                it("Unpause bitcoin container", async function () {
                    this.timeout(60000);
                    console.log("Ledgers are being unpaused...");
                    LedgerRunner.unpauseLedger("btc");
                    // return sleep(3000)
                    //     .then(() => {
                    //         console.log("Am I dead?");
                    //     });
                });

            });
        });
    });

    run();
}, 0);

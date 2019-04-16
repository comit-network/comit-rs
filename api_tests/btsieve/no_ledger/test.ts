import * as chai from "chai";
import chaiHttp = require("chai-http");
import { HarnessGlobal } from "../../lib/util";
import { Btsieve } from "../../lib/btsieve";

const should = chai.should();
chai.use(chaiHttp);

declare var global: HarnessGlobal;

const btsieve = new Btsieve("main", global.config, global.project_root);

setTimeout(async function() {
    describe("Test btsieve API - no ledger connected", () => {
        let token_contract_address: string;
        before(async function() {
            this.timeout(5000);
        });

        describe("Bitcoin", () => {
            describe("Transactions", () => {
                it("btsieve should respond `SERVICE UNAVAILABLE` as ledger is not connected if queried for transaction", async function() {
                    return chai
                        .request(btsieve.url())
                        .get("/queries/bitcoin/regtest/transactions/1")
                        .then(res => {
                            res.should.have.status(503);
                        });
                });

                const to_address =
                    "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap";

                it("btsieve should respond `SERVICE UNAVAILABLE` as ledger is not connected if posted new query", async function() {
                    return chai
                        .request(btsieve.url())
                        .post("/queries/bitcoin/regtest/transactions")
                        .send({
                            to_address: to_address,
                        })
                        .then(res => {
                            res.should.have.status(503);
                        });
                });
            });
        });

        describe("Ethereum", () => {
            describe("Transactions", () => {
                it("btsieve should respond `SERVICE UNAVAILABLE` as ledger is not connected if queried for transaction", async function() {
                    return chai
                        .request(btsieve.url())
                        .get("/queries/ethereum/regtest/transactions/1")
                        .then(res => {
                            res.should.have.status(503);
                        });
                });

                const to_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";

                it("btsieve should respond `SERVICE UNAVAILABLE` as ledger is not connected if posted new query", async function() {
                    return chai
                        .request(btsieve.url())
                        .post("/queries/ethereum/regtest/transactions")
                        .send({
                            to_address: to_address,
                        })
                        .then(res => {
                            res.should.have.status(503);
                        });
                });
            });
        });
    });

    run();
}, 0);

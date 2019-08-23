import { expect, request } from "chai";
import { Btsieve } from "../../lib/btsieve";
import "../../lib/setup_chai";
import { HarnessGlobal } from "../../lib/util";

declare var global: HarnessGlobal;

const btsieve = new Btsieve("main", global.config, global.project_root);

setTimeout(async function() {
    describe("Test btsieve API - no ledger connected", () => {
        describe("BTsieve", () => {
            describe("Ping", () => {
                it("btsieve ping should respond with 200", async function() {
                    const res = await request(btsieve.url())
                        .get("/health")
                        .set("Expected-Version", btsieve.expectedVersion);

                    expect(res).to.have.status(200);
                });
            });
        });

        describe("Bitcoin", () => {
            describe("Transactions", () => {
                it("btsieve should respond `SERVICE UNAVAILABLE` as ledger is not connected if queried for transaction", async function() {
                    const res = await request(btsieve.url())
                        .get("/queries/bitcoin/regtest/transactions/1")
                        .set("Expected-Version", btsieve.expectedVersion);

                    expect(res).to.have.status(503);
                });

                const toAddress =
                    "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap";

                it("btsieve should respond `SERVICE UNAVAILABLE` as ledger is not connected if posted new query", async function() {
                    const res = await request(btsieve.url())
                        .post("/queries/bitcoin/regtest/transactions")
                        .set("Expected-Version", btsieve.expectedVersion)
                        .send({
                            to_address: toAddress,
                        });

                    expect(res).to.have.status(503);
                });
            });
        });

        describe("Ethereum", () => {
            describe("Transactions", () => {
                it("btsieve should respond `SERVICE UNAVAILABLE` as ledger is not connected if queried for transaction", async function() {
                    const res = await request(btsieve.url())
                        .post("/queries/ethereum/regtest/transactions/1")
                        .set("Expected-Version", btsieve.expectedVersion)
                        .send({
                            to_address: toAddress,
                        });

                    expect(res).to.have.status(503);
                });

                const toAddress = "0x00a329c0648769a73afac7f9381e08fb43dbea72";

                it("btsieve should respond `SERVICE UNAVAILABLE` as ledger is not connected if posted new query", async function() {
                    const res = await request(btsieve.url())
                        .post("/queries/ethereum/regtest/transactions")
                        .set("Expected-Version", btsieve.expectedVersion)
                        .send({
                            to_address: toAddress,
                        });

                    expect(res).to.have.status(503);
                });
            });
        });
    });

    run();
}, 0);

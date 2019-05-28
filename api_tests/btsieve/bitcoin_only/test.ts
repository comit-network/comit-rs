import * as bitcoin from "../../lib/bitcoin";
import { Wallet } from "../../lib/wallet";
import { expect, request } from "chai";
import { HarnessGlobal } from "../../lib/util";
import { Btsieve, IdMatch } from "../../lib/btsieve";
import "../../lib/setupChai";

declare var global: HarnessGlobal;

const btsieve = new Btsieve("main", global.config, global.project_root);

const tobyWallet = new Wallet("toby", {
    ethereumNodeConfig: global.ledgers_config.ethereum,
    bitcoinNodeConfig: global.ledgers_config.bitcoin,
});

setTimeout(async function() {
    describe("Test btsieve API - bitcoin", () => {
        before(async function() {
            this.timeout(5000);
            await bitcoin.ensureFunding();
            await tobyWallet.btc().fund(5);
        });

        describe("Bitcoin", () => {
            describe("Transactions", () => {
                it("btsieve should respond not found when getting a non-existent bitcoin transaction query", async function() {
                    let res = await request(btsieve.url()).get(
                        "/queries/bitcoin/regtest/transactions/1"
                    );

                    expect(res).to.have.status(404);
                });

                const to_address =
                    "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap";
                let location: string;

                it("btsieve should respond not found when creating a bitcoin transaction query for an invalid network", async function() {
                    let res = await request(btsieve.url())
                        .post("/queries/bitcoin/banananet/transactions")
                        .send({
                            to_address: to_address,
                        });

                    expect(res).to.have.status(404);
                });

                it("btsieve should respond with location when creating a valid bitcoin transaction query", async function() {
                    let res = await request(btsieve.url())
                        .post("/queries/bitcoin/regtest/transactions")
                        .send({
                            to_address: to_address,
                        });

                    location = res.header.location;

                    expect(res).to.have.status(201);
                    expect(location).to.not.be.empty;
                });

                it("btsieve should respond with no match when querying an existing bitcoin transaction query", async function() {
                    let res = await request(
                        btsieve.absoluteLocation(location)
                    ).get("");

                    expect(res).to.have.status(200);
                    expect(res.body.query.to_address).to.equal(to_address);
                    expect(res.body.matches).to.be.empty;
                });

                it("btsieve should respond with transaction match when requesting on the `to_address` bitcoin transaction query", async function() {
                    this.slow(1000);
                    await tobyWallet.btc().sendToAddress(to_address, 100000000);

                    await bitcoin.generate(1);

                    let body = await btsieve.pollUntilMatches<IdMatch>(
                        btsieve.absoluteLocation(location)
                    );

                    expect(body.query.to_address).to.equal(to_address);
                    expect(body.matches).to.have.length(1);
                    expect(body.matches)
                        .each.property("id")
                        .to.be.a("string");
                });

                it("btsieve should respond with full transaction details when requesting on the `to_address` bitcoin transaction query with `return_as=transaction`", async function() {
                    await bitcoin.generate(1);

                    let res = await request(
                        btsieve.absoluteLocation(location)
                    ).get("?return_as=transaction");

                    expect(res.body.query.to_address).to.equal(to_address);
                    expect(res.body.matches).to.have.length(1);
                    expect(
                        res.body.matches[0].transaction.output
                    ).to.have.length(2);
                    expect(res.body.matches[0].transaction.output[0]).to.be.a(
                        "object"
                    );
                });

                it("btsieve should respond with no content when deleting an existing bitcoin transaction query", async function() {
                    let res = await request(
                        btsieve.absoluteLocation(location)
                    ).del("");

                    expect(res).to.have.status(204);
                });
            });

            describe("Blocks", () => {
                it("btsieve should respond not found when getting a non-existent bitcoin block query", async function() {
                    let res = await request(btsieve.url()).get(
                        "/queries/bitcoin/regtest/blocks/1"
                    );

                    expect(res).to.have.status(404);
                });

                const min_height = 200;
                let location: string;
                it("btsieve should respond not found when creating a bitcoin block query for an invalid network", async function() {
                    let res = await request(btsieve.url())
                        .post("/queries/bitcoin/banananet/blocks")
                        .send({
                            min_height: min_height,
                        });

                    expect(res).to.have.status(404);
                });

                it("btsieve should respond with location when creating a valid bitcoin block query", async function() {
                    let res = await request(btsieve.url())
                        .post("/queries/bitcoin/regtest/blocks")
                        .send({
                            min_height: min_height,
                        });

                    location = res.header.location;

                    expect(res).to.have.status(201);
                    expect(location).to.not.be.empty;
                });

                it("btsieve should respond with no match when querying an existing bitcoin block query", async function() {
                    let res = await request(
                        btsieve.absoluteLocation(location)
                    ).get("");

                    expect(res).to.have.status(200);
                    expect(res.body.query.min_height).to.equal(min_height);
                    expect(res.body.matches).to.be.empty;
                });

                it("btsieve should respond with no block match (yet) when requesting on the min_height 600 bitcoin block query", async function() {
                    this.slow(500);
                    await bitcoin.generate(50);

                    let res = await request(
                        btsieve.absoluteLocation(location)
                    ).get("");

                    expect(res).to.have.status(200);
                    expect(res.body.query.min_height).to.equal(min_height);
                    expect(res.body.matches).to.be.empty;
                });

                it("btsieve should respond with block match when requesting on the min_height 600 bitcoin block query", async function() {
                    this.slow(2000);
                    this.timeout(3000);

                    await bitcoin.generate(50);
                    let body = await btsieve.pollUntilMatches<IdMatch>(
                        btsieve.absoluteLocation(location)
                    );

                    expect(body.query.min_height).to.equal(min_height);
                    expect(body.matches).to.have.length.greaterThan(1);
                });

                it("btsieve should respond with no content when deleting an existing bitcoin block query", async function() {
                    let res = await request(
                        btsieve.absoluteLocation(location)
                    ).del("");

                    expect(res).to.have.status(204);
                });
            });
        });
    });

    run();
}, 0);

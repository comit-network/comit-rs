import { expect, request } from "chai";
import * as bitcoin from "../../lib/bitcoin";
import { Btsieve, IdMatch } from "../../lib/btsieve";
import "../../lib/setup_chai";
import { HarnessGlobal } from "../../lib/util";
import { Wallet } from "../../lib/wallet";

declare var global: HarnessGlobal;

const btsieve = new Btsieve(global.projectRoot);

const tobyWallet = new Wallet("toby", {
    ledgerConfig: global.ledgerConfigs,
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
                    const res = await request(btsieve.url())
                        .get("/queries/bitcoin/regtest/transactions/1")
                        .set("Expected-Version", btsieve.expectedVersion);

                    expect(res).to.have.status(404);
                });

                const toAddress =
                    "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap";
                let location: string;

                it("btsieve should respond not found when creating a bitcoin transaction query for an invalid network", async function() {
                    const res = await request(btsieve.url())
                        .post("/queries/bitcoin/banananet/transactions")
                        .set("Expected-Version", btsieve.expectedVersion)
                        .send({
                            to_address: toAddress,
                        });

                    expect(res).to.have.status(404);
                });

                it("btsieve should respond with location when creating a valid bitcoin transaction query", async function() {
                    const res = await request(btsieve.url())
                        .post("/queries/bitcoin/regtest/transactions")
                        .set("Expected-Version", btsieve.expectedVersion)
                        .send({
                            to_address: toAddress,
                        });

                    location = res.header.location;

                    expect(res).to.have.status(201);
                    expect(location).to.not.be.empty;
                });

                it("btsieve should respond with no match when querying an existing bitcoin transaction query", async function() {
                    const res = await request(
                        btsieve.absoluteLocation(location)
                    )
                        .get("")
                        .set("Expected-Version", btsieve.expectedVersion);

                    expect(res).to.have.status(200);
                    expect(res.body.query.to_address).to.equal(toAddress);
                    expect(res.body.matches).to.be.empty;
                });

                it("btsieve should respond with transaction match when requesting on the `toAddress` bitcoin transaction query", async function() {
                    this.slow(1000);
                    await tobyWallet.btc().sendToAddress(toAddress, 100000000);

                    await bitcoin.generate(1);

                    const body = await btsieve.pollUntilMatches<IdMatch>(
                        btsieve.absoluteLocation(location)
                    );

                    expect(body.query.to_address).to.equal(toAddress);
                    expect(body.matches).to.have.length(1);
                    expect(body.matches)
                        .each.property("id")
                        .to.be.a("string");
                });

                it("btsieve should respond with full transaction details when requesting on the `toAddress` bitcoin transaction query with `return_as=transaction`", async function() {
                    await bitcoin.generate(1);

                    const res = await request(
                        btsieve.absoluteLocation(location)
                    )
                        .get("?return_as=transaction")
                        .set("Expected-Version", btsieve.expectedVersion);

                    expect(res.body.query.to_address).to.equal(toAddress);
                    expect(res.body.matches).to.have.length(1);
                    expect(
                        res.body.matches[0].transaction.output
                    ).to.have.length(2);
                    expect(res.body.matches[0].transaction.output[0]).to.be.a(
                        "object"
                    );
                });

                it('btsieve should respond with 200 OK when "getting-or-creating" a query that already exists', async function() {
                    const res = await request(btsieve.url())
                        .put(location)
                        .set("Expected-Version", btsieve.expectedVersion)
                        .send({
                            to_address: toAddress,
                        });

                    expect(res).to.have.status(200);
                });

                it('btsieve should respond with 409 CONFLICT when "getting-or-creating" an existing query ID with a different query body', async function() {
                    const differentToAddress =
                        "mzkdMKoki1hoP3ogT2oGSJ4pBTC9UGDLfM";

                    const res = await request(btsieve.url())
                        .put(location)
                        .set("Expected-Version", btsieve.expectedVersion)
                        .send({
                            to_address: differentToAddress,
                        });

                    expect(res).to.have.status(409);
                });

                it('btsieve should respond with 204 NO_CONTENT when "getting-or-creating" a new query', async function() {
                    const newQueryId =
                        "4BvFBixM4HmhV8AJe5RC8v8csxxhDBscMxwpiK5e";
                    const newLocation =
                        "/queries/bitcoin/regtest/transactions/" + newQueryId;
                    const newQuery = {
                        to_address: "mndvsV4weBdPFTarHQbg71rCXRy8z79SH5",
                    };

                    const res = await request(btsieve.url())
                        .put(newLocation)
                        .set("Expected-Version", btsieve.expectedVersion)
                        .send(newQuery);

                    expect(res).to.have.status(204);
                });

                it("btsieve should respond with no content when deleting an existing bitcoin transaction query", async function() {
                    const res = await request(
                        btsieve.absoluteLocation(location)
                    )
                        .del("")
                        .set("Expected-Version", btsieve.expectedVersion);

                    expect(res).to.have.status(204);
                });
            });
        });
    });

    run();
}, 0);

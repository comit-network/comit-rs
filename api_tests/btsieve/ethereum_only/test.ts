import { Wallet } from "../../lib/wallet";
import { expect, request } from "chai";
import { HarnessGlobal, sleep } from "../../lib/util";
import { Btsieve, EthereumMatch, IdMatch } from "../../lib/btsieve";
import "../../lib/setupChai";

declare var global: HarnessGlobal;

const btsieve = new Btsieve("main", global.config, global.project_root);

const tobyWallet = new Wallet("toby", {
    ethereumNodeConfig: global.ledgers_config.ethereum,
    bitcoinNodeConfig: global.ledgers_config.bitcoin,
});

const aliceWallet = new Wallet("alice", {
    ethereumNodeConfig: global.ledgers_config.ethereum,
});

setTimeout(async function() {
    describe("Test btsieve API - ethereum", () => {
        let tokenContractAddress: string;
        before(async function() {
            this.timeout(10000);
            await tobyWallet.eth().fund("20");
            await aliceWallet.eth().fund("1");

            tokenContractAddress = await tobyWallet
                .eth()
                .deployErc20TokenContract(global.project_root);
            await tobyWallet
                .eth()
                .mintErc20To(
                    aliceWallet.eth().address(),
                    10,
                    tokenContractAddress
                );
        });

        describe("Ethereum", () => {
            describe("Transactions", () => {
                before(async () => {
                    await tobyWallet.eth().fund("10");
                });

                it("btsieve should respond not found when getting a non-existent ethereum transaction query", async function() {
                    let res = await request(btsieve.url()).get(
                        "/queries/ethereum/regtest/transactions/1"
                    );

                    expect(res).to.have.status(404);
                });

                it("btsieve should respond not found when creating an ethereum transaction query for an invalid network", async function() {
                    let res = await request(btsieve.url())
                        .post("/queries/ethereum/banananet/transactions")
                        .send({
                            to_address: to_address,
                        });

                    expect(res).to.have.status(404);
                });

                const to_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
                let location: string;
                it("btsieve should respond with location when creating a valid ethereum transaction query", async function() {
                    let res = await request(btsieve.url())
                        .post("/queries/ethereum/regtest/transactions")
                        .send({
                            to_address: to_address,
                        });

                    location = res.header.location;

                    expect(res).to.have.status(201);
                    expect(location).to.not.be.empty;
                });

                it("btsieve should respond with no match when querying an existing ethereum transaction query", async function() {
                    let res = await request(
                        btsieve.absoluteLocation(location)
                    ).get("");

                    expect(res).to.have.status(200);
                    expect(res.body.query.to_address).to.equal(to_address);
                    expect(res.body.matches).to.be.empty;
                });

                it("btsieve should respond with no transaction match (yet) when requesting on the `to_address` ethereum block query", async function() {
                    await tobyWallet
                        .eth()
                        .sendEthTransactionTo(
                            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                            "",
                            1
                        );

                    let res = await request(
                        btsieve.absoluteLocation(location)
                    ).get("");

                    expect(res).to.have.status(200);
                    expect(res.body.query.to_address).to.equal(to_address);
                    expect(res.body.matches).to.be.empty;
                });

                it("btsieve should respond with transaction match when requesting on the `to_address` ethereum transaction query", async function() {
                    this.slow(2000);
                    await tobyWallet
                        .eth()
                        .sendEthTransactionTo(to_address, "", 5);

                    let body = await btsieve.pollUntilMatches<EthereumMatch>(
                        btsieve.absoluteLocation(location)
                    );

                    expect(body.query.to_address).to.equal(to_address);
                    expect(body.matches).to.have.length(1);
                });

                it("btsieve should respond with no content when deleting an existing ethereum transaction query", async function() {
                    let res = await request(btsieve.url()).del(
                        "/queries/ethereum/regtest/transactions/1"
                    );

                    expect(res).to.have.status(204);
                });
            });

            describe("Blocks", () => {
                it("btsieve should respond not found when getting a non-existent ethereum block query", async function() {
                    let res = await request(btsieve.url()).get(
                        "/queries/ethereum/regtest/blocks/1"
                    );

                    expect(res).to.have.status(404);
                });

                it("btsieve should respond not found when creating an ethereum block query for an invalid network", async function() {
                    let res = await request(btsieve.url())
                        .post("/queries/ethereum/banananet/blocks")
                        .send({
                            min_timestamp_secs: min_timestamp_secs,
                        });

                    expect(res).to.have.status(404);
                });

                let location: string;
                const epoch_seconds_now = Math.round(Date.now() / 1000);
                const min_timestamp_secs = epoch_seconds_now + 3;
                it("btsieve should respond with location when creating a valid ethereum block query", async function() {
                    this.timeout(1000);
                    let res = await request(btsieve.url())
                        .post("/queries/ethereum/regtest/blocks")
                        .send({
                            min_timestamp_secs: min_timestamp_secs,
                        });

                    location = res.header.location;

                    expect(res).to.have.status(201);
                    expect(location).to.not.be.empty;
                });

                it("btsieve should respond with no match when querying an existing ethereum block query", async function() {
                    this.timeout(1000);
                    let res = await request(
                        btsieve.absoluteLocation(location)
                    ).get("");

                    expect(res).to.have.status(200);
                    expect(res.body.query.min_timestamp_secs).to.equal(
                        min_timestamp_secs
                    );
                    expect(res.body.matches).to.be.empty;
                });

                it("btsieve should respond with block match when requesting on the timestamp ethereum block query after waiting 3 seconds", async function() {
                    this.timeout(80000);
                    this.slow(6000);

                    await sleep(3000);
                    await tobyWallet
                        .eth()
                        .sendEthTransactionTo(
                            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                            "",
                            1
                        );

                    let body = await btsieve.pollUntilMatches<EthereumMatch>(
                        btsieve.absoluteLocation(location)
                    );

                    expect(body.query.min_timestamp_secs).to.equal(
                        min_timestamp_secs
                    );
                    expect(body.matches).to.have.length(1);
                });

                it("btsieve should respond with no content when deleting an existing ethereum block query", async function() {
                    let res = await request(
                        btsieve.absoluteLocation(location)
                    ).del("");

                    expect(res).to.have.status(204);
                });
            });

            describe("Transaction Receipts", () => {
                it("btsieve should respond not found when getting a non-existent ethereum transaction receipt query", async function() {
                    let res = await request(btsieve.url()).get(
                        "/queries/ethereum/regtest/logs/1"
                    );

                    expect(res).to.have.status(404);
                });

                // keccak('Transfer(address,address,uint256)')
                const transfer_topic =
                    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
                const from_address =
                    "0x000000000000000000000000" +
                    aliceWallet
                        .eth()
                        .address()
                        .replace("0x", "");
                const to_address =
                    "0x00000000000000000000000005cbb3fdb5060e04e33ea89c6029d7c79199b4cd";

                let location: string;
                it("btsieve should respond with location when creating a valid transaction receipt query", async function() {
                    this.timeout(1000);
                    let res = await request(btsieve.url())
                        .post("/queries/ethereum/regtest/logs")
                        .send({
                            event_matchers: [
                                {
                                    address: tokenContractAddress,
                                    data:
                                        "0x0000000000000000000000000000000000000000000000000000000000000001",
                                    topics: [
                                        transfer_topic,
                                        from_address,
                                        to_address,
                                    ],
                                },
                            ],
                        });

                    location = res.header.location;

                    expect(res).to.have.status(201);
                    expect(location).to.not.be.empty;
                });

                it("btsieve should respond with no match when querying an existing ethereum transaction receipt query", async function() {
                    this.timeout(1000);
                    let res = await request(
                        btsieve.absoluteLocation(location)
                    ).get("");

                    expect(res).to.have.status(200);
                    expect(res.body.matches).to.be.empty;
                });

                it("btsieve should respond with transaction receipt match when requesting on the transfer_topic query after waiting 3 seconds", async function() {
                    this.slow(2000);
                    this.timeout(20000);
                    const transfer_token_data =
                        "0xa9059cbb" +
                        to_address.replace("0x", "") +
                        "0000000000000000000000000000000000000000000000000000000000000001";

                    let receipt = await aliceWallet
                        .eth()
                        .sendEthTransactionTo(
                            tokenContractAddress,
                            transfer_token_data,
                            0
                        );

                    let body = await btsieve.pollUntilMatches<IdMatch>(
                        btsieve.absoluteLocation(location)
                    );

                    expect(body.matches).to.have.length(1);
                    expect(body.matches[0].id).to.equal(
                        receipt.transactionHash
                    );
                    expect(body.matches[0].id).to.match(/^0x/);
                });

                it("btsieve should return transaction and receipt if `return_as` is given", async function() {
                    let body = await btsieve.pollUntilMatches<EthereumMatch>(
                        btsieve.absoluteLocation(location) +
                            "?return_as=transaction_and_receipt"
                    );

                    expect(body.matches).to.have.length(1);
                    expect(body.matches[0].transaction).to.be.a("object");
                    expect(body.matches[0].receipt).to.be.a("object");
                });

                it("btsieve should respond with no content when deleting an existing ethereum transaction receipt query", async function() {
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

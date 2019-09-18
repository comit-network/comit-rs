import { expect, request } from "chai";
import { Btsieve, EthereumMatch, IdMatch } from "../../lib/btsieve";
import "../../lib/setup_chai";
import { HarnessGlobal } from "../../lib/util";
import { Wallet } from "../../lib/wallet";

declare var global: HarnessGlobal;

const btsieve = new Btsieve(global.projectRoot);

const tobyWallet = new Wallet("toby", {
    ledgerConfig: global.ledgerConfigs,
});
const aliceWallet = new Wallet("alice", {
    ledgerConfig: global.ledgerConfigs,
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
                .deployErc20TokenContract(global.projectRoot);
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
                    const res = await request(btsieve.url())
                        .get("/queries/ethereum/regtest/transactions/1")
                        .set("Expected-Version", btsieve.expectedVersion);

                    expect(res).to.have.status(404);
                });

                const toAddress = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
                const queryId = "4BvFBixM4HmhV8AJe5RC8v8csxxhDBsc420940949";

                const location =
                    "/queries/ethereum/regtest/transactions/" + queryId;
                const query = {
                    to_address: toAddress,
                };

                it("btsieve should respond not found when creating an ethereum transaction query for an invalid network", async function() {
                    const res = await request(btsieve.url())
                        .put(
                            "/queries/ethereum/banananet/transactions/toheusthoanu"
                        )
                        .set("Expected-Version", btsieve.expectedVersion)
                        .send(query);

                    expect(res).to.have.status(404);
                });

                it("btsieve should respond with NO_CONTENT when creating a valid ethereum transaction query", async function() {
                    const res = await request(btsieve.url())
                        .put(location)
                        .set("Expected-Version", btsieve.expectedVersion)
                        .send(query);

                    expect(res).to.have.status(204);
                });

                it("btsieve should respond with OK when querying an existing ethereum transaction query", async function() {
                    const res = await request(
                        btsieve.absoluteLocation(location)
                    )
                        .get("")
                        .set("Expected-Version", btsieve.expectedVersion);

                    expect(res).to.have.status(200);
                });

                it("btsieve should respond with no transaction match (yet) when requesting on the `toAddress` ethereum block query", async function() {
                    await tobyWallet
                        .eth()
                        .sendEthTransactionTo(
                            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                            "",
                            1
                        );

                    const res = await request(
                        btsieve.absoluteLocation(location)
                    )
                        .get("")
                        .set("Expected-Version", btsieve.expectedVersion);

                    expect(res).to.have.status(200);
                    expect(res.body.query.to_address).to.equal(toAddress);
                    expect(res.body.matches).to.be.empty;
                });

                it("btsieve should respond with transaction match when requesting on the `toAddress` ethereum transaction query", async function() {
                    this.slow(2000);
                    await tobyWallet
                        .eth()
                        .sendEthTransactionTo(toAddress, "", 5);

                    const body = await btsieve.pollUntilMatches<EthereumMatch>(
                        btsieve.absoluteLocation(location)
                    );

                    expect(body.query.to_address).to.equal(toAddress);
                    expect(body.matches).to.have.length(1);
                });

                it("btsieve should respond with no content when deleting an existing ethereum transaction query", async function() {
                    const res = await request(btsieve.url())
                        .del("/queries/ethereum/regtest/transactions/1")
                        .set("Expected-Version", btsieve.expectedVersion);

                    expect(res).to.have.status(204);
                });
            });

            describe("Transaction Receipts", () => {
                it("btsieve should respond not found when getting a non-existent ethereum transaction receipt query", async function() {
                    const res = await request(btsieve.url())
                        .get("/queries/ethereum/regtest/logs/1")
                        .set("Expected-Version", btsieve.expectedVersion);

                    expect(res).to.have.status(404);
                });

                // keccak('Transfer(address,address,uint256)')
                const transferTopic =
                    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
                const fromAddress =
                    "0x000000000000000000000000" +
                    aliceWallet
                        .eth()
                        .address()
                        .replace("0x", "");
                const toAddress =
                    "0x00000000000000000000000005cbb3fdb5060e04e33ea89c6029d7c79199b4cd";

                const queryId = "vounthdoeM4HmhV8AJe5620642062406420";
                const location = "/queries/ethereum/regtest/logs/" + queryId;

                it("btsieve should respond with NO_CONTENT when creating a valid transaction receipt query", async function() {
                    this.timeout(1000);
                    const res = await request(btsieve.url())
                        .put(location)
                        .set("Expected-Version", btsieve.expectedVersion)
                        .send({
                            event_matchers: [
                                {
                                    address: tokenContractAddress,
                                    data:
                                        "0x0000000000000000000000000000000000000000000000000000000000000001",
                                    topics: [
                                        transferTopic,
                                        fromAddress,
                                        toAddress,
                                    ],
                                },
                            ],
                        });

                    expect(res).to.have.status(204);
                });

                it("btsieve should respond with no match when querying an existing ethereum transaction receipt query", async function() {
                    this.timeout(1000);
                    const res = await request(
                        btsieve.absoluteLocation(location)
                    )
                        .get("")
                        .set("Expected-Version", btsieve.expectedVersion);

                    expect(res).to.have.status(200);
                    expect(res.body.matches).to.be.empty;
                });

                it("btsieve should respond with transaction receipt match when requesting on the transfer_topic query after waiting 3 seconds", async function() {
                    this.slow(2000);
                    this.timeout(20000);
                    const transferTokenData =
                        "0xa9059cbb" +
                        toAddress.replace("0x", "") +
                        "0000000000000000000000000000000000000000000000000000000000000001";

                    const receipt = await aliceWallet
                        .eth()
                        .sendEthTransactionTo(
                            tokenContractAddress,
                            transferTokenData,
                            0
                        );

                    const body = await btsieve.pollUntilMatches<IdMatch>(
                        btsieve.absoluteLocation(location)
                    );

                    expect(body.matches).to.have.length(1);
                    expect(body.matches[0].id).to.equal(
                        receipt.transactionHash
                    );
                    expect(body.matches[0].id).to.match(/^0x/);
                });

                it("btsieve should return transaction and receipt if `return_as` is given", async function() {
                    const body = await btsieve.pollUntilMatches<EthereumMatch>(
                        btsieve.absoluteLocation(location) +
                            "?return_as=transaction_and_receipt"
                    );

                    expect(body.matches).to.have.length(1);
                    expect(body.matches[0].transaction).to.be.a("object");
                    expect(body.matches[0].receipt).to.be.a("object");
                });

                it("btsieve should respond with no content when deleting an existing ethereum transaction receipt query", async function() {
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

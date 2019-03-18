import * as bitcoin from "../lib/bitcoin";
import { Wallet } from "../lib/wallet";
import * as chai from "chai";
import chaiHttp = require("chai-http");
import * as ethereum from "../lib/ethereum";
import { HarnessGlobal, sleep } from "../lib/util";
import {
    IdMatchResponse,
    EthereumTransactionResponse,
    Btsieve,
} from "../lib/btsieve";

const should = chai.should();
chai.use(chaiHttp);

declare var global: HarnessGlobal;

const btsieve = new Btsieve("localhost", 8080);

const tobyWallet = new Wallet("toby", {
    ethConfig: global.ledgers_config.ethereum,
    btcConfig: global.ledgers_config.bitcoin,
});

const aliceWallet = new Wallet("alice", {
    ethConfig: global.ledgers_config.ethereum,
});

const alice_wallet_address = aliceWallet.eth().address();

describe("Test btsieve API", () => {
    let token_contract_address: string;
    before(async function() {
        this.timeout(5000);
        await bitcoin.ensureSegwit();
        await tobyWallet.btc().fund(5);
        await tobyWallet.eth().fund("20");
        await aliceWallet.eth().fund("1");

        let receipt = await tobyWallet
            .eth()
            .deployErc20TokeContract(global.project_root);
        token_contract_address = receipt.contractAddress;

        await ethereum.mintErc20Tokens(
            tobyWallet.eth(),
            token_contract_address,
            alice_wallet_address,
            10
        );
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

            const to_address = "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap";
            let location: string;
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
                    .request(location)
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
                                .pollUntilMatches(location)
                                .then((body: IdMatchResponse) => {
                                    body.query.to_address.should.equal(
                                        to_address
                                    );
                                    body.matches.should.have.lengthOf(1);
                                    body.matches[0].id.should.be.a("string");
                                });
                        });
                    });
            });

            it("btsieve should respond with full transaction details when requesting on the `to_address` bitcoin transaction query with `return_as=transaction`", async function() {
                return bitcoin.generate(1).then(() => {
                    return chai
                        .request(location)
                        .get("?return_as=transaction")
                        .then(res => {
                            res.body.query.to_address.should.equal(to_address);
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
                    .request(location)
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
                    .request(location)
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
                        .request(location)
                        .get("")
                        .then(res => {
                            res.should.have.status(200);
                            res.body.query.min_height.should.equal(min_height);
                            res.body.matches.should.be.empty;
                        });
                });
            });

            it("btsieve should respond with block match when requesting on the min_height 600 bitcoin block query", async function() {
                this.slow(2000);
                this.timeout(3000);
                return bitcoin.generate(200).then(() => {
                    return btsieve
                        .pollUntilMatches(location)
                        .then((body: IdMatchResponse) => {
                            body.query.min_height.should.equal(min_height);
                            body.matches.length.should.greaterThan(1);
                        });
                });
            });

            it("btsieve should respond with no content when deleting an existing bitcoin block query", async function() {
                return chai
                    .request(location)
                    .del("")
                    .then(res => {
                        res.should.have.status(204);
                    });
            });
        });
    });

    describe("Ethereum", () => {
        describe("Transactions", () => {
            before(async () => {
                await tobyWallet.eth().fund("10");
            });

            it("btsieve should respond not found when getting a non-existent ethereum transaction query", async function() {
                return chai
                    .request(btsieve.url())
                    .get("/queries/ethereum/regtest/transactions/1")
                    .then(res => {
                        res.should.have.status(404);
                    });
            });

            it("btsieve should respond not found when creating an ethereum transaction query for an invalid network", async function() {
                return chai
                    .request(btsieve.url())
                    .post("/queries/ethereum/banananet/transactions")
                    .send({
                        to_address: to_address,
                    })
                    .then(res => {
                        res.should.have.status(404);
                    });
            });

            const to_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
            let location: string;
            it("btsieve should respond with location when creating a valid ethereum transaction query", async function() {
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

            it("btsieve should respond with no match when querying an existing ethereum transaction query", async function() {
                return chai
                    .request(location)
                    .get("")
                    .then(res => {
                        res.should.have.status(200);
                        res.body.query.to_address.should.equal(to_address);
                        res.body.matches.should.be.empty;
                    });
            });

            it("btsieve should respond with no transaction match (yet) when requesting on the `to_address` ethereum block query", async function() {
                return tobyWallet
                    .eth()
                    .sendEthTransactionTo(
                        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                        "",
                        1
                    )
                    .then(() => {
                        return chai
                            .request(location)
                            .get("")
                            .then(res => {
                                res.should.have.status(200);
                                res.body.query.to_address.should.equal(
                                    to_address
                                );
                                res.body.matches.should.be.empty;
                            });
                    });
            });

            it("btsieve should respond with transaction match when requesting on the `to_address` ethereum transaction query", async function() {
                this.slow(2000);
                return tobyWallet
                    .eth()
                    .sendEthTransactionTo(to_address, "", 5)
                    .then(() => {
                        return btsieve
                            .pollUntilMatches(location)
                            .then((body: EthereumTransactionResponse) => {
                                body.query.to_address.should.equal(to_address);
                                body.matches.should.lengthOf(1);
                            });
                    });
            });

            it("btsieve should respond with no content when deleting an existing ethereum transaction query", async function() {
                return chai
                    .request(btsieve.url())
                    .del("/queries/ethereum/regtest/transactions/1")
                    .then(res => {
                        res.should.have.status(204);
                    });
            });
        });

        describe("Blocks", () => {
            it("btsieve should respond not found when getting a non-existent ethereum block query", async function() {
                return chai
                    .request(btsieve.url())
                    .get("/queries/ethereum/regtest/blocks/1")
                    .then(res => {
                        res.should.have.status(404);
                    });
            });

            it("btsieve should respond not found when creating an ethereum block query for an invalid network", async function() {
                return chai
                    .request(btsieve.url())
                    .post("/queries/ethereum/banananet/blocks")
                    .send({
                        min_timestamp_secs: min_timestamp_secs,
                    })
                    .then(res => {
                        res.should.have.status(404);
                    });
            });

            let location: string;
            const epoch_seconds_now = Math.round(Date.now() / 1000);
            const min_timestamp_secs = epoch_seconds_now + 3;
            it("btsieve should respond with location when creating a valid ethereum block query", async function() {
                this.timeout(1000);
                return chai
                    .request(btsieve.url())
                    .post("/queries/ethereum/regtest/blocks")
                    .send({
                        min_timestamp_secs: min_timestamp_secs,
                    })
                    .then(res => {
                        res.should.have.status(201);
                        location = res.header.location;
                        location.should.not.be.empty;
                    });
            });

            it("btsieve should respond with no match when querying an existing ethereum block query", async function() {
                this.timeout(1000);
                return chai
                    .request(location)
                    .get("")
                    .then(res => {
                        res.should.have.status(200);
                        res.body.query.min_timestamp_secs.should.equal(
                            min_timestamp_secs
                        );
                        res.body.matches.should.be.empty;
                    });
            });

            it("btsieve should respond with block match when requesting on the timestamp ethereum block query after waiting 3 seconds", async function() {
                this.timeout(80000);
                this.slow(6000);
                return sleep(3000)
                    .then(() => {
                        return tobyWallet
                            .eth()
                            .sendEthTransactionTo(
                                "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                                "",
                                1
                            );
                    })
                    .then(() => {
                        return btsieve
                            .pollUntilMatches(location)
                            .then((body: EthereumTransactionResponse) => {
                                body.query.min_timestamp_secs.should.equal(
                                    min_timestamp_secs
                                );
                                body.matches.should.lengthOf(1);
                            });
                    });
            });

            it("btsieve should respond with no content when deleting an existing ethereum block query", async function() {
                return chai
                    .request(location)
                    .del("")
                    .then(res => {
                        res.should.have.status(204);
                    });
            });
        });

        describe("Transaction Receipts", () => {
            it("btsieve should respond not found when getting a non-existent ethereum transaction receipt query", async function() {
                return chai
                    .request(btsieve.url())
                    .get("/queries/ethereum/regtest/logs/1")
                    .then(res => {
                        res.should.have.status(404);
                    });
            });

            // keccak('Transfer(address,address,uint256)')
            const transfer_topic =
                "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
            const from_address =
                "0x000000000000000000000000" +
                alice_wallet_address.replace("0x", "");
            const to_address =
                "0x00000000000000000000000005cbb3fdb5060e04e33ea89c6029d7c79199b4cd";

            let location: string;
            it("btsieve should respond with location when creating a valid transaction receipt query", async function() {
                this.timeout(1000);
                return chai
                    .request(btsieve.url())
                    .post("/queries/ethereum/regtest/logs")
                    .send({
                        event_matchers: [
                            {
                                address: token_contract_address,
                                data:
                                    "0x0000000000000000000000000000000000000000000000000000000000000001",
                                topics: [
                                    transfer_topic,
                                    from_address,
                                    to_address,
                                ],
                            },
                        ],
                    })
                    .then(res => {
                        res.should.have.status(201);
                        location = res.header.location;
                        location.should.not.be.empty;
                    });
            });

            it("btsieve should respond with no match when querying an existing ethereum transaction receipt query", async function() {
                this.timeout(1000);
                return chai
                    .request(location)
                    .get("")
                    .then(res => {
                        res.should.have.status(200);
                        res.body.matches.should.be.empty;
                    });
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
                        token_contract_address,
                        transfer_token_data,
                        0
                    );

                let body = (await btsieve.pollUntilMatches(
                    location
                )) as IdMatchResponse;

                body.matches.should.have.lengthOf(1);
                body.matches[0].id.should.equal(receipt.transactionHash);
                body.matches[0].id.should.match(/^0x/);
            });

            it("btsieve should return transaction and receipt if `return_as` is given", async function() {
                let body: any = await btsieve.pollUntilMatches(
                    location + "?return_as=transaction_and_receipt"
                );
                body.matches.should.have.lengthOf(1);
                body.matches[0].transaction.should.be.a("object");
                body.matches[0].receipt.should.be.a("object");
            });

            it("btsieve should respond with no content when deleting an existing ethereum transaction receipt query", async function() {
                return chai
                    .request(location)
                    .del("")
                    .then(res => {
                        res.should.have.status(204);
                    });
            });
        });
    });
});

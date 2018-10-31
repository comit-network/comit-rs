const BigNumber = require('bignumber.js');
const chai = require('chai');
chai.use(require('chai-http'));
const EthereumTx = require('ethereumjs-tx');
const ethutil = require('ethereumjs-util');
const fs = require('fs');
const should = chai.should();
const test_lib = require("../test_lib.js");
const Toml = require('toml');
const Web3 = require('web3');

const bitcoin_rpc_client = test_lib.bitcoin_rpc_client();
const lqs = test_lib.ledger_query_service_conf("localhost", 8080);
const web3 = test_lib.web3();
const alice = test_lib.player_conf("alice", {
    txid: process.env.BTC_FUNDED_TX,
    value: parseInt(process.env.BTC_FUNDED_AMOUNT + '00000000'),
    private_key: process.env.BTC_FUNDED_PRIVATE_KEY,
    vout: parseInt(process.env.BTC_FUNDED_VOUT)
});


function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

describe("Test Ledger Query Service API", () => {

    describe('Bitcoin', () => {

        describe('Transactions', () => {

            it("LQS should respond not found when getting a non-existent bitcoin transaction query", async function () {
                return chai.request(lqs.url())
                    .get('/queries/bitcoin/transactions/1')
                    .then((res) => {
                        res.should.have.status(400);
                    });
            });

            const to_address = "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap";
            let location;
            it("LQS should respond with location when creating a valid bitcoin transaction query", async function () {
                return chai.request(lqs.url())
                    .post('/queries/bitcoin/transactions')
                    .send({
                        "to_address": to_address
                    })
                    .then((res) => {
                        res.should.have.status(201);
                        location = res.headers.location;
                        location.should.be.a('string');
                    });
            });

            it("LQS should respond with no match when querying an existing bitcoin transaction query", async function () {
                return chai.request(location)
                    .get('')
                    .then((res) => {
                        res.should.have.status(200);
                        res.body.query.to_address.should.equal(to_address);
                        res.body.query.confirmations_needed.should.equal(1);
                        res.body.matches.should.be.empty;
                    });
            });

            it("LQS should respond with transaction match when requesting on the `to_address` bitcoin transaction query", async function () {
                this.slow(1000);
                return alice.send_btc_to_p2wpkh_address(to_address, 100000000).then(() => {
                    return bitcoin_rpc_client.generate(1).then(() => {
                        return lqs.poll_until_matches(chai, location).then((body) => {
                            body.query.to_address.should.equal(to_address);
                            body.matches.should.have.lengthOf(1);
                        });
                    });
                });
            });

            it("LQS should respond with no content when deleting an existing bitcoin transaction query", async function () {
                return chai.request(location)
                    .delete('')
                    .then((res) => {
                        res.should.have.status(204);
                    });
            });

        });

        describe('Blocks', () => {

            it("LQS should respond not found when getting a non-existent bitcoin block query", async function () {
                return chai.request(lqs.url())
                    .get('/queries/bitcoin/blocks/1')
                    .then((res) => {
                        res.should.have.status(400);
                    });
            });

            const min_height = 600;
            let location;
            it("LQS should respond with location when creating a valid bitcoin block query", async function () {
                return chai.request(lqs.url())
                    .post('/queries/bitcoin/blocks')
                    .send({
                        "min_height": min_height
                    })
                    .then((res) => {
                        res.should.have.status(201);
                        location = res.headers.location;
                        location.should.be.a('string');
                    });
            });

            it("LQS should respond with no match when querying an existing bitcoin block query", async function () {
                return chai.request(location)
                    .get('')
                    .then((res) => {
                        res.should.have.status(200);
                        res.body.query.min_height.should.equal(min_height);
                        res.body.matches.should.be.empty;
                    });
            });

            it("LQS should respond with no block match (yet) when requesting on the min_height 600 bitcoin block query", async function () {
                this.slow(500);
                return bitcoin_rpc_client.generate(50).then(() => {
                    return chai.request(location)
                        .get('')
                        .then((res) => {
                            res.should.have.status(200);
                            res.body.query.min_height.should.equal(min_height);
                            res.body.matches.should.be.empty;
                        });
                });
            });

            it("LQS should respond with block match when requesting on the min_height 600 bitcoin block query", async function () {
                this.slow(2000);
                this.timeout(3000);
                return bitcoin_rpc_client.generate(200).then(() => {
                    return lqs.poll_until_matches(chai, location).then((body) => {
                        body.query.min_height.should.equal(min_height);
                        body.matches.length.should.greaterThan(1);
                    });
                });
            });

            it("LQS should respond with no content when deleting an existing bitcoin block query", async function () {
                return chai.request(location)
                    .delete('')
                    .then((res) => {
                        res.should.have.status(204);
                    });
            });

        });
    });

    describe('Ethereum', () => {

        describe('Transactions', () => {

            before(async function () {
                Promise.all(
                    [
                        await test_lib.give_eth_to(alice.eth_address(), "10")
                    ]
                ).then(receipt => {
                    console.log(`Giving 10 Ether to Alice; success : ${receipt[0].status}`);
                }).catch(error => {
                    console.log(`Error on giving Ether to Alice: ${error}`);
                });
            });

            it("LQS should respond not found when getting a non-existent ethereum transaction query", async function () {
                return chai.request(lqs.url())
                    .get('/queries/ethereum/transactions/1')
                    .then((res) => {
                        res.should.have.status(400);
                    });
            });

            const to_address = "0x00a329c0648769a73afac7f9381e08fb43dbea72";
            let location;
            it("LQS should respond with location when creating a valid ethereum transaction query", async function () {
                return chai.request(lqs.url())
                    .post('/queries/ethereum/transactions')
                    .send({
                        "to_address": to_address
                    })
                    .then((res) => {
                        res.should.have.status(201);
                        location = res.headers.location;
                        location.should.be.a('string');
                    });
            });

            it("LQS should respond with no match when querying an existing ethereum transaction query", async function () {
                return chai.request(location)
                    .get('')
                    .then((res) => {
                        res.should.have.status(200);
                        res.body.query.to_address.should.equal(to_address);
                        res.body.matches.should.be.empty;
                    });
            });

            it("LQS should respond with no transaction match (yet) when requesting on the `to_address` ethereum block query", async function () {
                return alice.send_eth_transaction_to("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "", 1).then(() => {
                    return chai.request(location)
                        .get('')
                        .then((res) => {
                            res.should.have.status(200);
                            res.body.query.to_address.should.equal(to_address);
                            res.body.matches.should.be.empty;
                        });
                });
            });

            it("LQS should respond with transaction match when requesting on the `to_address` ethereum transaction query", async function () {
                this.slow(2000);
                return alice.send_eth_transaction_to(to_address, "", 5).then(() => {
                    return lqs.poll_until_matches(chai, location).then((body) => {
                        body.query.to_address.should.equal(to_address);
                        body.matches.should.lengthOf(1);
                    });
                });
            });

            it("LQS should respond with no content when deleting an existing ethereum transaction query", async function () {
                return chai.request(lqs.url())
                    .delete('/queries/ethereum/transactions/1')
                    .then((res) => {
                        res.should.have.status(204);
                    });
            });

        });

        describe('Blocks', () => {

            it("LQS should respond not found when getting a non-existent ethereum block query", async function () {
                return chai.request(lqs.url())
                    .get('/queries/ethereum/blocks/1')
                    .then((res) => {
                        res.should.have.status(400);
                    });
            });

            let location;
            const epoch_seconds_now = Math.round(Date.now() / 1000);;
            const min_timestamp_secs = epoch_seconds_now + 3;
            it("LQS should respond with location when creating a valid ethereum block query", async function () {
                this.timeout(1000);
                return chai.request(lqs.url())
                    .post('/queries/ethereum/blocks')
                    .send({
                        "min_timestamp_secs": min_timestamp_secs
                    })
                    .then((res) => {
                        res.should.have.status(201);
                        location = res.headers.location;
                        location.should.be.a('string');
                    });
            });

            it("LQS should respond with no match when querying an existing ethereum block query", async function () {
                this.timeout(1000);
                return chai.request(location)
                    .get('')
                    .then((res) => {
                        res.should.have.status(200);
                        res.body.query.min_timestamp_secs.should.equal(min_timestamp_secs);
                        res.body.matches.should.be.empty;
                    });
            });

            it("LQS should respond with block match when requesting on the timestamp ethereum block query after waiting 3 seconds", async function () {
                this.timeout(80000);
                this.slow(6000);
                return sleep(3000).then(() => {
                    return alice.send_eth_transaction_to("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "", 1);
                }).then(() => {
                    return lqs.poll_until_matches(chai, location).then((body) => {
                        body.query.min_timestamp_secs.should.equal(min_timestamp_secs);
                        body.matches.should.lengthOf(1);
                    });
                });
            });

            it("LQS should respond with no content when deleting an existing ethereum block query", async function () {
                return chai.request(location)
                    .delete('')
                    .then((res) => {
                        res.should.have.status(204);
                    });
            });

        });
    });
});

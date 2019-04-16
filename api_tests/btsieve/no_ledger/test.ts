import * as bitcoin from "../../lib/bitcoin";
import { Wallet } from "../../lib/wallet";
import * as chai from "chai";
import chaiHttp = require("chai-http");
import * as ethereum from "../../lib/ethereum";
import { HarnessGlobal, sleep } from "../../lib/util";
import {
    IdMatchResponse,
    EthereumTransactionResponse,
    Btsieve,
} from "../../lib/btsieve";

const should = chai.should();
chai.use(chaiHttp);

declare var global: HarnessGlobal;

const btsieve = new Btsieve("main", global.config, global.project_root);

const tobyWallet = new Wallet("toby", {
    ethConfig: global.ledgers_config.ethereum,
    btcConfig: global.ledgers_config.bitcoin,
});

const aliceWallet = new Wallet("alice", {
    ethConfig: global.ledgers_config.ethereum,
});

const alice_wallet_address = aliceWallet.eth().address();

setTimeout(async function() {
    describe("Test btsieve API - no ledger connected", () => {
        let token_contract_address: string;
        before(async function() {
            this.timeout(5000);
            await bitcoin.ensureSegwit();
            await tobyWallet.btc().fund(5);
            await tobyWallet.eth().fund("20");
            await aliceWallet.eth().fund("1");

            let receipt = await tobyWallet
                .eth()
                .deployErc20TokenContract(global.project_root);
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
                before(async () => {
                    await tobyWallet.eth().fund("10");
                });

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
                before(async () => {
                    await tobyWallet.eth().fund("10");
                });

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

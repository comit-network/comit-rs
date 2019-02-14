const bitcoin = require("bitcoinjs-lib");
const BitcoinRpcClient = require("bitcoin-core");
const myBitcoin = require("./bitcoin");
const BitcoinWallet = myBitcoin.BitcoinWallet;
const Buffer = require("buffer").Buffer;

const OMNI_HEADER = "6f6d6e69";

function payloadToEmbed(payload) {
    const buffer = Buffer.from(OMNI_HEADER + payload, "hex");
    return bitcoin.payments.embed({ data: [buffer] });
}

//FIXME: Remove this whenever this change:
// https://github.com/bitcoinjs/bitcoinjs-lib/commit/44a98c0fa6487eaf81500427366787a953ff890d#diff-9e60abeb4e2333a5d2f02de53b4edfac
// Hits npm!
const regtest = {
    messagePrefix: "\x18Bitcoin Signed Message:\n",
    bech32: "bcrt",
    bip32: {
        public: 0x043587cf,
        private: 0x04358394,
    },
    pubKeyHash: 0x6f,
    scriptHash: 0xc4,
    wif: 0xef,
};

let _rpc_client;

function create_omni_rpc_client() {
    const config = global.harness.ledgers_config.omnilayer;
    if (!config) {
        throw new Error("ledger.omnilayer configuration is needed");
    }
    return (_rpc_client =
        _rpc_client ||
        new BitcoinRpcClient({
            network: "regtest",
            port: config.rpc_port,
            host: config.rpc_host,
            username: config.rpc_username,
            password: config.rpc_password,
        }));
}

module.exports.create_client = () => {
    return create_omni_rpc_client();
};

module.exports.omni_generate = async function(num = 1) {
    return create_omni_rpc_client().generate(num);
};

module.exports.activate_segwit = async function() {
    return create_omni_rpc_client().generate(432);
};

module.exports.omni_import_address = async function(address) {
    return create_omnni_rpc_client().importAddress(address);
};

module.exports.swaperoo = async function(aliceDetails, bobDetails, tokenId, omni_value, btc_value) {
    if (!tokenId) {
        throw new Error("tokenId must be provided, got: " + tokenId);
    }
    // Caveat: If alice has a lot of BTC on her omni output it will all go to Bob

    // alice_output = prev output for omni = new output for BTC (should only have dust BTC)
    const { alice_keypair, alice_omni_utxo, alice_final_address } = aliceDetails;
    // bob_btc_output = prev output for BTC = output for BTC change
    // bob_omni_output = new output for Omni
    const { bob_keypair, bob_btc_utxo, bob_btc_output, bob_final_address } = bobDetails;

    const alice_output = bitcoin.address.toOutputScript(alice_final_address, regtest);
    const bob_omni_output = bitcoin.address.toOutputScript(bob_final_address, regtest);

    if (bob_omni_output === bob_btc_output) {
        throw new Error("Bob BTC and Omni output MUST be different");
    }

    const txb = new bitcoin.TransactionBuilder();

    const fee = 2500;
    const dust = 500;
    const bob_btc = bob_btc_utxo.value - fee / 2 - btc_value - dust;
    const alice_btc = alice_omni_utxo.value - fee / 2 + btc_value;

    // Alice Omni input
    txb.addInput(alice_omni_utxo.txid, alice_omni_utxo.vout, null, alice_output);
    // Bob BTC input
    txb.addInput(bob_btc_utxo.txid, bob_btc_utxo.vout, null, bob_btc_output);

    // Add BTC change back to Bob
    txb.addOutput(bob_btc_output, bob_btc);
    // Add BTC output to Alice
    txb.addOutput(alice_output, alice_btc);
    // Add Omni output to Bob (it's Omni because it's different from the inputs)
    txb.addOutput(bob_omni_output, dust);

    // Add Omni instructions
    const payload = await create_omni_rpc_client().command([
        {
            method: "omni_createpayload_simplesend",
            parameters: [tokenId, omni_value.toString()],
        },
    ]);
    const embed = payloadToEmbed(payload[0]);
    txb.addOutput(embed.output, 500);

    txb.sign(0, alice_keypair, null, null, alice_omni_utxo.value);
    txb.sign(1, bob_keypair, null, null, bob_btc_utxo.value);

    const tx = await create_omni_rpc_client().command([
        {
            method: "omni_decodetransaction",
            parameters: [txb.buildIncomplete().toHex()],
        },
    ]);
    console.log("--\nSwaperoo Omni transaction:", tx);

    const plainTransaction = await _rpc_client.decodeRawTransaction(txb.buildIncomplete().toHex());
    console.log("----\nFinal transaction:", plainTransaction);
    const txid = await _rpc_client.sendRawTransaction(txb.build().toHex());
    await _rpc_client.generate(10);

    const balance = await create_omni_rpc_client().command([
        {
            method: "omni_getallbalancesforid",
            parameters: [tokenId],
        },
    ]);
    console.log("---\nBalance after swaperoo:", balance);
};

class OmniWallet extends BitcoinWallet {
    constructor() {
        super();
    }

    async btcFund(value) {
        await this.fund(value, _rpc_client);
        await this.fund(value, _rpc_client);
        await this.fund(value, _rpc_client);
        await this.fund(value, _rpc_client);
        await this.fund(value, _rpc_client);
    }

    async createOmniToken() {
        const txb = new bitcoin.TransactionBuilder();
        const utxo = this.bitcoin_utxos.shift();
        const input_amount = utxo.value;
        const key_pair = this.keypair;
        const fee = 2500;
        const change = input_amount - fee;
        txb.addInput(utxo.txid, utxo.vout, null, this.identity().output);
        txb.addOutput(this.identity().output, change);

        const payload = await create_omni_rpc_client().command([
            {
                method: "omni_createpayload_issuancemanaged",
                parameters: [
                    2,
                    1,
                    0,
                    "Money",
                    "Gonna make you rich",
                    "Regtest Token",
                    "",
                    "Better swap those tokens",
                ],
            },
        ]);

        const embed = payloadToEmbed(payload[0]);
        txb.addOutput(embed.output, 500);

        txb.sign(0, key_pair, null, null, input_amount);

        await _rpc_client.sendRawTransaction(txb.build().toHex());
        await _rpc_client.generate(10);

        const properties = await create_omni_rpc_client().command([
            {
                method: "omni_listproperties",
                parameters: [],
            },
        ]);

        function isRegtestToken(property) {
            return property.name === "Regtest Token";
        }

        return properties[0].find(isRegtestToken);
    }

    async grantOmniToken(tokenId, recipientOutput) {
        if (!tokenId) {
            throw new Error("tokenId must be provided, got: " + tokenId);
        }

        const txb = new bitcoin.TransactionBuilder();
        const utxo = this.bitcoin_utxos.shift();
        const input_amount = utxo.value;
        const key_pair = this.keypair;
        const fee = 2500;
        const change = input_amount - fee;
        txb.addInput(utxo.txid, utxo.vout, null, this.identity().output);
        txb.addOutput(recipientOutput, change);

        console.log("---\nGrant Output:", bitcoin.script.toASM(recipientOutput));

        const payload = await create_omni_rpc_client().command([
            {
                method: "omni_createpayload_grant",
                parameters: [tokenId, "9000", ""],
            },
        ]);

        const embed = payloadToEmbed(payload[0]);
        txb.addOutput(embed.output, 500);

        txb.sign(0, key_pair, null, null, input_amount);

        const txid = await _rpc_client.sendRawTransaction(txb.build().toHex());
        await _rpc_client.generate(10);

        const balance = await create_omni_rpc_client().command([
            {
                method: "omni_getallbalancesforid",
                parameters: [tokenId],
            },
        ]);
        console.log("---\nBalance after granting:", balance);
        console.log("Hopefully address with balance:", this.identity().address);

        // const rawTransaction = await _rpc_client.getRawTransaction(txid);
        // const plainTransaction = await _rpc_client.decodeRawTransaction(rawTransaction);
        // console.log("---\nGranting Transaction:", JSON.stringify(plainTransaction, null, 2));

        return { txid: txid, vout: 0, value: change }; // We assume bitcoin-js preserves the order
    }
}

module.exports.create_wallet = () => {
    return new OmniWallet();
};

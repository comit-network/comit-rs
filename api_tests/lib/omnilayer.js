const bitcoin = require("bitcoinjs-lib");
const BitcoinRpcClient = require("bitcoin-core");
const myBitcoin = require("./bitcoin");
const BitcoinWallet = myBitcoin.BitcoinWallet;
const Buffer = require("buffer").Buffer;
const hash = require("hash.js");
const regtest = bitcoin.networks.regtest;

const OMNI_HEADER = "6f6d6e69";

function payloadToEmbed(payload) {
    const buffer = Buffer.from(OMNI_HEADER + payload, "hex");
    return bitcoin.payments.embed({ data: [buffer] });
}

let _rpc_client;

function createOmniRpcClient() {
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

module.exports.getBalance = async function(tokenId, address) {
    const res = await _rpc_client.command([
        {
            method: "omni_getbalance",
            parameters: [address, tokenId],
        },
    ]);
    return res[0].balance;
};

module.exports.createClient = () => {
    return createOmniRpcClient();
};

module.exports.generate = async function(num = 1) {
    return createOmniRpcClient().generate(num);
};

module.exports.activateSegwit = async function() {
    return createOmniRpcClient().generate(432);
};

module.exports.createOmniToken = async function(name, keypair, utxo, output) {
    const txb = new bitcoin.TransactionBuilder(regtest);
    const input_amount = utxo.value;
    const fee = 2500;
    const change = input_amount - fee;
    txb.addInput(utxo.txid, utxo.vout, null, output);
    txb.addOutput(output, change);

    const payload = await createOmniRpcClient().command([
        {
            method: "omni_createpayload_issuancemanaged",
            parameters: [
                2,
                1,
                0,
                "Money",
                "",
                name,
                "",
                "",
            ],
        },
    ]);

    const embed = payloadToEmbed(payload[0]);
    txb.addOutput(embed.output, 0);

    txb.sign(0, keypair, null, bitcoin.Transaction.SIGHASH_ALL, input_amount);

    // const rawTransaction = await _rpc_client.getRawTransaction(txid);
    // const plainTransaction = await _rpc_client.decodeRawTransaction(txb.build().toHex());
    // console.log("---\nToken Create Transaction:", JSON.stringify(plainTransaction, null, 2));

    await createOmniRpcClient().sendRawTransaction(txb.build().toHex());
    await createOmniRpcClient().generate(1);

    const properties = await createOmniRpcClient().command([
        {
            method: "omni_listproperties",
            parameters: [],
        },
    ]);

    function isRegtestToken(property) {
        return property.name === name;
    }

    return properties[0].find(isRegtestToken);
};

module.exports.grantOmniToken = async function(keypair, utxo, prevOutput, tokenId, recipientOutput, amount) {
    if (!tokenId) {
        throw new Error("tokenId must be provided, got: " + tokenId);
    }

    const txb = new bitcoin.TransactionBuilder(regtest);
    const input_amount = utxo.value;
    const fee = 2500;
    const change = input_amount - fee;
    txb.addInput(utxo.txid, utxo.vout, null, prevOutput);
    txb.addOutput(recipientOutput, change);

    const payload = await createOmniRpcClient().command([
        {
            method: "omni_createpayload_grant",
            parameters: [tokenId, amount.toString(), ""],
        },
    ]);

    const embed = payloadToEmbed(payload[0]);
    txb.addOutput(embed.output, 0);

    txb.sign(0, keypair, null, bitcoin.Transaction.SIGHASH_ALL, input_amount);

    const txid = await _rpc_client.sendRawTransaction(txb.build().toHex());
    await createOmniRpcClient().generate(1);

    // const rawTransaction = await _rpc_client.getRawTransaction(txid);
    // const plainTransaction = await _rpc_client.decodeRawTransaction(rawTransaction);
    // console.log("---\nGranting Transaction:", JSON.stringify(plainTransaction, null, 2));

    return { txid: txid, vout: 0, value: change }; // We assume bitcoin-js preserves the order
};

module.exports.swaperoo = async function(aliceDetails, bobDetails, tokenId, omni_value, btc_value) {
    if (!tokenId) {
        throw new Error("tokenId must be provided, got: " + tokenId);
    }

    // alice_output = prev output for omni = new output for BTC
    const { alice_keypair, alice_omni_utxo, alice_final_address } = aliceDetails;
    // bob_btc_output = prev output for BTC = output for BTC change
    // bob_omni_output = new output for Omni
    const { bob_keypair, bob_btc_utxo, bob_btc_output, bob_final_address } = bobDetails;

    const alice_output = bitcoin.address.toOutputScript(alice_final_address, regtest);
    const bob_omni_output = bitcoin.address.toOutputScript(bob_final_address, regtest);

    if (bob_omni_output === bob_btc_output) {
        throw new Error("Bob BTC and Omni output MUST be different");
    }

    const txb = new bitcoin.TransactionBuilder(regtest);

    const fee = 2500;
    const dust = 546;
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
    const payload = await createOmniRpcClient().command([
        {
            method: "omni_createpayload_simplesend",
            parameters: [tokenId, omni_value.toString()],
        },
    ]);
    const embed = payloadToEmbed(payload[0]);
    txb.addOutput(embed.output, 0);

    txb.sign(0, alice_keypair, null, bitcoin.Transaction.SIGHASH_ALL, alice_omni_utxo.value);
    txb.sign(1, bob_keypair, null, bitcoin.Transaction.SIGHASH_ALL, bob_btc_utxo.value);

    // const tx = await createOmniRpcClient().command([
    //     {
    //         method: "omni_decodetransaction",
    //         parameters: [txb.buildIncomplete().toHex()],
    //     },
    // ]);
    // const plainTransaction = await _rpc_client.decodeRawTransaction(txb.buildIncomplete().toHex());

    await createOmniRpcClient().sendRawTransaction(txb.build().toHex());
    await createOmniRpcClient().generate(1);
};

module.exports.lockInHTLC = async function(aliceDetails, bobDetails, tokenId, omni_value) {
    if (!tokenId) {
        throw new Error("tokenId must be provided, got: " + tokenId);
    }

    const { alice_keypair, alice_omni_utxo, alice_final_address } = aliceDetails;
    const { bob_keypair, bob_btc_output, bob_final_address } = bobDetails;

    const alice_output = bitcoin.address.toOutputScript(alice_final_address, regtest);
    const bob_omni_output = bitcoin.address.toOutputScript(bob_final_address, regtest);

    if (bob_omni_output === bob_btc_output) {
        throw new Error("Bob BTC and Omni output MUST be different");
    }

    const txb = new bitcoin.TransactionBuilder(regtest);

    const fee = 2500;
    const dust = 546;
    const btcHtlc = fee + dust;
    const btcChange = alice_omni_utxo.value - 2 * fee - 2 * dust;

    // Alice Omni input
    txb.addInput(alice_omni_utxo.txid, alice_omni_utxo.vout, null, alice_output);

    // Add BTC change back to Alice
    txb.addOutput(alice_output, btcChange);

    // Add Omni instructions
    const payload = await createOmniRpcClient().command([
        {
            method: "omni_createpayload_simplesend",
            parameters: [tokenId, omni_value.toString()],
        },
    ]);
    const embed = payloadToEmbed(payload[0]);
    txb.addOutput(embed.output, 0);

    const recipientPubkeyHash = bitcoin.crypto.hash160(bob_keypair.publicKey).toString("hex");

    const senderPubkeyHash = bitcoin.crypto.hash160(alice_keypair.publicKey).toString("hex");

    const secret = 0x0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF;

    const secretHash = "4884fdaafea47c29fea7159d0daddd9c085d6200e1359e85bb81736af6b7c837";

    const scriptASM = "OP_IF " +
        "OP_SIZE " +
        "20 " +
        "OP_EQUALVERIFY " +
        "OP_SHA256 " +
        secretHash +
        " OP_EQUALVERIFY " +
        "OP_DUP " +
        "OP_HASH160 " +
        recipientPubkeyHash +
        " OP_ELSE " +
        "f08fbecf00 " +
        "OP_NOP2 " +
        "OP_DROP " +
        "OP_DUP " +
        "OP_HASH160 " +
        senderPubkeyHash +
        " OP_ENDIF " +
        "OP_EQUALVERIFY " +
        "OP_CHECKSIG";

    const script = bitcoin.script.fromASM(scriptASM);

    const { address } = bitcoin.payments.p2sh({ redeem: { output: script, network: regtest }, network: regtest });

    txb.addOutput(address, btcHtlc);
    txb.sign(0, alice_keypair, null, bitcoin.Transaction.SIGHASH_ALL, alice_omni_utxo.value);
    const txHex = txb.build().toHex();
    // const omniTransaction = await createOmniRpcClient().command([
    //     {
    //         method: "omni_decodetransaction",
    //         parameters: [txHex],
    //     },
    // ]);bt
    // console.log("---\nomni transaction:", JSON.stringify(omniTransaction, null, 2));
    // const plainTransaction = await _rpc_client.decodeRawTransaction(txHex);
    // console.log("---\nplainTransaction:", JSON.stringify(plainTransaction, null, 2));

    const txId = await createOmniRpcClient().sendRawTransaction(txHex);
    await createOmniRpcClient().generate(1);
    return { txid: txId, vout: 2, value: btcHtlc, address: address, script: script };
};

module.exports.redeemHTLC = async function(redeemScript, bobDetails, htlcUTXO, tokenId, omni_value) {
    if (!tokenId) {
        throw new Error("tokenId must be provided, got: " + tokenId);
    }

    const secret = "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF";

    const { bob_keypair, bob_btc_output, bob_final_address } = bobDetails;

    const bob_omni_output = bitcoin.address.toOutputScript(bob_final_address, regtest);

    if (bob_omni_output === bob_btc_output) {
        throw new Error("Bob BTC and Omni output MUST be different");
    }

    const txb = new bitcoin.TransactionBuilder(regtest);

    const fee = 2500;
    const btc = htlcUTXO.value - fee;

    // HTLC input
    txb.addInput(htlcUTXO.txid, htlcUTXO.vout, null);

    // Add BTC change back to Alice
    txb.addOutput(bob_final_address, btc);

    // Add Omni instructions
    const payload = await createOmniRpcClient().command([
        {
            method: "omni_createpayload_simplesend",
            parameters: [tokenId, omni_value.toString()],
        },
    ]);
    const embed = payloadToEmbed(payload[0]);
    txb.addOutput(embed.output, 0);

    // Redeem script
    const hashType = bitcoin.Transaction.SIGHASH_ALL;
    const tx = txb.buildIncomplete();
    const signatureHash = tx.hashForSignature(0, redeemScript, hashType);
    const secretBuffer = Buffer.from(secret, "hex");
    const redeemScriptSig = bitcoin.payments.p2sh({
        redeem: {
            input: bitcoin.script.compile([
                bitcoin.script.signature.encode(bob_keypair.sign(signatureHash), hashType),
                bob_keypair.publicKey,
                secretBuffer,
                bitcoin.opcodes.OP_TRUE,
            ]),
            output: redeemScript,
        },
    }).input;
    tx.setInputScript(0, redeemScriptSig);

    const txHex = tx.toHex();
    // const omniTransaction = await createOmniRpcClient().command([
    //     {
    //         method: "omni_decodetransaction",
    //         parameters: [txHex],
    //     },
    // ]);
    // console.log("---\nomni transaction:", JSON.stringify(omniTransaction, null, 2));
    // const plainTransaction = await _rpc_client.decodeRawTransaction(txHex);
    // console.log("---\nplainTransaction:", JSON.stringify(plainTransaction, null, 2));

    const txId = await createOmniRpcClient().sendRawTransaction(txHex);
    await createOmniRpcClient().generate(1);
};

module.exports.lockInWitnessHTLC = async function(aliceDetails, bobDetails, tokenId, omni_value) {
    if (!tokenId) {
        throw new Error("tokenId must be provided, got: " + tokenId);
    }

    const { alice_keypair, alice_omni_utxo, alice_final_address } = aliceDetails;
    const { bob_keypair, bob_btc_output, bob_final_address } = bobDetails;

    const alice_output = bitcoin.address.toOutputScript(alice_final_address, regtest);
    const bob_omni_output = bitcoin.address.toOutputScript(bob_final_address, regtest);

    if (bob_omni_output === bob_btc_output) {
        throw new Error("Bob BTC and Omni output MUST be different");
    }

    const txb = new bitcoin.TransactionBuilder(regtest);

    const fee = 2500;
    const dust = 546;
    const btcHtlc = fee + dust;
    const btcChange = alice_omni_utxo.value - 2 * fee - 2 * dust;

    // Alice Omni input
    txb.addInput(alice_omni_utxo.txid, alice_omni_utxo.vout, null, alice_output);

    // Add BTC change back to Alice
    txb.addOutput(alice_output, btcChange);

    // Add Omni instructions
    const payload = await createOmniRpcClient().command([
        {
            method: "omni_createpayload_simplesend",
            parameters: [tokenId, omni_value.toString()],
        },
    ]);
    const embed = payloadToEmbed(payload[0]);
    txb.addOutput(embed.output, 0);

    const recipientPubkeyHash = bitcoin.crypto.hash160(bob_keypair.publicKey).toString("hex");

    const senderPubkeyHash = bitcoin.crypto.hash160(alice_keypair.publicKey).toString("hex");

    const secret = 0x0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF;

    const secretHash = "4884fdaafea47c29fea7159d0daddd9c085d6200e1359e85bb81736af6b7c837";

    const scriptASM = "OP_IF " +
        "OP_SIZE " +
        "20 " +
        "OP_EQUALVERIFY " +
        "OP_SHA256 " +
        secretHash +
        " OP_EQUALVERIFY " +
        "OP_DUP " +
        "OP_HASH160 " +
        recipientPubkeyHash +
        " OP_ELSE " +
        "f08fbecf00 " +
        "OP_NOP2 " +
        "OP_DROP " +
        "OP_DUP " +
        "OP_HASH160 " +
        senderPubkeyHash +
        " OP_ENDIF " +
        "OP_EQUALVERIFY " +
        "OP_CHECKSIG";

    const script = bitcoin.script.fromASM(scriptASM);

    const p2wsh = bitcoin.payments.p2wsh({
        redeem: { output: script, network: regtest },
        network: regtest,
    });
    const p2sh = bitcoin.payments.p2sh({ redeem: p2wsh, network: regtest });

    txb.addOutput(p2sh.address, btcHtlc);
    txb.sign(0, alice_keypair, null, bitcoin.Transaction.SIGHASH_ALL, alice_omni_utxo.value);
    const txHex = txb.build().toHex();
    // const omniTransaction = await createOmniRpcClient().command([
    //     {
    //         method: "omni_decodetransaction",
    //         parameters: [txHex],
    //     },
    // ]);
    // console.log("---\nomni transaction:", JSON.stringify(omniTransaction, null, 2));
    // const plainTransaction = await _rpc_client.decodeRawTransaction(txHex);
    // console.log("---\nplainTransaction:", JSON.stringify(plainTransaction, null, 2));

    const txId = await createOmniRpcClient().sendRawTransaction(txHex);
    await createOmniRpcClient().generate(1);
    return { txid: txId, vout: 2, value: btcHtlc, address: p2sh.address, script: script, p2wsh: p2wsh };
};

module.exports.redeemWitnessHTLC = async function(redeemScript, p2wsh, bobDetails, htlcUTXO, tokenId, omni_value) {
    if (!tokenId) {
        throw new Error("tokenId must be provided, got: " + tokenId);
    }

    const secret = "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF";

    const { bob_keypair, bob_btc_output, bob_final_address } = bobDetails;

    const bob_omni_output = bitcoin.address.toOutputScript(bob_final_address, regtest);

    if (bob_omni_output === bob_btc_output) {
        throw new Error("Bob BTC and Omni output MUST be different");
    }

    const txb = new bitcoin.TransactionBuilder(regtest);

    const fee = 2500;
    const btc = htlcUTXO.value - fee;

    // HTLC input
    txb.addInput(htlcUTXO.txid, htlcUTXO.vout);

    // Add BTC change back to Alice
    txb.addOutput(bob_final_address, btc);

    // Add Omni instructions
    const payload = await createOmniRpcClient().command([
        {
            method: "omni_createpayload_simplesend",
            parameters: [tokenId, omni_value.toString()],
        },
    ]);
    const embed = payloadToEmbed(payload[0]);
    txb.addOutput(embed.output, 0);

    // Redeem script
    const hashType = bitcoin.Transaction.SIGHASH_ALL;
    const tx = txb.buildIncomplete();

    const scriptSig = bitcoin.script.compile([p2wsh.output]);
    // const signatureHash = tx.hashForSignature(0, p2wsh.output, hashType);
    const signatureHash = tx.hashForWitnessV0(0, redeemScript, htlcUTXO.value, hashType)
    const secretBuffer = Buffer.from(secret, "hex");
    const witnessStack = [
        bitcoin.script.signature.encode(bob_keypair.sign(signatureHash), hashType),
        bob_keypair.publicKey,
        secretBuffer,
        Buffer.from('01','hex'),
        p2wsh.redeem.output
    ];
    tx.setInputScript(0, scriptSig);
    tx.setWitness(0, witnessStack);

    const txHex = tx.toHex();
    // const omniTransaction = await createOmniRpcClient().command([
    //     {
    //         method: "omni_decodetransaction",
    //         parameters: [txHex],
    //     },
    // ]);
    // console.log("---\nomni transaction:", JSON.stringify(omniTransaction, null, 2));
    // const plainTransaction = await _rpc_client.decodeRawTransaction(txHex);
    // console.log("---\nplainTransaction:", JSON.stringify(plainTransaction, null, 2));

    const txId = await createOmniRpcClient().sendRawTransaction(txHex);
    await createOmniRpcClient().generate(1);
};

class OmniWallet extends BitcoinWallet {
    constructor() {
        super();
    }

    async btcFund(value, base58 = false) {
        await this.fund(value, _rpc_client, base58);
        await this.fund(value, _rpc_client, base58);
        await this.fund(value, _rpc_client, base58);
        await this.fund(value, _rpc_client, base58);
        await this.fund(value, _rpc_client, base58);
    }
}

module.exports.create_wallet = () => {
    return new OmniWallet();
};

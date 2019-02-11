const bitcoin = require("bitcoinjs-lib");
const BitcoinRpcClient = require("bitcoin-core");
const myBitcoin = require("./bitcoin");
const BitcoinWallet = myBitcoin.BitcoinWallet;
const Buffer = require("buffer").Buffer;


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

// omnicore-cli -regtest -rpcport=19443 -rpcuser=omnicore -rpcpassword=54pLR_f7-G6is32LP-7nbhzZSbJs_2zSATtZV_r05yg= "omni_createpayload_issuancemanaged" 2 1 0 "Money" "Gonna make you rich" "Regtest USDT" "" "Better swap those tokens"
// 00000036020001000000004d6f6e657900476f6e6e61206d616b6520796f75207269636800526567746573742055534454000042657474657220737761702074686f736520746f6b656e7300


// TODO: fix this
async function omni_balance(address) {
    let btc_balance = await _bitcoin_rpc_client.getReceivedByAddress(address);
    return parseFloat(btc_balance) * 100000000;
}

// TODO: fix this
module.exports.log_omni_balance = async function(
    when,
    player,
    address,
    address_type,
) {
    global.harness.logger.info(
        "%s the swap, %s has %s satoshis at the %s address %s",
        when,
        player,
        await btc_balance(address),
        address_type,
        address,
    );
};

class OmniWallet extends BitcoinWallet {

    constructor() {
        super();
    }

    async omniFund(value) {
        await this.fund(value, _rpc_client);
    }

    async sendIssuanceManaged() {

        const utxo = this.bitcoin_utxos.shift();
        const address = this.identity().address;
        console.log("Address:", address);
        return await create_omni_rpc_client().command([{
            "method": "omni_sendissuancemanaged",
            "parameters": [address, 2, 1, 0, "Money", "Gonna make you rich", "Regtest USDT", "", "Better swap those tokens"],
        }]);
    }

    async createPayloadIssuanceManaged() {
        const txb = new bitcoin.TransactionBuilder();
        const utxo = this.bitcoin_utxos.shift();
        const input_amount = utxo.value;
        const key_pair = this.keypair;
        const fee = 2500;
        const change = input_amount - fee;
        txb.addInput(utxo.txid, utxo.vout, null, this.identity().output);
        txb.addOutput(this.identity().output, change);

        const payload = await create_omni_rpc_client().command([{
            "method": "omni_createpayload_issuancemanaged",
            "parameters": [2, 1, 0, "Money", "Gonna make you rich", "Regtest USDT", "", "Better swap those tokens"],
        }]);

        const OMNI_HEADER = "6f6d6e69";

        const payload_with_header = OMNI_HEADER + payload[0];

        // const omnirawtx = await create_omni_rpc_client().command([{
        //     "method": "omni_createrawtx_opreturn",
        //     "parameters": [ txb.buildIncomplete().toHex(), payload[0]],
        // }]);
        //
        // console.log("omnirawtx:" + omnirawtx[0]);
        //
        // const omnitx = await create_omni_rpc_client().command([{
        //     "method": "decoderawtransaction",
        //     "parameters": omnirawtx,
        // }]);
        //
        // console.log("omnitx:", JSON.stringify(omnitx[0]));


        const buffer = Buffer.from(payload_with_header, "hex");

        const embed = bitcoin.payments.embed({ data: [buffer] });
        txb.addOutput(embed.output, 500);

        const tx = await create_omni_rpc_client().command([{
            "method": "omni_decodetransaction",
            "parameters": [ txb.buildIncomplete().toHex()],
        }]);

        console.log("tx: ", JSON.stringify(tx[0]));

        txb.sign(0, key_pair, null, null, input_amount);

        return _rpc_client.sendRawTransaction(txb.build().toHex());
    };

}

module.exports.create_wallet = () => {
    return new OmniWallet();
};

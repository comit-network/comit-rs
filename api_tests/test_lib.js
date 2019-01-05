const web3_conf = require("./web3_conf.js");

const web3 = web3_conf.create();

{
    let _test_rng_counter = 0;

    function test_rng() {
        _test_rng_counter++;
        return Buffer.from(("" + _test_rng_counter).padStart(32, "0"));
    }

    module.exports.test_rng = test_rng;
}

{
    const logger = global.harness.logger;
    module.exports.logger = function() {
        return logger;
    };
}

{
    async function sleep(time) {
        return new Promise((res, rej) => {
            setTimeout(res, time);
        });
    }

    module.exports.sleep = sleep;
}

{
    function project_root() {
        return global.harness.project_root;
    }

    module.exports.project_root = project_root();
}

{
    const function_identifier = "40c10f19";
    module.exports.mint_erc20_tokens = (
        owner_wallet,
        contract_address,
        to_address,
        amount
    ) => {
        to_address = to_address.replace(/^0x/, "").padStart(64, "0");
        amount = web3.utils
            .numberToHex(amount)
            .replace(/^0x/, "")
            .padStart(64, "0");
        const payload = "0x" + function_identifier + to_address + amount;

        return owner_wallet
            .eth()
            .send_eth_transaction_to(contract_address, payload, "0x0");
    };
}

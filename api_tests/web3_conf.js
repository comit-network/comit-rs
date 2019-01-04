const Web3 = require("web3");

let web3;

module.exports.create = () => {
    if (global.harness.ledgers_config.ethereum) {
        const eth_config = global.harness.ledgers_config.ethereum;
        return (web3 =
            web3 ||
            new Web3(new Web3.providers.HttpProvider(eth_config.rpc_url)));
    } else {
        return new Web3();
    }
};

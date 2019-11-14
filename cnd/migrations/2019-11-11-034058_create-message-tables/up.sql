-- Your SQL goes here

CREATE TABLE rfc003_bitcoin_ethereum_bitcoin_ether_request_messages
(
    id INTEGER               NOT NULL PRIMARY KEY,
    swap_id UNIQUE           NOT NULL,
    bitcoin_network          NOT NULL,
    ethereum_chain_id        NOT NULL,
    bitcoin_amount           NOT NULL,
    ether_amount             NOT NULL,
    hash_function            NOT NULL,
    bitcoin_refund_identity  NOT NULL,
    ethereum_redeem_identity NOT NULL,
    bitcoin_expiry           NOT NULL,
    ethereum_expiry          NOT NULL,
    secret_hash              NOT NULL
);

CREATE TABLE rfc003_ethereum_bitcoin_ether_bitcoin_request_messages
(
    id INTEGER               NOT NULL PRIMARY KEY,
    swap_id UNIQUE           NOT NULL,
    bitcoin_network          NOT NULL,
    ethereum_chain_id        NOT NULL,
    bitcoin_amount           NOT NULL,
    ether_amount             NOT NULL,
    hash_function            NOT NULL,
    bitcoin_redeem_identity  NOT NULL,
    ethereum_refund_identity NOT NULL,
    bitcoin_expiry           NOT NULL,
    ethereum_expiry          NOT NULL,
    secret_hash              NOT NULL
);

CREATE TABLE rfc003_bitcoin_ethereum_bitcoin_erc20_request_messages
(
    id INTEGER               NOT NULL PRIMARY KEY,
    swap_id UNIQUE           NOT NULL,
    bitcoin_network          NOT NULL,
    ethereum_chain_id        NOT NULL,
    bitcoin_amount           NOT NULL,
    erc20_amount             NOT NULL,
    erc20_token_contract     NOT NULL,
    hash_function            NOT NULL,
    bitcoin_refund_identity  NOT NULL,
    ethereum_redeem_identity NOT NULL,
    bitcoin_expiry           NOT NULL,
    ethereum_expiry          NOT NULL,
    secret_hash              NOT NULL
);

CREATE TABLE rfc003_ethereum_bitcoin_erc20_bitcoin_request_messages
(
    id INTEGER               NOT NULL PRIMARY KEY,
    swap_id UNIQUE           NOT NULL,
    bitcoin_network          NOT NULL,
    ethereum_chain_id        NOT NULL,
    bitcoin_amount           NOT NULL,
    erc20_amount             NOT NULL,
    erc20_token_contract     NOT NULL,
    hash_function            NOT NULL,
    bitcoin_redeem_identity  NOT NULL,
    ethereum_refund_identity NOT NULL,
    bitcoin_expiry           NOT NULL,
    ethereum_expiry          NOT NULL,
    secret_hash              NOT NULL
);

CREATE TABLE rfc003_ethereum_bitcoin_accept_messages
(
    id INTEGER               NOT NULL PRIMARY KEY,
    swap_id UNIQUE           NOT NULL,
    bitcoin_refund_identity  NOT NULL,
    ethereum_redeem_identity NOT NULL
);

CREATE TABLE rfc003_bitcoin_ethereum_accept_messages
(
    id INTEGER               NOT NULL PRIMARY KEY,
    swap_id UNIQUE           NOT NULL,
    bitcoin_redeem_identity  NOT NULL,
    ethereum_refund_identity NOT NULL
);

CREATE TABLE rfc003_decline_messages
(
    id INTEGER     NOT NULL PRIMARY KEY,
    swap_id UNIQUE NOT NULL,
    reason
);


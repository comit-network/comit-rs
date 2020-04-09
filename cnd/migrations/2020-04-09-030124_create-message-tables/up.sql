-- Your SQL goes here

CREATE TABLE dial_infos
(
        id             INTEGER NOT NULL PRIMARY KEY,
        local_swap_id  UNIQUE NOT NULL,
        peer_id        NOT NULL
);

CREATE TABLE han_ethereum_created_swaps
(
        id             INTEGER NOT NULL PRIMARY KEY,
        local_swap_id  UNIQUE NOT NULL,
        role           NOT NULL,
        chain_id       NOT NULL,
        my_identity    NOT NULL,
        expiry         NOT NULL,
        amount         NOT NULL
);

CREATE TABLE han_bitcoin_created_swaps
(
        id             INTEGER NOT NULL PRIMARY KEY,
        local_swap_id  UNIQUE NOT NULL,
        role           NOT NULL,
        network        NOT NULL,
        my_identity    NOT NULL,
        expiry         NOT NULL,
        amount         NOT NULL
);


CREATE TABLE herc20_created_swaps
(
        id             INTEGER NOT NULL PRIMARY KEY,
        local_swap_id  UNIQUE NOT NULL,
        role           NOT NULL,
        chain_id       NOT NULL,
        my_identity    NOT NULL,
        expiry         NOT NULL,
        amount         NOT NULL,
        token_contract NOT NULL
);

CREATE TABLE halight_created_swaps
(
        id             INTEGER NOT NULL PRIMARY KEY,
        local_swap_id  UNIQUE NOT NULL,
        role           NOT NULL,
        network        NOT NULL,
        my_identity    NOT NULL,
        expiry         NOT NULL,
        amount         NOT NULL
);

CREATE TABLE finalized_swaps
(
    id                  INTEGER NOT NULL PRIMARY KEY,
    swap_id             UNIQUE NOT NULL,
    alpha_identity      NOT NULL,
    beta_identity       NOT NULL,
    secret_hash         NOT NULL,
    finalized_at        DATETIME DEFAULT CURRENT_TIMESTAMP
);

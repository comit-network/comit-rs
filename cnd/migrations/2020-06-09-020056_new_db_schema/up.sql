-- Your SQL goes here

CREATE TABLE swaps
(
    id                          INTEGER NOT NULL PRIMARY KEY,
    local_swap_id               UNIQUE NOT NULL,
    role                        NOT NULL,
    counterparty_peer_id        NOT NULL,
    start_of_swap               DATETIME NOT NULL
);

CREATE TABLE hbits
(
    id                          INTEGER NOT NULL PRIMARY KEY,
    swap_id                     INTEGER NOT NULL,
    amount                      NOT NULL,
    network                     NOT NULL,
    expiry                      NOT NULL,
    transient_identity,
    final_identity              NOT NULL,
    side                        NOT NULL,
    FOREIGN KEY(swap_id)        REFERENCES swaps(id)
);

CREATE TABLE herc20s
(
    id                          INTEGER NOT NULL PRIMARY KEY,
    swap_id                     INTEGER NOT NULL,
    amount                      NOT NULL,
    chain_id                    NOT NULL,
    expiry                      NOT NULL,
    token_contract              NOT NULL,
    redeem_identity,
    refund_identity,
    side                        NOT NULL,
    FOREIGN KEY(swap_id)        REFERENCES swaps(id)
);

CREATE TABLE halbits
(
    id                          INTEGER NOT NULL PRIMARY KEY,
    swap_id                     INTEGER NOT NULL,
    amount                      NOT NULL,
    network                     NOT NULL,
    chain                       NOT NULL,
    cltv_expiry                 NOT NULL,
    redeem_identity,
    refund_identity,
    side                        NOT NULL,
    FOREIGN KEY(swap_id)        REFERENCES swaps(id)
);

CREATE TABLE address_book
(
    id                          INTEGER NOT NULL PRIMARY KEY,
    peer_id                     NOT NULL,
    multi_address                NOT NULL
);

CREATE TABLE secret_hashes
(
    id                          INTEGER NOT NULL PRIMARY KEY,
    swap_id                     INTEGER NOT NULL,
    secret_hash                 NULL,
    FOREIGN KEY (swap_id)       REFERENCES swaps(id)
);

CREATE TABLE shared_swap_ids
(
    id                          INTEGER NOT NULL PRIMARY KEY,
    swap_id                     INTEGER NOT NULL,
    shared_swap_id              NOT NULL,
    FOREIGN KEY (swap_id)       REFERENCES swaps(id)
);

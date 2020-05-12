-- Your SQL goes here

CREATE TABLE swaps
(
    id                          INTEGER NOT NULL PRIMARY KEY,
    local_swap_id               UNIQUE NOT NULL,
    role                        NOT NULL,
    counterparty_peer_id        NOT NULL
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

CREATE TABLE address_hints
(
    id                          INTEGER NOT NULL PRIMARY KEY,
    peer_id                     UNIQUE NOT NULL,
    address_hint                NOT NULL
);

CREATE TABLE herc20s
(
    id                          INTEGER NOT NULL PRIMARY KEY,
    swap_id                     INTEGER NOT NULL,
    amount                      NOT NULL,
    chain_id                    NOT NULL,
    expiry                      NOT NULL,
    hash_function               NOT NULL,
    token_contract              NOT NULL,
    redeem_identity,
    refund_identity,
    ledger                      NOT NULL,
    FOREIGN KEY(swap_id)        REFERENCES swaps(id)
);

CREATE TABLE halights
(
    id                          INTEGER NOT NULL PRIMARY KEY,
    swap_id                     INTEGER NOT NULL,
    amount                      NOT NULL,
    network                     NOT NULL,
    chain                       NOT NULL,
    cltv_expiry                 NOT NULL,
    hash_function               NOT NULL,
    redeem_identity,
    refund_identity,
    ledger                      NOT NULL,
    FOREIGN KEY(swap_id)        REFERENCES swaps(id)
);

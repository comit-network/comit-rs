-- This file should undo anything in `up.sql`

DROP TABLE herc20s;
DROP TABLE halights;
DROP TABLE hbits;

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
    redeem_identity,
    refund_identity,
    ledger                      NOT NULL,
    FOREIGN KEY(swap_id)        REFERENCES swaps(id)
);

CREATE TABLE hbits
(
    id                          INTEGER NOT NULL PRIMARY KEY,
    swap_id                     INTEGER NOT NULL,
    amount                      NOT NULL,
    network                     NOT NULL,
    redeem_identity,
    refund_identity,
    ledger                      NOT NULL,
    FOREIGN KEY(swap_id)        REFERENCES swaps(id)
);

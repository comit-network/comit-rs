-- Your SQL goes here

-- There is no data in these tables, so we skip the backup and just immediately drop it.

DROP TABLE herc20s;
DROP TABLE halights;
DROP TABLE hbits;

-- Re-create the tables with `ledger` renamed to `side`.

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
    side                        NOT NULL,
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
    side                        NOT NULL,
    FOREIGN KEY(swap_id)        REFERENCES swaps(id)
);

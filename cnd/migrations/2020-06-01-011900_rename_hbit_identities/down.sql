-- This file should undo anything in `up.sql`

DROP TABLE hbits;

CREATE TABLE hbits
(
    id                          INTEGER NOT NULL PRIMARY KEY,
    swap_id                     INTEGER NOT NULL,
    amount                      NOT NULL,
    network                     NOT NULL,
    redeem_identity
    refund_identity
    side                        NOT NULL,
    FOREIGN KEY(swap_id)        REFERENCES swaps(id)
);

-- Your SQL goes here

-- There is no data in these tables, so we skip the backup and just immediately drop it.

DROP TABLE hbits;

-- Model Hbit identities in terms of transient vs final, rather than redeem vs refund.

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

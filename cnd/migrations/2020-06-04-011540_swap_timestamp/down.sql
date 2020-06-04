-- This file should undo anything in `up.sql`

DROP TABLE swaps;

CREATE TABLE swaps
(
    id                          INTEGER NOT NULL PRIMARY KEY,
    local_swap_id               UNIQUE NOT NULL,
    role                        NOT NULL,
    counterparty_peer_id        NOT NULL
);

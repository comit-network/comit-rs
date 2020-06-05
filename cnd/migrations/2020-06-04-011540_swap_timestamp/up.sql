-- Your SQL goes here

-- There is no data in these tables, so we skip the backup and just immediately drop it.

DROP TABLE swaps;

-- Save time when the swap was created as `start_of_swap`.

CREATE TABLE swaps
(
    id                          INTEGER NOT NULL PRIMARY KEY,
    local_swap_id               UNIQUE NOT NULL,
    role                        NOT NULL,
    counterparty_peer_id        NOT NULL,
    start_of_swap               DATETIME NOT NULL
);

-- This file should undo anything in `up.sql`

DROP TABLE address_book;

CREATE TABLE address_hints
(
    id                          INTEGER NOT NULL PRIMARY KEY,
    peer_id                     UNIQUE NOT NULL,
    address_hint                NOT NULL
);

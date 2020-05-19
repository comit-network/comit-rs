-- Your SQL goes here

DROP TABLE address_hints;

CREATE TABLE address_book
(
    id                          INTEGER NOT NULL PRIMARY KEY,
    peer_id                     NOT NULL,
    multi_address                NOT NULL
);

-- This file should undo anything in `up.sql`

CREATE TABLE address_book
(
    id                          INTEGER NOT NULL PRIMARY KEY,
    peer_id                     NOT NULL,
    multi_address                NOT NULL
);

DROP VIEW swap_contexts;

DROP TABLE orders;
DROP TABLE btc_dai_orders;
DROP TABLE order_hbit_params;
DROP TABLE order_herc20_params;

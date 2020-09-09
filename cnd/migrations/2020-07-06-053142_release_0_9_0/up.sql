-- Your SQL goes here

DROP TABLE address_book;

-- Here is how this works:
-- * COALESCE selects the first non-null value from a list of values
-- * We use 3 sub-selects to select a static value (i.e. 'halbit', etc) if that particular child table has a row with a foreign key to the parent table
-- * We do this two times, once where we limit the results to rows that have `side` set to `Alpha` and once where `side` is set to `Beta`
-- The result is a view with 5 columns: `id`, `local_swap_id`, `role`, `alpha` and `beta` where the `alpha` and `beta` columns have one of the values `halbit`, `herc20` or `hbit`
CREATE VIEW swap_contexts AS
SELECT id,
       local_swap_id,
       role,
       COALESCE(
               (SELECT 'halbit' from halbits where halbits.swap_id = swaps.id and halbits.side = 'Alpha'),
               (SELECT 'herc20' from herc20s where herc20s.swap_id = swaps.id and herc20s.side = 'Alpha'),
               (SELECT 'hbit' from hbits where hbits.swap_id = swaps.id and hbits.side = 'Alpha')
           ) as alpha,
       COALESCE(
               (SELECT 'halbit' from halbits where halbits.swap_id = swaps.id and halbits.side = 'Beta'),
               (SELECT 'herc20' from herc20s where herc20s.swap_id = swaps.id and herc20s.side = 'Beta'),
               (SELECT 'hbit' from hbits where hbits.swap_id = swaps.id and hbits.side = 'Beta')
           ) as beta
FROM swaps;

-- These should have been in the previous migration but that was already shipped.
DROP TABLE rfc003_bitcoin_ethereum_bitcoin_ether_request_messages;
DROP TABLE rfc003_ethereum_bitcoin_ether_bitcoin_request_messages;
DROP TABLE rfc003_bitcoin_ethereum_bitcoin_erc20_request_messages;
DROP TABLE rfc003_ethereum_bitcoin_erc20_bitcoin_request_messages;
DROP TABLE rfc003_ethereum_bitcoin_accept_messages;
DROP TABLE rfc003_bitcoin_ethereum_accept_messages;
DROP TABLE rfc003_decline_messages;
DROP TABLE rfc003_swaps;

CREATE TABLE orders
(
    id INTEGER      NOT NULL PRIMARY KEY,
    order_id UNIQUE NOT NULL,
    position        NOT NULL,
    created_at      NOT NULL
);

CREATE TABLE btc_dai_orders
(
    id INTEGER      NOT NULL PRIMARY KEY,
    order_id UNIQUE NOT NULL,
    quantity        NOT NULL,
    price           NOT NULL,
    open            NOT NULL,
    closed          NOT NULL,
    settling        NOT NULL,
    failed          NOT NULL,
    cancelled       NOT NULL,
    FOREIGN KEY (order_id) REFERENCES orders (id)
);

CREATE TABLE order_hbit_params
(
    id INTEGER        NOT NULL PRIMARY KEY,
    order_id UNIQUE   NOT NULL,
    network           NOT NULL,
    side              NOT NULL,
    our_final_address NOT NULL,
    expiry_offset     NOT NULL,
    FOREIGN KEY (order_id) REFERENCES orders (id)
);

CREATE TABLE order_herc20_params
(
    id INTEGER        NOT NULL PRIMARY KEY,
    order_id UNIQUE   NOT NULL,
    chain_id          NOT NULL,
    side              NOT NULL,
    our_htlc_identity NOT NULL,
    token_contract    NOT NULL,
    expiry_offset     NOT NULL,
    FOREIGN KEY (order_id) REFERENCES orders (id)
);

CREATE TABLE order_swaps
(
    id INTEGER     NOT NULL PRIMARY KEY,
    order_id       NOT NULL,
    swap_id UNIQUE NOT NULL,
    FOREIGN KEY (order_id) REFERENCES orders (id),
    FOREIGN KEY (swap_id) REFERENCES swaps (id)
)

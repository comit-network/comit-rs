-- Your SQL goes here

CREATE TABLE finalized_swaps
(
    id INTEGER                   NOT NULL PRIMARY KEY,
    local_swap_id                UNIQUE NOT NULL,
    shared_swap_id               UNIQUE NOT NULL,
    counterparty_alpha_identity  NOT NULL,
    counterparty_beta_identity   NOT NULL,
    secret_hash                  NOT NULL,
    finalized_at                 DATETIME DEFAULT CURRENT_TIMESTAMP
);

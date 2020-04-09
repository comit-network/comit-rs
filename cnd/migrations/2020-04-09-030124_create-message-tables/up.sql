-- Your SQL goes here

CREATE TABLE finalized_swaps
(
    id INTEGER          NOT NULL PRIMARY KEY,
    swap_id UNIQUE      NOT NULL,
    alpha_identity      NOT NULL,
    beta_identity,      NOT NULL,
    secret_hash         NOT NULL
)

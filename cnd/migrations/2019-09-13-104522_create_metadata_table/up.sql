-- Your SQL goes here

CREATE TABLE metadatas
(
    swap_id      VARCHAR(255) NOT NULL PRIMARY KEY,
    alpha_ledger VARCHAR(255) NOT NULL,
    beta_ledger  VARCHAR(255) NOT NULL,
    alpha_asset  VARCHAR(255) NOT NULL,
    beta_asset   VARCHAR(255) NOT NULL,
    role         VARCHAR(255) NOT NULL,
    counterparty VARCHAR(255) NOT NULL
)

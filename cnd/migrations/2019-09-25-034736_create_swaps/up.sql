CREATE TABLE swaps
(
        id               INTEGER NOT NULL PRIMARY KEY,
        swap_id          VARCHAR(255) UNIQUE NOT NULL,
        alpha_ledger     INTEGER NOT NULL,
        beta_ledger      INTEGER NOT NULL,
        alpha_asset      INTEGER NOT NULL,
        beta_asset       INTEGER NOT NULL,
        role             INTEGER NOT NULL
)

CREATE TABLE swaps
(
        id               INTEGER NOT NULL PRIMARY KEY,
        swap_id          VARCHAR(255) UNIQUE NOT NULL,
        alpha_ledger     VARCHAR(255) NOT NULL,
        beta_ledger      VARCHAR(255) NOT NULL,
        alpha_asset      VARCHAR(255) NOT NULL,
        beta_asset       VARCHAR(255) NOT NULL,
        role             VARCHAR(255) NOT NULL
)

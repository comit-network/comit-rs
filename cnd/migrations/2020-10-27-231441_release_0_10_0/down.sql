-- This file should undo anything in `up.sql`

DROP VIEW swap_contexts;
CREATE VIEW swap_contexts AS
SELECT local_swap_id as id,
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

CREATE TABLE halbits
(
    id                          INTEGER NOT NULL PRIMARY KEY,
    swap_id                     INTEGER NOT NULL,
    amount                      NOT NULL,
    network                     NOT NULL,
    chain                       NOT NULL,
    cltv_expiry                 NOT NULL,
    redeem_identity,
    refund_identity,
    side                        NOT NULL,
    FOREIGN KEY(swap_id)        REFERENCES swaps(id)
);

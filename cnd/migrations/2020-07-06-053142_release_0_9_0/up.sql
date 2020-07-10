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
FROM swaps

-- Your SQL goes here

-- Remove halbit from swap_contexts
DROP VIEW swap_contexts;
CREATE VIEW swap_contexts AS
SELECT local_swap_id as id,
       role,
       COALESCE(
               (SELECT 'herc20' from herc20s where herc20s.swap_id = swaps.id and herc20s.side = 'Alpha'),
               (SELECT 'hbit' from hbits where hbits.swap_id = swaps.id and hbits.side = 'Alpha')
           ) as alpha,
       COALESCE(
               (SELECT 'herc20' from herc20s where herc20s.swap_id = swaps.id and herc20s.side = 'Beta'),
               (SELECT 'hbit' from hbits where hbits.swap_id = swaps.id and hbits.side = 'Beta')
           ) as beta
FROM swaps;

-- Remove halbits table
DROP TABLE halbits;

DROP TABLE shared_swap_ids;

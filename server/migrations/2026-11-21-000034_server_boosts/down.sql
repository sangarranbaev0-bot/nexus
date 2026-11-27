ALTER TABLE servers DROP COLUMN IF EXISTS boost_count;
ALTER TABLE servers DROP COLUMN IF EXISTS boost_level;
DROP TABLE IF EXISTS server_boosts;

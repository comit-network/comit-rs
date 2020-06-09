# Migrations in cnd

This README offers guidelines on how to manage database migrations in cnd.
This document is motivated by the fact, that handling unnecessary migrations adds unnecessary complexity.
If you do multiple changes to the database that concern the same set of tables before the release of a new version, the migrations should be consolidated.
Only if there is a good reason (e.g. semantically different concern that justifies another migration) not to consolidate the migrations should there be multiple migrations during one release cycle.

If you want to add changes to the database please run `migrations.sh` to check which migration you should use.

The `migrations.sh` script will output the migration you are supposed to modify.
The script sets up a new migration if the last migration was included into a release.
If a new migration is added you will be asked to enter a name for the new migration.

Note, that if you add your changes to an existing migration you may want to consider changing the name for the migration if necessary.

Note that the script will automatically install Diesel CLI for you if it was not already installed.

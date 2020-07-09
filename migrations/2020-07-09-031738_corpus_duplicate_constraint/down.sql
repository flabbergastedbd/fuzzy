-- This file should undo anything in `up.sql`
ALTER TABLE corpora DROP IF EXISTS CONSTRAINT "corpora_checksum_label_key";
ALTER TABLE corpora ADD UNIQUE (checksum);

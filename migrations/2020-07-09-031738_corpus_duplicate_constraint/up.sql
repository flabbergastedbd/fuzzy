-- Your SQL goes here
ALTER TABLE corpora DROP CONSTRAINT "corpora_checksum_key";
ALTER TABLE corpora ADD UNIQUE (checksum, label);

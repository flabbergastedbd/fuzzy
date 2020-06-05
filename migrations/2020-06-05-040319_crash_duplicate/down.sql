-- This file should undo anything in `up.sql`
ALTER TABLE crashes
DROP COLUMN duplicate;

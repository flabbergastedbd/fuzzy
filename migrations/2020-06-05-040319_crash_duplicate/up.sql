-- Your SQL goes here
ALTER TABLE crashes
ADD COLUMN duplicate INTEGER REFERENCES crashes(id) ON DELETE SET NULL;

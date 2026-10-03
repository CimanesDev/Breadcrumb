BEGIN;
ALTER TABLE files ADD COLUMN browser_name TEXT;
ALTER TABLE files ADD COLUMN browser_profile TEXT;
ALTER TABLE files ADD COLUMN source_confidence TEXT;
COMMIT;

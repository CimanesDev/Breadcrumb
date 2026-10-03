BEGIN;
ALTER TABLE files ADD COLUMN file_key TEXT;
CREATE INDEX idx_files_key ON files(file_key);
COMMIT;

-- Add migration script here
BEGIN;
-- Backfill `status` for historical entries
UPDATE profile
SET status = 'confirmed'
WHERE status IS NULL;
-- Make `status` mandatory
ALTER TABLE profile
ALTER COLUMN status
SET NOT NULL;
COMMIT;
ALTER TABLE feeds
ADD COLUMN updated TEXT;
ALTER TABLE entries
ADD COLUMN updated TEXT;
ALTER TABLE feeds
  RENAME COLUMN updated_at TO modified_at;
ALTER TABLE entries
  RENAME COLUMN updated_at TO modified_at;

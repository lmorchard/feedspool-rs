ALTER TABLE feed_history
ADD COLUMN etag TEXT;

ALTER TABLE feed_history
ADD COLUMN last_modified TEXT;

ALTER TABLE feed_history
ADD COLUMN json TEXT;

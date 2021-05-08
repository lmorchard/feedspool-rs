CREATE TABLE tmp_feed_history (
  id TEXT PRIMARY KEY,
  feed_id TEXT,
  created_at TEXT,
  updated_at TEXT,
  src TEXT,
  status TEXT,
  etag TEXT,
  last_modified TEXT,
  json TEXT
);
INSERT INTO tmp_feed_history
SELECT id,
  feed_id,
  created_at,
  updated_at,
  src,
  status,
  etag,
  last_modified,
  json
FROM feed_history;
DROP TABLE IF EXISTS feed_history;
ALTER TABLE tmp_feed_history
  RENAME TO feed_history;

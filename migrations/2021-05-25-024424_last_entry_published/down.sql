CREATE TABLE tmp_feeds (
  id TEXT PRIMARY KEY,
  published TEXT,
  created_at TEXT,
  modified_at TEXT,
  url TEXT,
  title TEXT,
  subtitle TEXT,
  link TEXT,
  json TEXT,
  updated TEXT
);
INSERT INTO tmp_feeds
SELECT id,
  published,
  created_at,
  modified_at,
  url,
  title,
  subtitle,
  link,
  json,
  updated
FROM feeds;
DROP TABLE IF EXISTS feeds;
ALTER TABLE tmp_feeds
  RENAME TO feeds;
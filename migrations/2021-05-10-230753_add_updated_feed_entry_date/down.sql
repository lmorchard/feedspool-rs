ALTER TABLE feeds
  RENAME COLUMN modified_at TO updated_at;
ALTER TABLE entries
  RENAME COLUMN modified_at TO updated_at;
CREATE TABLE tmp_feeds (
  id TEXT PRIMARY KEY,
  published TEXT,
  created_at TEXT,
  updated_at TEXT,
  url TEXT,
  title TEXT,
  subtitle TEXT,
  link TEXT,
  json TEXT
);
INSERT INTO tmp_feeds
SELECT id,
  published,
  created_at,
  updated_at,
  url,
  title,
  subtitle,
  link,
  json
FROM feeds;
DROP TABLE IF EXISTS feeds;
ALTER TABLE tmp_feeds
  RENAME TO feeds;
CREATE TABLE tmp_entries (
  id TEXT PRIMARY KEY,
  feed_id TEXT,
  published TEXT,
  created_at TEXT,
  updated_at TEXT,
  defunct BOOLEAN,
  json TEXT,
  guid TEXT,
  title TEXT,
  link TEXT,
  summary TEXT,
  content TEXT
);
INSERT INTO tmp_entries
SELECT id,
  feed_id,
  published,
  created_at,
  updated_at,
  defunct,
  json,
  guid,
  title,
  link,
  summary,
  content
FROM entries;
DROP TABLE IF EXISTS entries;
ALTER TABLE tmp_entries
  RENAME TO entries;
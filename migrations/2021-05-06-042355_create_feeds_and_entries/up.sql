CREATE TABLE feeds (
  id TEXT PRIMARY KEY,
  created_at TEXT,
  updated_at TEXT,
  url TEXT,
  title TEXT,
  subtitle TEXT,
  link TEXT
);

CREATE TABLE feed_history (
  id TEXT PRIMARY KEY,
  feed_id TEXT,
  created_at TEXT,
  updated_at TEXT,
  src TEXT,
  status TEXT,
  status_text TEXT
);

CREATE TABLE entries (
  id TEXT PRIMARY KEY,
  feed_id TEXT,
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

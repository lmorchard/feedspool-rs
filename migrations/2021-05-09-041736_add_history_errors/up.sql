ALTER TABLE feed_history
ADD COLUMN is_error BOOLEAN;
ALTER TABLE feed_history
ADD COLUMN error_text TEXT;

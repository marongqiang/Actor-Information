-- v1.2.5: 刮削时间与错误记录
ALTER TABLE movies ADD COLUMN scrape_started_at INTEGER;
ALTER TABLE movies ADD COLUMN scrape_finished_at INTEGER;
ALTER TABLE movies ADD COLUMN scrape_error TEXT;

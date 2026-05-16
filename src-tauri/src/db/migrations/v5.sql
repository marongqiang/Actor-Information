-- v1.2.4: movies 增加刮削状态字段

ALTER TABLE movies ADD COLUMN scrape_status INTEGER NOT NULL DEFAULT 0;
-- 0=未刮削, 1=刮削中, 2=已刮削, 3=刮削失败

INSERT OR REPLACE INTO config (key, value, updated_at) VALUES ('db_version', '5', strftime('%s','now'));

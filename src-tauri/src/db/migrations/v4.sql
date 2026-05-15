-- v1.2.2: av_actors 增加 created_at，扩展刮削源列表

ALTER TABLE av_actors ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0;

-- 更新刮削源列表（增加更多源）
INSERT OR REPLACE INTO config (key, value, updated_at) VALUES
('scrape_sources', '["tmdb","imdb","douban","javbus","javdb","fanza","airav","xcity","mgstage","fc2","jav321","javlibrary","arzon"]', strftime('%s','now'));

INSERT OR REPLACE INTO config (key, value, updated_at) VALUES ('db_version', '4', strftime('%s','now'));

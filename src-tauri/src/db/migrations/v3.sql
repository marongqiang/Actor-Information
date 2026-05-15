-- v1.2.0: av_actors 增加本地文件夹相关字段

ALTER TABLE av_actors ADD COLUMN local_folder_name TEXT;
ALTER TABLE av_actors ADD COLUMN is_pending INTEGER NOT NULL DEFAULT 1;
ALTER TABLE av_actors ADD COLUMN source TEXT;

-- 新增配置项
INSERT OR IGNORE INTO config (key, value, use_system_credential, updated_at) VALUES
('local_actor_base_dir', 'D:\\Media Library\\Actor Information\\picture', 0, strftime('%s','now')),
('auto_create_actors_from_scrape', '1', 0, strftime('%s','now')),
('actor_pending_review', '1', 0, strftime('%s','now')),
('allow_app_rename_actor_folders', '0', 0, strftime('%s','now')),
('actor_merge_auto_merge_folders', '1', 0, strftime('%s','now')),
('actor_merge_file_naming_pattern', '{name}_{index}{ext}', 0, strftime('%s','now')),
('actor_merge_dry_run', '1', 0, strftime('%s','now'));

-- 更新 db_version
INSERT OR REPLACE INTO config (key, value, updated_at) VALUES ('db_version', '3', strftime('%s','now'));

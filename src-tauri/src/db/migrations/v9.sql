-- v1.2.5: 标签黑名单
ALTER TABLE genre_translations ADD COLUMN blacklisted INTEGER NOT NULL DEFAULT 0;

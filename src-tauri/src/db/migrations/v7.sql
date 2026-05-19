-- v1.2.5: 标签翻译库
CREATE TABLE IF NOT EXISTS genre_translations (
    ja_name TEXT PRIMARY KEY,
    cn_name TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

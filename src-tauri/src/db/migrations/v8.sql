-- v1.2.5: 刮削日志表
CREATE TABLE IF NOT EXISTS scrape_log (
    file_id TEXT PRIMARY KEY,
    status INTEGER NOT NULL DEFAULT 0,
    started_at INTEGER,
    finished_at INTEGER,
    error TEXT,
    FOREIGN KEY (file_id) REFERENCES movies(file_id) ON DELETE CASCADE
);

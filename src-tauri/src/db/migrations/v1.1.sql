-- v1.1 新增 tasks 和 actress_aliases 表

CREATE TABLE IF NOT EXISTS tasks (
    id                          TEXT PRIMARY KEY,
    type                        TEXT NOT NULL,
    target_ids                  TEXT NOT NULL,
    status                      TEXT NOT NULL,
    progress                    INTEGER NOT NULL DEFAULT 0,
    result                      TEXT,
    created_at                  INTEGER NOT NULL,
    updated_at                  INTEGER NOT NULL,
    error                       TEXT,
    checkpoint                  TEXT
);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_type ON tasks(type);

CREATE TABLE IF NOT EXISTS actress_aliases (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    actress_id                  INTEGER NOT NULL,
    alias_name                  TEXT NOT NULL UNIQUE,
    FOREIGN KEY (actress_id) REFERENCES av_actors(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_aliases_actress_id ON actress_aliases(actress_id);
CREATE INDEX IF NOT EXISTS idx_aliases_name ON actress_aliases(alias_name);

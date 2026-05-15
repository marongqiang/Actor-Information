-- v1.0 初始 schema

CREATE TABLE IF NOT EXISTS movies (
    file_id                     TEXT PRIMARY KEY,
    title                       TEXT NOT NULL,
    original_title              TEXT,
    year                        INTEGER,
    poster_url                  TEXT,
    poster_local                TEXT,
    backdrop_url                TEXT,
    overview                    TEXT,
    rating                      REAL,
    runtime                     INTEGER,
    director                    TEXT,
    genre                       TEXT,
    file_name                   TEXT NOT NULL,
    file_size                   INTEGER,
    created_at                  INTEGER NOT NULL,
    updated_at                  INTEGER NOT NULL,
    is_hidden                   INTEGER NOT NULL DEFAULT 0,
    last_play_url               TEXT,
    last_play_url_expire        INTEGER
);
CREATE INDEX IF NOT EXISTS idx_movies_year ON movies(year);
CREATE INDEX IF NOT EXISTS idx_movies_title ON movies(title);
CREATE INDEX IF NOT EXISTS idx_movies_updated ON movies(updated_at);

CREATE TABLE IF NOT EXISTS meta (
    key                         TEXT PRIMARY KEY,
    value                       TEXT NOT NULL,
    updated_at                  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS actors (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    name                        TEXT UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS movie_actors (
    movie_id                    TEXT NOT NULL,
    actor_id                    INTEGER NOT NULL,
    PRIMARY KEY (movie_id, actor_id),
    FOREIGN KEY (movie_id) REFERENCES movies(file_id) ON DELETE CASCADE,
    FOREIGN KEY (actor_id) REFERENCES actors(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS play_progress (
    file_id                     TEXT PRIMARY KEY,
    progress                    INTEGER NOT NULL DEFAULT 0,
    duration                    INTEGER NOT NULL DEFAULT 0,
    is_finished                 INTEGER NOT NULL DEFAULT 0,
    updated_at                  INTEGER NOT NULL,
    FOREIGN KEY (file_id) REFERENCES movies(file_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS groups (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    name                        TEXT NOT NULL,
    type                        TEXT NOT NULL DEFAULT 'manual',
    sort_order                  INTEGER NOT NULL DEFAULT 0,
    created_at                  INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_groups_sort ON groups(sort_order);

CREATE TABLE IF NOT EXISTS movie_groups (
    group_id                    INTEGER NOT NULL,
    movie_id                    TEXT NOT NULL,
    added_at                    INTEGER NOT NULL,
    PRIMARY KEY (group_id, movie_id),
    FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE,
    FOREIGN KEY (movie_id) REFERENCES movies(file_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS av_actors (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    name                        TEXT NOT NULL,
    avatar_url                  TEXT,
    avatar_local                TEXT,
    debut_year                  INTEGER,
    height                      INTEGER,
    birthdate                   TEXT,
    blood_type                  TEXT,
    bust                        INTEGER,
    waist                       INTEGER,
    hip                         INTEGER,
    cup                         TEXT,
    letter                      CHAR(1)
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_av_actors_name ON av_actors(name);
CREATE INDEX IF NOT EXISTS idx_av_actors_letter ON av_actors(letter);

CREATE TABLE IF NOT EXISTS actress_groups (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    name                        TEXT NOT NULL,
    sort_order                  INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS actress_group_members (
    group_id                    INTEGER NOT NULL,
    actress_id                  INTEGER NOT NULL,
    added_at                    INTEGER NOT NULL,
    PRIMARY KEY (group_id, actress_id),
    FOREIGN KEY (group_id) REFERENCES actress_groups(id) ON DELETE CASCADE,
    FOREIGN KEY (actress_id) REFERENCES av_actors(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS config (
    key                         TEXT PRIMARY KEY,
    value                       TEXT,
    encrypted_value             BLOB,
    use_system_credential       INTEGER NOT NULL DEFAULT 0,
    updated_at                  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS scrape_cache (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    query_key                   TEXT NOT NULL,
    source                      TEXT NOT NULL,
    result_json                 TEXT NOT NULL,
    expires_at                  INTEGER NOT NULL,
    UNIQUE(query_key, source)
);
CREATE INDEX IF NOT EXISTS idx_cache_expires ON scrape_cache(expires_at);

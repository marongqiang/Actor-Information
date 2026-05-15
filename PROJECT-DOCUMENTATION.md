
# 智能网盘影视库 (smart-media-vault) 完整规格说明书

**版本**：1.2.4  
**最后更新**：2026-05-15  
**维护者**：开发团队

---

## 目录

1. [项目概述](#1-项目概述)
2. [技术栈](#2-技术栈)
3. [项目目录结构](#3-项目目录结构)
4. [数据库设计](#4-数据库设计)
5. [TypeScript 接口定义](#5-typescript-接口定义)
6. [Tauri 命令接口](#6-tauri-命令接口)
7. [前端路由](#7-前端路由)
8. [侧边栏导航结构](#8-侧边栏导航结构)
9. [设置页 Tab 结构](#9-设置页-tab-结构)
10. [扫描工作流](#10-扫描工作流)
11. [刮削源配置](#11-刮削源配置)
12. [错误码定义](#12-错误码定义)
13. [应用数据存储路径](#13-应用数据存储路径)
14. [构建与打包](#14-构建与打包)
15. [版本记录](#15-版本记录)

---

## 1. 项目概述

**项目名称**：智能网盘影视库 (smart-media-vault)  
**版本**：1.2.4  
**类型**：Windows x64 桌面应用程序  
**核心功能**：管理 115 网盘中的影视文件，自动刮削元数据（海报、演员、简介等），提供海报墙浏览、演员库管理、播放进度追踪、分组管理等。

**关键设计目标**：
- 通过 Cookie 登录 115 网盘，浏览并扫描网盘目录中的视频文件
- 视频文件入库后在海报墙展示，支持分组管理和收藏
- 演员库支持本地文件夹扫描、别名管理、合并去重
- 登录状态和扫描设置持久化，重启自动恢复
- 扫描请求自动限流（每秒1次+批次冷却），防止被封 IP

---

## 2. 技术栈

| 类别 | 技术选型 | 版本/说明 |
|------|----------|------------|
| 核心框架 | Tauri | 2.0 |
| 前端框架 | Vue 3 | 3.4，Composition API |
| 状态管理 | Pinia | 2.1 |
| 路由 | Vue Router | 4.3，createWebHistory |
| UI 库 | Element Plus | 2.5，中文 locale，暗色主题 |
| 构建工具 | Vite | 5.4 |
| 语言 | TypeScript (前端) + Rust (后端) | TS 5.4，Rust 2021 |
| 数据库 | rusqlite (bundled SQLite) | 0.31，WAL 模式 |
| HTTP 客户端 | reqwest (Rust) | 0.12，异步 |
| HTML 解析 | scraper (Rust) | 0.19 |
| 日志 | log + fern (Rust) | 轮转输出到文件 |
| 图片处理 | image (Rust) | 0.25，转 WebP |
| 加密 | aes-gcm + rand | 本地加密存储 |
| 测试 | vitest + @vue/test-utils / cargo test | - |

---

## 3. 项目目录结构

```
smart-media-vault/
├── src-tauri/                     # Tauri 后端 (Rust)
│   ├── src/
│   │   ├── main.rs                # 入口
│   │   ├── lib.rs                 # 插件注册、命令注册、启动恢复
│   │   ├── commands/              # Tauri 命令处理器
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs            # 登录/登出/验证
│   │   │   ├── fs.rs              # 115文件浏览/扫描/播放链接
│   │   │   ├── scrape.rs          # 刮削任务管理
│   │   │   ├── library.rs         # 影片库 CRUD
│   │   │   ├── player.rs          # 播放进度
│   │   │   ├── config.rs          # 配置读写
│   │   │   ├── task.rs            # 任务管理
│   │   │   ├── groups.rs          # 影片分组 + 演员分组
│   │   │   └── actress_merge.rs   # 演员库操作
│   │   ├── services/              # 业务服务
│   │   │   ├── mod.rs
│   │   │   ├── pan115.rs          # 115 API 客户端 (category/files)
│   │   │   ├── scanner.rs         # 网盘扫描（递归+限流+去重）
│   │   │   ├── scrape_manager.rs  # 刮削调度
│   │   │   ├── actress_sync.rs    # 演员数据同步
│   │   │   ├── actress_folder_manager.rs  # 演员文件夹管理
│   │   │   ├── image_cache.rs     # 图片下载缓存
│   │   │   ├── task_manager.rs    # 任务持久化
│   │   │   ├── playback_refresher.rs  # 播放链接续期
│   │   │   └── secure_config.rs   # 加密配置存储
│   │   ├── db/
│   │   │   ├── mod.rs             # 连接初始化、迁移、种子数据
│   │   │   ├── queries.rs         # 预编译 SQL
│   │   │   └── migrations/
│   │   │       ├── v1.sql         # 初始 schema（11张表）
│   │   │       ├── v2.sql         # tasks + actress_aliases
│   │   │       ├── v3.sql         # av_actors 新增字段
│   │   │       └── v4.sql         # created_at 列 + 刮削源扩展
│   │   └── utils/
│   │       ├── mod.rs
│   │       ├── error.rs           # 统一错误类型
│   │       ├── filename_parser.rs # 文件名解析
│   │       ├── logger.rs          # 日志初始化（exe目录/logs/）
│   │       └── crypto.rs          # AES-256-GCM 加解密
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   └── icons/
├── src/                           # 前端 Vue 3 + TS
│   ├── main.ts
│   ├── App.vue                    # 主布局 + 自定义侧边栏导航 + 右键菜单
│   ├── router/index.ts            # 路由表（9条路由）
│   ├── stores/
│   │   ├── library.ts             # 影片库状态
│   │   ├── actress.ts             # 演员库状态
│   │   ├── scan.ts                # 扫描状态
│   │   └── task.ts                # 任务状态
│   ├── views/
│   │   ├── PosterWall.vue         # 海报墙（网格 + 右键二级菜单）
│   │   ├── Detail.vue             # 影片详情（演员名可点击跳转）
│   │   ├── Player.vue             # 在线播放器
│   │   ├── Favorites.vue          # 收藏影片
│   │   ├── Actress.vue            # 演员库（圆头像 + 右键菜单）
│   │   ├── ActressDetail.vue      # 演员详情（信息/别名/关联影片）
│   │   ├── ActressTable.vue       # 演员表格（可编辑/排序/勾选）
│   │   ├── Scan.vue               # 扫描管理（三步流程）
│   │   └── Settings.vue           # 设置（5个Tab）
│   ├── types/index.ts             # 全局 TS 接口
│   ├── composables/
│   │   └── usePlayerRefresh.ts    # 播放链接续期
│   └── vite-env.d.ts
├── index.html
├── package.json
├── vite.config.ts
├── tsconfig.json
├── tsconfig.node.json
├── README.md
├── PROJECT-DOCUMENTATION.md
└── .gitignore
```

---

## 4. 数据库设计

数据库文件：`<exe目录>/../data/vault.db`（首次启动自动创建）  
启动 PRAGMA：
```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA cache_size = -20000;
PRAGMA foreign_keys = ON;
```

迁移版本通过 `schema_version` 表管理，当前最新为 v4。

### 4.1 核心表

#### `movies` — 影片主表
```sql
CREATE TABLE movies (
    file_id                     TEXT PRIMARY KEY,      -- 115文件ID (fid)
    title                       TEXT NOT NULL,         -- 主标题
    original_title              TEXT,                  -- 原片名
    year                        INTEGER,               -- 年份
    poster_url                  TEXT,                  -- 原始海报URL
    poster_local                TEXT,                  -- 本地缓存路径
    backdrop_url                TEXT,                  -- 背景图URL
    overview                    TEXT,                  -- 简介
    rating                      REAL,                  -- 评分0-10
    runtime                     INTEGER,               -- 分钟
    director                    TEXT,                  -- 导演
    genre                       TEXT,                  -- JSON数组字符串
    file_name                   TEXT NOT NULL,         -- 原始文件名
    file_size                   INTEGER,               -- 字节
    created_at                  INTEGER NOT NULL,      -- 入库时间戳
    updated_at                  INTEGER NOT NULL,      -- 更新时间戳
    is_hidden                   INTEGER NOT NULL DEFAULT 0,
    last_play_url               TEXT,                  -- 播放链接缓存
    last_play_url_expire        INTEGER                -- 链接过期时间戳
);
CREATE INDEX idx_movies_year ON movies(year);
CREATE INDEX idx_movies_title ON movies(title);
CREATE INDEX idx_movies_updated ON movies(updated_at);
```

#### `actors` — 基础演员表
```sql
CREATE TABLE actors (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    name                        TEXT UNIQUE NOT NULL
);
```

#### `movie_actors` — 影片-演员关联
```sql
CREATE TABLE movie_actors (
    movie_id                    TEXT NOT NULL,
    actor_id                    INTEGER NOT NULL,
    PRIMARY KEY (movie_id, actor_id),
    FOREIGN KEY (movie_id) REFERENCES movies(file_id) ON DELETE CASCADE,
    FOREIGN KEY (actor_id) REFERENCES actors(id) ON DELETE CASCADE
);
```

#### `play_progress` — 播放进度
```sql
CREATE TABLE play_progress (
    file_id                     TEXT PRIMARY KEY,
    progress                    INTEGER NOT NULL DEFAULT 0,
    duration                    INTEGER NOT NULL DEFAULT 0,
    is_finished                 INTEGER NOT NULL DEFAULT 0,
    updated_at                  INTEGER NOT NULL,
    FOREIGN KEY (file_id) REFERENCES movies(file_id) ON DELETE CASCADE
);
```

#### `groups` — 影片分组
```sql
CREATE TABLE groups (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    name                        TEXT NOT NULL,
    type                        TEXT NOT NULL DEFAULT 'manual',
                                -- 'manual'=海报墙分组, 'favorite'=收藏分组, 'genre', 'collection'
    sort_order                  INTEGER NOT NULL DEFAULT 0,
    created_at                  INTEGER NOT NULL
);
CREATE INDEX idx_groups_sort ON groups(sort_order);
```

#### `movie_groups` — 影片-分组关联
```sql
CREATE TABLE movie_groups (
    group_id                    INTEGER NOT NULL,
    movie_id                    TEXT NOT NULL,
    added_at                    INTEGER NOT NULL,
    PRIMARY KEY (group_id, movie_id),
    FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE,
    FOREIGN KEY (movie_id) REFERENCES movies(file_id) ON DELETE CASCADE
);
```

#### `av_actors` — 演员库（含本地文件夹信息）
```sql
CREATE TABLE av_actors (
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
    letter                      CHAR(1),               -- 拼音首字母
    local_folder_name           TEXT,                  -- 本地文件夹完整路径
    is_pending                  INTEGER NOT NULL DEFAULT 0, -- 0=已确认 1=待审核
    source                      TEXT,                  -- 'local_folder'|'scrape'|'manual'
    created_at                  INTEGER NOT NULL
);
CREATE UNIQUE INDEX idx_av_actors_name ON av_actors(name);
CREATE INDEX idx_av_actors_letter ON av_actors(letter);
```

#### `actress_groups` — 演员分组
```sql
CREATE TABLE actress_groups (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    name                        TEXT NOT NULL,
    sort_order                  INTEGER NOT NULL DEFAULT 0
);
```

#### `actress_group_members` — 演员-分组关联
```sql
CREATE TABLE actress_group_members (
    group_id                    INTEGER NOT NULL,
    actress_id                  INTEGER NOT NULL,
    added_at                    INTEGER NOT NULL,
    PRIMARY KEY (group_id, actress_id),
    FOREIGN KEY (group_id) REFERENCES actress_groups(id) ON DELETE CASCADE,
    FOREIGN KEY (actress_id) REFERENCES av_actors(id) ON DELETE CASCADE
);
```

#### `actress_aliases` — 演员别名
```sql
CREATE TABLE actress_aliases (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    actress_id                  INTEGER NOT NULL,
    alias_name                  TEXT NOT NULL UNIQUE,
    FOREIGN KEY (actress_id) REFERENCES av_actors(id) ON DELETE CASCADE
);
CREATE INDEX idx_aliases_actress_id ON actress_aliases(actress_id);
CREATE INDEX idx_aliases_name ON actress_aliases(alias_name);
```

#### `config` — 配置表（支持加密）
```sql
CREATE TABLE config (
    key                         TEXT PRIMARY KEY,
    value                       TEXT,                  -- 明文
    encrypted_value             BLOB,                  -- AES加密值
    use_system_credential       INTEGER NOT NULL DEFAULT 0,
    updated_at                  INTEGER NOT NULL
);
```

#### `scrape_cache` — 刮削缓存
```sql
CREATE TABLE scrape_cache (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    query_key                   TEXT NOT NULL,
    source                      TEXT NOT NULL,
    result_json                 TEXT NOT NULL,
    expires_at                  INTEGER NOT NULL,
    UNIQUE(query_key, source)
);
CREATE INDEX idx_cache_expires ON scrape_cache(expires_at);
```

#### `tasks` — 长任务持久化
```sql
CREATE TABLE tasks (
    id                          TEXT PRIMARY KEY,
    type                        TEXT NOT NULL,         -- 'scan'|'scrape'
    target_ids                  TEXT NOT NULL,
    status                      TEXT NOT NULL,
    progress                    INTEGER NOT NULL DEFAULT 0,
    result                      TEXT,
    created_at                  INTEGER NOT NULL,
    updated_at                  INTEGER NOT NULL,
    error                       TEXT,
    checkpoint                  TEXT
);
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_type ON tasks(type);
```

#### `meta` — 扩展元数据
```sql
CREATE TABLE meta (
    key                         TEXT PRIMARY KEY,
    value                       TEXT NOT NULL,
    updated_at                  INTEGER NOT NULL
);
```

### 4.2 默认配置

```sql
INSERT OR IGNORE INTO config (key, value) VALUES
('db_version', '4'),
('scan_depth', '5'),
('theme', 'dark'),
('poster_size', 'medium'),
('font_size', '14'),
('privacy_title', '智能网盘影视库'),
('external_player', ''),
('scrape_sources', '["tmdb","imdb","douban","javbus","javdb","fanza","airav","xcity","mgstage","fc2","jav321","javlibrary","arzon"]'),
('video_extensions', '["mp4","mkv","avi","mov","rmvb","flv","wmv","ts","iso","m2ts"]'),
('playback_refresh_interval', '240'),
('auto_start', 'false'),
('local_actor_base_dir', 'D:\\Media Library\\Actor Information\\picture'),
('auto_create_actors_from_scrape', '1'),
('actor_pending_review', '1'),
('allow_app_rename_actor_folders', '0'),
('actor_merge_auto_merge_folders', '1'),
('actor_merge_file_naming_pattern', '{name}_{index}{ext}'),
('actor_merge_dry_run', '1');
```

---

## 5. TypeScript 接口定义

```typescript
// 文件项（来自115列表）
interface FileItem {
  cid: string;          // 目录ID
  name: string;         // 文件名
  is_dir: boolean;      // 是否目录
  size: number;         // 字节
  update_time: number;  // 修改时间戳
  file_id?: string;     // 文件ID（仅文件有）
}

// 影片项（海报墙用）
interface MovieItem {
  file_id: string;
  title: string;
  year: number | null;
  poster_local: string | null;
  rating: number | null;
  genre: string[];
  is_hidden: boolean;
  progress?: number;
  duration?: number;
}

// 影片详情
interface MovieDetail extends MovieItem {
  original_title: string | null;
  backdrop_local: string | null;
  overview: string | null;
  runtime: number | null;
  director: string | null;
  actors: string[];
  file_name: string;
  file_size: number;
  created_at: number;
  updated_at: number;
  groups: GroupItem[];
}

// 影片分组
interface GroupItem {
  id: number;
  name: string;
  type: 'manual' | 'favorite' | 'genre' | 'collection';
  sort_order: number;
  movie_count?: number;
}

// 演员项
interface ActressItem {
  id: number;
  name: string;
  avatar_local: string | null;
  debut_year: number | null;
  height: number | null;
  bust: number | null;
  waist: number | null;
  hip: number | null;
  cup: string | null;
  letter: string;
  movie_count?: number;
  local_folder_name?: string | null;
  is_pending: boolean;
  source?: string | null;       // 'local_folder' | 'scrape' | 'manual'
  _aliases?: string[];          // 运行时注入
}

// 演员分组
interface ActressGroupItem {
  id: number;
  name: string;
  sort_order: number;
  member_count?: number;
}

// 筛选参数
interface FilterParams {
  keyword?: string;
  year?: number;
  genre?: string;
  group_id?: number;
  is_hidden?: boolean;
  is_finished?: boolean;
}

// 任务
interface Task {
  id: string;
  type: 'scan' | 'scrape';
  status: 'pending' | 'running' | 'paused' | 'completed' | 'failed';
  progress: number;
  result?: any;
  error?: string;
  created_at: number;
  updated_at: number;
}

// 刮削结果
interface ScrapeResult {
  source: string;
  title: string;
  year?: number;
  poster_url?: string;
  backdrop_url?: string;
  overview?: string;
  rating?: number;
  runtime?: number;
  director?: string;
  genre?: string[];
  actors?: string[];
  score: number;
}

// 合并选项
interface MergeOptions {
  mergeFolders: boolean;
  conflictPolicy?: 'rename' | 'skip' | 'overwrite';
  dryRun?: boolean;
}

// 合并结果
interface MergeResult {
  success: boolean;
  movedFiles: string[];
  conflicts: string[];
  renamedFiles?: Array<{ from: string; to: string }>;
  error?: string;
}

// 重复演员对
interface DuplicatePair {
  id1: number;
  id2: number;
  similarity: number;
}
```

---

## 6. Tauri 命令接口

所有命令通过 `@tauri-apps/api/core` 的 `invoke` 调用，返回 `Result<T, CommandError>`。

### 6.1 认证 (auth)

```
login_qrcode()             → { qrcode_url: string, uid: string }
login_status(uid)          → { status: 'waiting'|'scanned'|'authorized'|'expired', cookie?: string }
login_cookie(cookie)       → void   (存储扫码获得的cookie)
login_cookie_direct(cookie)→ void   (验证并存储手动输入的cookie)
logout()                   → void
check_token()              → bool
```

Cookie 登录后自动加密存储到 config 表，下次启动自动恢复。

### 6.2 文件系统与扫描 (fs)

```
list_root()                → { cid: string, name: string }[]
get_files(cid, page, pageSize) → { files: FileItem[], total: number }
scan_directory(cid, depth, mode) → { total, new, updated, deleted }
get_play_url(fileId)       → { url: string, expire_at: number }
refresh_play_url(fileId)   → { url: string, expire_at: number }
export_list()              → string  (导出路径)
```

- `mode`: `'incremental'` | `'full'`
- `depth`: 1-10，子目录递归深度
- 扫描使用 `category/files` 端点，自动限流（1次/秒 + 每10次冷却3秒）
- 已扫描目录去重（HashSet），不会重复请求

### 6.3 刮削 (scrape)

```
start_scrape(fileIds)       → taskId
pause_scrape(taskId)        → void
resume_scrape(taskId)       → void
manual_scrape(fileId, keyword) → ScrapeResult[]
select_scrape_result(fileId, resultIdx) → void
test_source(url)            → { status, time_ms }
```

### 6.4 任务管理 (task)

```
get_pending_tasks()         → Task[]
resume_task(taskId)         → void
cancel_task(taskId)         → void
```

### 6.5 影视库 (library)

```
get_movies(filters, sort, page)  → { movies: MovieItem[], total: number }
get_movie_detail(fileId)         → MovieDetail | null
batch_action(fileIds, action)    → void  ('mark_watched'|'mark_unwatched'|'rescrape')
hide_movies(fileIds)             → void
unhide_movies(fileIds)           → void
```

### 6.6 影片分组 (groups)

```
get_groups(category?)       → GroupItem[]  (category: 'manual'|'favorite'|'all')
create_group(name, groupType?) → GroupItem
rename_group(groupId, newName)  → void
delete_group(groupId)           → void
reorder_groups(orderedIds)      → void
add_movies_to_group(groupId, fileIds) → void
remove_movie_from_group(groupId, fileId) → void
```

- 重名检查按 `(name, type)` 范围
- `type`: `'manual'`=海报墙分组, `'favorite'`=收藏分组

### 6.7 演员库 (actress)

```
sync_actress_data()                     → void  (从本地文件夹+影片演员表同步)
get_actresses_by_letter()               → { letter, actresses[] }[]
get_actresses_paginated(page, pageSize, search?, sortField?, sortOrder?, includePending?, groupId?)
    → { list: ActressItem[], total: number }
find_actress(name)                      → ActressItem | null
update_actress(id, data)                → void  (支持 is_pending/name/debut_year/height/bust/waist/hip/cup/source)
delete_actresses(ids)                   → number  (删除数量)
delete_all_actresses()                  → number  (调试用，清空全部)
get_actress_aliases(actressId)          → string[]
add_actress_alias(actressId, alias)     → void

// 演员文件夹相关
scan_local_actress_folder(folderPath?)  → { added, total }
refresh_actress_avatar(actressId)       → void
confirm_actor(actorId, accepted)        → void  (待审核→确认/拒绝)
update_actor_local_folder(actorId, folderPath?) → void
rename_actor_and_folder(actorId, newName, renameFolder) → { success, error? }
merge_actresses(sourceId, targetId, options) → MergeResult
detect_duplicate_actresses(threshold?)  → DuplicatePair[]
get_actress_local_folder(actressId)     → string | null
sync_actress_with_local_folder(actressId) → void
```

- 排序字段：name, debut_year, height, bust, waist, hip, cup, movie_count, id, is_pending, source, letter
- `includePending`: `undefined`=全部, `true`=仅待审核, `false`=仅已确认
- 本地文件夹扫描时 `is_pending` 默认 0（已确认）

### 6.8 演员分组 (actress groups)

```
get_actress_groups()                → ActressGroupItem[]
create_actress_group(name)          → ActressGroupItem
rename_actress_group(groupId, newName) → void
delete_actress_group(groupId)       → void
add_actresses_to_group(groupId, actressIds) → void
remove_actress_from_group(groupId, actressId) → void
```

### 6.9 播放进度 (player)

```
get_progress(fileId)                → { progress, duration, is_finished }
save_progress(fileId, progress, duration) → void
end_playback(fileId)                → void
```

### 6.10 配置 (config)

```
get_config(key)                     → string | null
set_config(key, value)              → void
get_all_config()                    → Record<string, string>
set_secure_config(key, value)       → void  (AES-256-GCM加密存储)
get_secure_config(key)              → string | null
clear_secure_config(key)            → void
```

---

## 7. 前端路由

| 路径 | 名称 | 组件 | 说明 |
|------|------|------|------|
| `/` | home | PosterWall.vue | 海报墙，支持分组筛选(`?group_id=N`) |
| `/detail/:fileId` | detail | Detail.vue | 影片详情，演员名可点击跳转 |
| `/player/:fileId` | player | Player.vue | 在线播放（5秒自动保存进度） |
| `/favorites` | favorites | Favorites.vue | 收藏影片，支持分组筛选 |
| `/actress` | actress | Actress.vue | 演员库（圆头像网格），头像可点击跳详情 |
| `/actress/:id` | actress-detail | ActressDetail.vue | 演员详情（信息/别名/关联影片） |
| `/actress-table` | actress-table | ActressTable.vue | 演员表格（可编辑/排序/勾选/分组筛选） |
| `/scan` | scan | Scan.vue | 扫描管理（三步流程：浏览→设置→执行） |
| `/settings` | settings | Settings.vue | 设置（5个Tab页签） |

路由守卫：不强制登录，115 相关 API 调用时提示未登录。

---

## 8. 侧边栏导航结构

```
智能网盘影视库
├── 🖼 海报墙 ▸               ← 点击展开/折叠，右键 → 新增分组
│   ├── 分组1 (12)           ← 右键 → 重命名/删除
│   └── 分组2 (5)
├── ⭐ 收藏影片 ▸             ← 同上
│   └── 收藏分组1 (3)
├── 👤 演员库 ▸               ← 同上
│   └── 演员分组1 (8)
├── 📋 演员表格               ← 独立一级入口
├── 📁 扫描管理
└── ⚙ 设置
```

- 分组按类型隔离：海报墙(type=manual)、收藏(type=favorite)、演员(actress_groups表)
- 允许跨类型同名分组
- 所有入口点击即可导航+折叠切换

---

## 9. 设置页 Tab 结构

| Tab | 内容 |
|-----|------|
| **刮削源** | 13个刮削源复选框(tmdb/imdb/douban/javbus/javdb/fanza/airav/xcity/mgstage/fc2/jav321/javlibrary/arzon)、视频扩展名(逗号分隔)、TMDB API Key |
| **网络代理** | 代理开关/类型/地址/端口 |
| **缓存** | 图片缓存大小、数据管理（导出/清缓存）、日志路径 |
| **界面** | 主题(暗色/亮色)、海报尺寸、字体大小、窗口标题、外部播放器路径、播放链接续期间隔、开机自启 |
| **常规** | 115登录(扫码/Cookie双Tab)、演员文件夹管理(路径/审核/重命名/合并配置) |

---

## 10. 扫描工作流

```
步骤一：浏览网盘目录
  → 加载根目录(cid=0, category/files)
  → 点击📁进入子目录（面包屑导航）
  → 勾选要扫描的目录

步骤二：扫描设置
  → 已选目录标签展示
  → 扫描模式（增量/全量）
  → 子目录深度（1-10）

步骤三：执行扫描
  → 弹窗显示进度：大号数字"已扫描到影片数量: XXX"
  → 后端递归遍历 → 限流 → 去重 → 过滤视频扩展名 → 入库
  → 完成显示新增/更新统计
  → "关闭"或"去海报墙查看"
```

扫描完成后自动保存 CID/深度/模式到配置，下次启动恢复。

---

## 11. 刮削源配置

当前支持的 13 个刮削源：

| 源 | 类型 | 说明 |
|----|------|------|
| tmdb | 通用 | The Movie Database |
| imdb | 通用 | Internet Movie Database |
| douban | 通用 | 豆瓣电影 |
| javbus | JAV | JavBus |
| javdb | JAV | JavDB |
| fanza | JAV | Fanza（官方） |
| airav | JAV | Airav |
| xcity | JAV | XCITY |
| mgstage | JAV | MGStage |
| fc2 | JAV | FC2 |
| jav321 | JAV | Jav321 |
| javlibrary | JAV | JavLibrary |
| arzon | JAV | Arzon |

可在设置页动态增删，也可编辑 `scrape_sources` 配置项。

视频扩展名：默认 `mp4,mkv,avi,mov,rmvb,flv,wmv,ts,iso,m2ts`，不区分大小写，可在设置页修改。

---

## 12. 错误码定义

| code | 含义 | 说明 |
|------|------|------|
| 1000 | 数据库错误 | 查询失败、迁移错误、约束冲突 |
| 2000 | 网络错误 | 115 API 请求失败、超时、DNS 错误 |
| 2100 | 未授权 | 115 Cookie 失效或未登录 |
| 2200 | 限流 | HTTP 429 响应 |
| 3000 | 无效输入 | 参数缺失、格式错误、超出范围 |
| 3100 | 资源未找到 | 文件、演员、分组不存在 |
| 4000 | 内部错误 | panic 或其他未知错误 |
| 4100 | 刮削失败 | 所有刮削源均无结果或全部超时 |

---

## 13. 应用数据存储路径

| 路径 | 说明 |
|------|------|
| `<exe目录>/logs/app.log` | 滚动日志文件 |
| `<exe目录>/../data/vault.db` | SQLite 数据库（WAL 模式） |
| `%APPDATA%/smart-media-vault/images/` | 海报/头像缓存（WebP 格式） |
| `%APPDATA%/smart-media-vault/exports/` | 影片列表导出目录 |

日志级别：生产环境 INFO，开发环境 DEBUG。

---

## 14. 构建与打包

### 开发

```bash
npm install
npx tauri dev
```

### 生产构建

```bash
npx tauri build
```

产物：
- 安装包：`src-tauri/target/release/bundle/nsis/智能网盘影视库_1.2.4_x64-setup.exe`
- 便携版：`smart-media-vault-v1.2.4-portable.zip`

---

## 15. 版本记录

| 版本 | 日期 | 内容 |
|------|------|------|
| v1.2.4 | 2026-05-15 | 演员详情页+头像/演员名点击跳转；右键菜单统一二级展开；排序字段扩展；Cookie/扫描设置持久化；扫描去重+限流；fid字符串误判目录修复；视频扩展名可配置 |
| v1.2.3 | 2026-05-15 | 分组类型隔离+右键子项操作；演员表独立入口/可编辑单元格/排序；演员库圆头像+右键分组；扫描三步流程+弹窗进度；115 category/files API集成 |
| v1.2.2 | 2026-05-15 | 侧边栏重构+收藏入口；演员表列宽/排序/勾选；刮削源扩至13个；设置Tab布局；二维码CSP修复 |
| v1.2.0 | 2026-05-15 | 演员本地文件夹管理(扫描/合并/重复检测/待审核)；Cookie直接登录 |
| v1.1.0 | 2026-05-15 | 初始版本：海报墙/详情/播放/演员库/演员表格/扫描/设置；60+命令；115扫码登录；13张核心表 |

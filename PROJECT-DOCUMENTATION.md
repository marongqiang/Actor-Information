

# 智能网盘影视库 (smart-media-vault) 完整规格说明书

**版本**：1.2.3  
**最后更新**：2026-05-15  
**维护者**：开发团队  

---

## 目录

1. [项目概述](#1-项目概述)
2. [技术栈](#2-技术栈)
3. [项目目录结构](#3-项目目录结构)
4. [数据库设计（完整 DDL）](#4-数据库设计完整-ddl)
5. [TypeScript 接口定义](#5-typescript-接口定义)
6. [Tauri 命令接口](#6-tauri-命令接口)
7. [前端路由与导航守卫](#7-前端路由与导航守卫)
8. [系统托盘菜单](#8-系统托盘菜单)
9. [Tauri 事件列表](#9-tauri-事件列表)
10. [外部播放器调用协议](#10-外部播放器调用协议)
11. [日志规范](#11-日志规范)
12. [错误码定义](#12-错误码定义)
13. [应用数据存储路径](#13-应用数据存储路径)
14. [构建与打包](#14-构建与打包)
15. [测试策略](#15-测试策略)
16. [安全与隐私](#16-安全与隐私)
17. [性能指标要求](#17-性能指标要求)
18. [未来扩展性预留](#18-未来扩展性预留)
19. [版本与模块变更标记](#19-版本与模块变更标记)
20. [代码清理与维护规范](#20-代码清理与维护规范)

---

## 1. 项目概述

**项目名称**：智能网盘影视库 (smart-media-vault)  
**版本**：1.2.0  
**类型**：Windows x64 桌面应用程序  
**核心功能**：管理 115 网盘中的影视文件，自动刮削元数据（海报、演员、简介等），提供海报墙浏览、演员库管理、播放进度追踪、分组管理等。

**关键设计目标**：
- 完全离线可用（元数据本地存储，海报本地缓存）
- 支持增量扫描与断点续刮
- 播放链接自动续期（解决 115 链接 1 小时过期问题）
- 演员别名合并去重，且支持**本地文件夹同步与合并**
- 敏感配置安全存储（支持 Windows 凭据管理器）
- 高并发刮削限流，避免被封 IP
- 单元测试与集成测试覆盖核心逻辑

---

## 2. 技术栈

| 类别 | 技术选型 | 版本/说明 |
|------|----------|------------|
| 核心框架 | Tauri | 2.0（系统 WebView2） |
| 前端框架 | Vue 3 | 3.4，Composition API |
| 状态管理 | Pinia | 2.1 |
| 路由 | Vue Router | 4.3，createWebHistory |
| UI 库 | Element Plus | 2.5，中文 locale |
| 构建工具 | Vite | 5.4 |
| 语言 | TypeScript (前端) + Rust (后端) | TS 5.4，Rust 2021 |
| 数据库 | better-sqlite3 | 11.0，同步 API，WAL 模式 |
| HTTP 客户端 | reqwest (Rust) | 异步，带重试中间件 |
| HTML 解析 | scraper (Rust) | 替代 cheerio |
| 日志 | log + fern (Rust) | 输出到文件，轮转 |
| 图片处理 | image (Rust) | 转 WebP，缩放 |
| 加密 | aes-gcm + rand | 用于本地加密存储 |
| 系统凭据 | credential-manager | Windows 专用 |
| 文件监控 | notify (Rust) | 可选，用于监听演员文件夹变化 |
| 测试 | vitest + @vue/test-utils (前端) / cargo test (Rust) / tauri-driver (E2E) | - |

---

## 3. 项目目录结构

```
smart-media-vault/
├── src-tauri/                     # Tauri 后端 (Rust)
│   ├── src/
│   │   ├── main.rs                # 入口，插件注册，事件监听
│   │   ├── commands/              # Tauri 命令处理器
│   │   │   ├── auth.rs
│   │   │   ├── fs.rs
│   │   │   ├── scrape.rs
│   │   │   ├── library.rs
│   │   │   ├── player.rs
│   │   │   ├── config.rs
│   │   │   ├── task.rs
│   │   │   └── actress_merge.rs
│   │   ├── services/              # 业务服务
│   │   │   ├── pan115.rs
│   │   │   ├── scanner.rs
│   │   │   ├── scrape_manager.rs
│   │   │   ├── actress_sync.rs
│   │   │   ├── actress_folder_manager.rs
│   │   │   ├── image_cache.rs
│   │   │   ├── task_manager.rs
│   │   │   ├── playback_refresher.rs
│   │   │   └── secure_config.rs
│   │   ├── db/
│   │   │   ├── mod.rs
│   │   │   ├── migrations/
│   │   │   │   ├── v1.sql
│   │   │   │   ├── v2.sql
│   │   │   │   └── v3.sql
│   │   │   └── queries.rs
│   │   └── utils/
│   │       ├── filename_parser.rs
│   │       ├── logger.rs
│   │       ├── error.rs
│   │       └── crypto.rs
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── icons/
├── src/                           # 前端 Vue 3 + TS
│   ├── main.ts
│   ├── App.vue
│   ├── router/index.ts
│   ├── stores/
│   │   ├── library.ts
│   │   ├── actress.ts
│   │   ├── scan.ts
│   │   └── task.ts
│   ├── views/
│   │   ├── PosterWall.vue
│   │   ├── Detail.vue
│   │   ├── Player.vue
│   │   ├── Actress.vue
│   │   ├── ActressTable.vue
│   │   ├── Scan.vue
│   │   └── Settings.vue
│   ├── components/
│   ├── types/
│   ├── composables/
│   └── assets/
├── tests/
│   ├── unit/
│   ├── e2e/
│   └── mocks/
├── index.html
├── package.json
├── vite.config.ts
├── tsconfig.json
├── .env.example
└── .gitignore
```

---

## 4. 数据库设计（完整 DDL）

数据库文件：`%APPDATA%\smart-media-vault\vault.db`  
启动 PRAGMA：
```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA cache_size = -20000;
PRAGMA foreign_keys = ON;
```

### 4.1 核心表（v1.0）

#### `movies` – 影片主表
```sql
CREATE TABLE movies (
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
CREATE INDEX idx_movies_year ON movies(year);
CREATE INDEX idx_movies_title ON movies(title);
CREATE INDEX idx_movies_updated ON movies(updated_at);
```

#### `meta` – 备用扩展元数据
```sql
CREATE TABLE meta (
    key                         TEXT PRIMARY KEY,
    value                       TEXT NOT NULL,
    updated_at                  INTEGER NOT NULL
);
```

#### `actors` – 基础演员表（用于影片关联）
```sql
CREATE TABLE actors (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    name                        TEXT UNIQUE NOT NULL
);
```

#### `movie_actors` – 影片与演员关联
```sql
CREATE TABLE movie_actors (
    movie_id                    TEXT NOT NULL,
    actor_id                    INTEGER NOT NULL,
    PRIMARY KEY (movie_id, actor_id),
    FOREIGN KEY (movie_id) REFERENCES movies(file_id) ON DELETE CASCADE,
    FOREIGN KEY (actor_id) REFERENCES actors(id) ON DELETE CASCADE
);
```

#### `play_progress` – 播放进度
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

#### `groups` – 用户自定义分组
```sql
CREATE TABLE groups (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    name                        TEXT NOT NULL,
    type                        TEXT NOT NULL DEFAULT 'manual',
    sort_order                  INTEGER NOT NULL DEFAULT 0,
    created_at                  INTEGER NOT NULL
);
CREATE INDEX idx_groups_sort ON groups(sort_order);
```

#### `movie_groups` – 影片与分组关联
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

#### `av_actors` – 演员库（增强信息）
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
    letter                      CHAR(1),
    local_folder_name           TEXT,
    is_pending                  INTEGER NOT NULL DEFAULT 1,
    source                      TEXT
);
CREATE UNIQUE INDEX idx_av_actors_name ON av_actors(name);
CREATE INDEX idx_av_actors_letter ON av_actors(letter);
CREATE INDEX idx_av_actors_pending ON av_actors(is_pending);
```

#### `actress_groups` – 演员分组表
```sql
CREATE TABLE actress_groups (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    name                        TEXT NOT NULL,
    sort_order                  INTEGER NOT NULL DEFAULT 0
);
```

#### `actress_group_members` – 演员与分组关联
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

#### `config` – 配置表（支持加密）
```sql
CREATE TABLE config (
    key                         TEXT PRIMARY KEY,
    value                       TEXT,
    encrypted_value             BLOB,
    use_system_credential       INTEGER NOT NULL DEFAULT 0,
    updated_at                  INTEGER NOT NULL
);
```

#### `scrape_cache` – 刮削结果缓存
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

### 4.2 表（v1.1）

#### `tasks` – 长任务持久化
```sql
CREATE TABLE tasks (
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
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_type ON tasks(type);
```

#### `actress_aliases` – 演员别名映射
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

### 4.3 迁移脚本 v3.sql（1.2.0）

```sql
-- 为 av_actors 表增加字段
ALTER TABLE av_actors ADD COLUMN local_folder_name TEXT;
ALTER TABLE av_actors ADD COLUMN is_pending INTEGER NOT NULL DEFAULT 1;
ALTER TABLE av_actors ADD COLUMN source TEXT;

-- 更新配置中的 db_version
INSERT OR REPLACE INTO config (key, value, updated_at) VALUES ('db_version', '3', strftime('%s','now'));
```

### 4.4 初始化数据（默认配置）

```sql
INSERT OR IGNORE INTO config (key, value, use_system_credential, updated_at) VALUES
('db_version', '3', 0, strftime('%s','now')),
('scan_depth', '5', 0, strftime('%s','now')),
('cache_max_size', '2147483648', 0, strftime('%s','now')),
('theme', 'dark', 0, strftime('%s','now')),
('poster_size', 'medium', 0, strftime('%s','now')),
('font_size', '14', 0, strftime('%s','now')),
('auto_start', 'false', 0, strftime('%s','now')),
('privacy_title', '智能网盘影视库', 0, strftime('%s','now')),
('privacy_tray', '智能网盘影视库', 0, strftime('%s','now')),
('proxy_enabled', 'false', 0, strftime('%s','now')),
('external_player', '', 0, strftime('%s','now')),
('scrape_sources', '["tmdb","douban","javbus","javdb","fanza"]', 0, strftime('%s','now')),
('video_extensions', '["mp4","mkv","avi","mov","rmvb","flv","wmv","ts","iso","m2ts"]', 0, strftime('%s','now')),
('use_system_credential', '0', 0, strftime('%s','now')),
('auto_resume_tasks', '1', 0, strftime('%s','now')),
('playback_refresh_interval', '240', 0, strftime('%s','now')),
('local_actor_base_dir', 'D:\\Media Library\\Actor Information\\picture', 0, strftime('%s','now')),
('auto_create_actors_from_scrape', '1', 0, strftime('%s','now')),
('actor_pending_review', '1', 0, strftime('%s','now')),
('allow_app_rename_actor_folders', '0', 0, strftime('%s','now')),
('actor_merge_auto_merge_folders', '1', 0, strftime('%s','now')),
('actor_merge_file_naming_pattern', '{name}_{index}{ext}', 0, strftime('%s','now')),
('actor_merge_dry_run', '1', 0, strftime('%s','now'));
```

---

## 5. TypeScript 接口定义

```typescript
// 文件项（来自115列表）
interface FileItem {
  cid: string;
  name: string;
  is_dir: boolean;
  size: number;
  update_time: number;
  file_id?: string;
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

// 分组项
interface GroupItem {
  id: number;
  name: string;
  type: 'manual' | 'genre' | 'collection';
  sort_order: number;
  movie_count?: number;
}

// 演员项（库中）
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
  local_folder_name?: string;
  is_pending: boolean;
  source?: string;
}

// 演员分组项
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

// 任务项
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
```

---

## 6. Tauri 命令接口

所有命令通过 `@tauri-apps/api/core` 的 `invoke` 调用。  
返回格式统一为 `Result<T, CommandError>`，前端接收时转换为 `{ status: 'ok', data: T }` 或 `{ status: 'error', code: number, message: string }`（错误码见第 12 节）。

### 6.1 认证 (auth)
```typescript
login_qrcode(): Promise<{ qrcode_url: string; uid: string }>;
login_status(uid: string): Promise<{ status: 'waiting'|'scanned'|'authorized'|'expired'; cookie?: string }>;
login_cookie(cookie: string): Promise<void>;
logout(): Promise<void>;
check_token(): Promise<boolean>;
```

### 6.2 文件系统与扫描 (fs)
```typescript
list_root(): Promise<Array<{ cid: string; name: string }>>;
scan_directory(path: string, depth: number, mode: 'manual' | 'incremental' | 'full'): Promise<{ total: number; new: number; updated: number; deleted: number }>;
get_files(path: string, page: number, pageSize: number): Promise<{ files: FileItem[]; total: number }>;
get_play_url(fileId: string): Promise<{ url: string; expire_at: number }>;
refresh_play_url(fileId: string): Promise<{ url: string; expire_at: number }>;
export_list(): Promise<string>;
```

### 6.3 刮削 (scrape)
```typescript
start_scrape(fileIds: string[]): Promise<string>;
pause_scrape(taskId: string): Promise<void>;
resume_scrape(taskId: string): Promise<void>;
manual_scrape(fileId: string, keyword: string): Promise<ScrapeResult[]>;
select_scrape_result(fileId: string, resultIdx: number): Promise<void>;
test_source(url: string): Promise<{ status: number; time_ms: number }>;
```

### 6.4 任务管理 (task)
```typescript
get_pending_tasks(): Promise<Task[]>;
resume_task(taskId: string): Promise<void>;
cancel_task(taskId: string): Promise<void>;
```

### 6.5 影视库 (library)
```typescript
get_movies(filters: FilterParams, sort: string, page: number): Promise<{ movies: MovieItem[]; total: number }>;
get_movie_detail(fileId: string): Promise<MovieDetail | null>;
batch_action(fileIds: string[], action: 'mark_watched' | 'mark_unwatched' | 'rescrape'): Promise<void>;
hide_movies(fileIds: string[]): Promise<void>;
unhide_movies(fileIds: string[]): Promise<void>;
```

### 6.6 分组 (groups)
```typescript
get_groups(): Promise<GroupItem[]>;
create_group(name: string, groupType?: 'genre' | 'collection'): Promise<GroupItem>;
rename_group(groupId: number, newName: string): Promise<void>;
delete_group(groupId: number): Promise<void>;
reorder_groups(orderedIds: number[]): Promise<void>;
add_movies_to_group(groupId: number, fileIds: string[]): Promise<void>;
remove_movie_from_group(groupId: number, fileId: string): Promise<void>;
```

### 6.7 演员库 (actress)
```typescript
// 基本查询与操作
sync_actress_data(): Promise<void>;
get_actresses_by_letter(): Promise<{ letter: string; actresses: ActressItem[] }[]>;
get_actresses_paginated(page: number, pageSize: number, search?: string, sortField?: string, sortOrder?: 'asc'|'desc', includePending?: boolean): Promise<{ list: ActressItem[]; total: number }>;
find_actress(name: string): Promise<ActressItem | null>;
update_actress(id: number, data: Partial<ActressItem>): Promise<void>;
delete_actresses(ids: number[]): Promise<number>;
get_actress_aliases(actressId: number): Promise<string[]>;
add_actress_alias(actressId: number, alias: string): Promise<void>;

// 本地文件夹扫描与管理
scan_local_actress_folder(folderPath?: string): Promise<{ added: number; total: number }>;
refresh_actress_avatar(actressId: number): Promise<void>;
confirm_actor(actorId: number, accepted: boolean): Promise<void>;
update_actor_local_folder(actorId: number, folderPath: string | null): Promise<void>;
rename_actor_and_folder(actorId: number, newName: string, renameFolder: boolean): Promise<{ success: boolean; error?: string }>;
get_actress_local_folder(actressId: number): Promise<string | null>;
sync_actress_with_local_folder(actressId: number): Promise<void>;

// 合并演员
merge_actresses(
  sourceId: number, 
  targetId: number, 
  options: { 
    mergeFolders: boolean; 
    conflictPolicy?: 'rename' | 'skip' | 'overwrite';
    dryRun?: boolean;
  }
): Promise<{ 
  success: boolean; 
  movedFiles: string[]; 
  conflicts: string[]; 
  renamedFiles?: Array<{ from: string; to: string }>;
  error?: string 
}>;

detect_duplicate_actresses(threshold?: number): Promise<Array<{ id1: number, id2: number, similarity: number }>>;
```

### 6.8 演员分组 (actress groups)
```typescript
get_actress_groups(): Promise<ActressGroupItem[]>;
create_actress_group(name: string): Promise<ActressGroupItem>;
rename_actress_group(groupId: number, newName: string): Promise<void>;
delete_actress_group(groupId: number): Promise<void>;
add_actresses_to_group(groupId: number, actressIds: number[]): Promise<void>;
remove_actress_from_group(groupId: number, actressId: number): Promise<void>;
```

### 6.9 播放进度 (player)
```typescript
get_progress(fileId: string): Promise<{ progress: number; duration: number; is_finished: number }>;
save_progress(fileId: string, progress: number, duration: number): Promise<void>;
end_playback(fileId: string): Promise<void>;
```

### 6.10 配置 (config)
```typescript
get_config(key: string): Promise<string | null>;
set_config(key: string, value: string): Promise<void>;
get_all_config(): Promise<Record<string, string>>;
set_secure_config(key: string, value: string): Promise<void>;
get_secure_config(key: string): Promise<string | null>;
clear_secure_config(key: string): Promise<void>;
```

### 6.11 应用控制 (app)
```typescript
hide_window(): Promise<void>;
check_updates(): Promise<{ has_update: boolean; version?: string; url?: string; body?: string }>;
```

---

## 7. 前端路由与导航守卫

### 7.1 路由表

使用 `createWebHistory()`，基础路径 `/`。

| 路径 | 名称 | 组件 | 元信息 |
|------|------|------|--------|
| `/` | home | PosterWall.vue | `requiresAuth: true` |
| `/detail/:fileId` | detail | Detail.vue | `requiresAuth: true` |
| `/player/:fileId` | player | Player.vue | `requiresAuth: true` |
| `/actress` | actress | Actress.vue | `requiresAuth: true` |
| `/actress-table` | actress-table | ActressTable.vue | `requiresAuth: true` |
| `/scan` | scan | Scan.vue | `requiresAuth: true` |
| `/settings` | settings | Settings.vue | `requiresAuth: false` |

### 7.2 路由守卫

```typescript
router.beforeEach(async (to, from, next) => {
  const requiresAuth = to.meta.requiresAuth !== false;
  if (requiresAuth) {
    const isLoggedIn = await invoke('check_token');
    if (!isLoggedIn) {
      next('/settings');
      return;
    }
  }
  next();
});
```

### 7.3 路由参数类型

- `/detail/:fileId` – `fileId` 为字符串，对应 `MovieItem.file_id`
- `/player/:fileId` – 同上

---

## 8. 系统托盘菜单

托盘菜单结构（右键菜单）：

```
📌 显示主窗口          (单击托盘图标同样行为)
─────────────────────
📁 扫描任务状态        → 动态文本，如“扫描中 45%”或“空闲”；点击打开扫描页
📁 刮削任务状态        → 同上
─────────────────────
🎬 最近添加            → 子菜单：最近7天添加的5部影片，点击跳转详情页
─────────────────────
🌙 暗色模式            → 勾选状态，点击切换
🚪 退出
```

- 单击托盘图标：如果主窗口隐藏则显示并置顶；如果已显示则隐藏。
- 暗色模式切换立即生效，并保存到配置。

---

## 9. Tauri 事件列表

后端通过 `app.emit_all(event_name, payload)` 发送。

| 事件名 | 负载类型 | 说明 |
|--------|----------|------|
| `scan-progress` | `{ percent: number, current_file: string, total_files: number }` | 扫描进度 |
| `scrape-progress` | `{ taskId: string, percent: number, current: number, total: number }` | 刮削进度 |
| `task-recovery-required` | `Task[]` | 应用启动时未完成任务列表 |
| `login-status-changed` | `{ isLoggedIn: boolean }` | 115登录状态变化 |
| `config-updated` | `{ key: string, value: any }` | 配置变更（如主题、代理） |
| `actress-folder-scanned` | `{ added: number; total: number }` | 本地演员文件夹扫描完成 |
| `actresses-merged` | `{ sourceId: number; targetId: number; movedFiles: number }` | 演员合并完成 |
| `merge-progress` | `{ current: number; total: number; file: string }` | 合并文件进度 |

前端监听示例：
```typescript
import { listen } from '@tauri-apps/api/event';
listen('scan-progress', (event) => {
  console.log(event.payload);
});
```

---

## 10. 外部播放器调用协议

- **配置键** `external_player`：存储外部播放器可执行文件的完整路径（如 `C:\Program Files\DAUM\PotPlayer\PotPlayerMini64.exe`）。
- **调用方式**：使用 `tauri-plugin-shell` 的 `Command`。
- **参数模板**：支持以下占位符自动替换：
  - `%URL%` → 播放链接（必须）
  - `%TITLE%` → 影片标题
  - `%SUBTITLE%` → 字幕文件路径（预留，暂不实现）
- 示例：如果配置值为 `"%PLAYER%" "%URL%"`，实际执行 `PotPlayerMini64.exe "https://..."`。
- 前端点击“外部播放”按钮时，调用 `invoke('get_play_url', { fileId })` 获得链接，再通过 `shell` 打开，并可选择隐藏主窗口。

---

## 11. 日志规范

- **日志级别**：`INFO` 及以上（生产环境），`DEBUG` 仅在开发版本启用。
- **输出格式**：`[2025-01-15 10:30:45] [INFO] [pan115] Request successful`
- **存储路径**：`%APPDATA%\smart-media-vault\logs\app.log`
- **轮转策略**：
  - 单文件最大 10 MB
  - 保留最近 5 个备份文件（`app.log.1`, `app.log.2` ...）
- **敏感信息脱敏**：打印 115 cookie 时仅显示前 8 位 + `****`；打印 TMDB API Key 时仅显示前 4 位 + `****`。

---

## 12. 错误码定义（`CommandError.code`）

| code | 含义 | 说明 |
|------|------|------|
| 1000 | 数据库错误 | 查询失败、迁移错误、约束冲突 |
| 2000 | 网络错误 | 115 API 请求失败、超时、DNS 错误 |
| 2100 | 未授权 | 115 cookie 失效或未登录 |
| 2200 | 限流 | HTTP 429 响应 |
| 3000 | 无效输入 | 参数缺失、格式错误、超出范围 |
| 3100 | 资源未找到 | 文件、演员、分组不存在 |
| 4000 | 内部错误 | 未捕获的 panic 或其他未知错误 |
| 4100 | 刮削失败 | 所有刮削源均无结果或全部超时 |

前端可根据错误码做差异化处理（如 2100 跳转登录、2200 显示重试等待等）。

---

## 13. 应用数据存储路径

- **Windows**：`%APPDATA%\smart-media-vault\`
  - `vault.db` – 主数据库
  - `config.json` – 运行时配置备份（非敏感，用于快速读取）
  - `logs/app.log` – 滚动日志文件
  - `images/` – 海报、头像缓存（WebP 格式）
  - `avatars/` – 演员头像缓存
  - `backups/` – 数据库备份（最多 5 份，每日首次启动时自动备份）
- **开发环境**：`<项目目录>/target/tauri/smart-media-vault-data/`（模拟路径）

---

## 14. 构建与打包

### 14.1 开发环境

```bash
npm install
cargo install tauri-cli
cargo tauri dev
```

### 14.2 生产构建（Windows 便携版 + 安装包）

```bash
cargo tauri build --target x86_64-pc-windows-msvc --release
```

产物位置：
- 安装包：`src-tauri/target/release/bundle/nsis/智能网盘影视库_1.2.0_x64-setup.exe`
- 便携版 zip：`src-tauri/target/release/bundle/windows/`（需在 `tauri.conf.json` 中配置）

### 14.3 环境变量

开发时在项目根目录放置 `.env` 文件（**不提交 Git**）：

```ini
TMDB_API_KEY=your_key_here
GFRIENDS_REPO_URL=https://raw.githubusercontent.com/xxx/Filetree.json
```

前端 Vite 可通过 `import.meta.env.VITE_*` 访问，但敏感密钥仅 Rust 后端使用。

### 14.4 GitHub Actions CI

```yaml
name: Build and Release
on: push
jobs:
  build:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 20 }
      - uses: actions-rs/toolchain@v1
        with: { toolchain: stable, target: x86_64-pc-windows-msvc }
      - run: npm install
      - run: cargo tauri build
      - uses: tauri-apps/tauri-action@v0
        with:
          tagName: v__VERSION__
          releaseName: 'v__VERSION__'
          releaseBody: 'See changelog'
          releaseDraft: true
```

---

## 15. 测试策略

### 15.1 Rust 单元测试

位于 `src-tauri/src/`，使用 `#[cfg(test)]`。

- `utils/filename_parser.rs`：测试 16 种番号格式。
- `services/scrape_manager.rs`：测试评分合并算法。
- `services/actress_sync.rs`：测试别名匹配。
- `services/actress_folder_manager.rs`：测试扫描、合并、文件重命名、冲突处理。
- `db/queries.rs`：使用 `tempfile` 临时数据库。

运行：`cargo test`

### 15.2 前端单元测试

- Vitest + `@vue/test-utils`
- 测试 Pinia stores（如 `library.ts` 中的 `fetchMovies`）
- 测试组合式函数 `usePlayerRefresh`
- 测试演员表组件的合并对话框逻辑

运行：`npm run test:unit`

### 15.3 端到端测试

- `tauri-driver` + `WebDriverIO`
- 测试完整流程：登录 115 → 扫描 → 刮削 → 浏览详情 → 播放 → 检查进度
- 可增加演员合并流程的 E2E 测试

运行：`npm run test:e2e`（需先构建）

### 15.4 Mock 外部服务

- 使用 `mockito` 模拟 115 API 和刮削源 HTTP 响应
- 使用 `tempfile` 模拟文件系统

---

## 16. 安全与隐私

- **敏感数据**：115 Cookie、TMDB API Key 均使用安全存储（系统凭据管理器或 AES-256-GCM 本地加密）。
- **日志脱敏**：输出前对 Cookie、API Key 进行部分遮盖。
- **HTTPS only**：所有外部 API 请求使用 HTTPS。
- **输入校验**：Tauri 命令参数进行类型校验与边界检查，数据库使用参数化查询防注入。
- **自动更新签名**：使用 Tauri 的更新签名公钥，防止中间人攻击。
- **文件操作安全**：应用重命名本地演员文件夹需要用户明确授权（配置 `allow_app_rename_actor_folders` 默认为 `false`）。所有文件操作前应有 dry-run 预览。

---

## 17. 性能指标要求

- 启动时间（冷启动）：< 2 秒
- 数据库查询（分页 20 条，含 FTS5 全文搜索）：< 50 ms
- 海报墙滚动帧率：≥ 60 fps（虚拟滚动）
- 扫描 5000 个文件（增量模式）：< 30 秒
- 刮削单文件（5 个源）：< 5 秒
- 内存占用空闲：≤ 100 MB，播放时 ≤ 200 MB
- 演员合并（1000 个文件）：< 5 秒

---

## 18. 未来扩展性预留

- **多网盘后端**：定义 `StorageBackend` trait，后续可添加阿里云盘、百度网盘实现。
- **刮削源插件化**：通过 `dyn Scraper` 注册，支持用户编写 Lua 脚本。
- **WebSocket 监控**：开发模式下可开启 WebSocket 服务，实时查看日志与性能指标。

---

## 19. 版本与模块变更标记

### v1.2.3（相对于 v1.2.2）

| 模块 | 变更类型 | 说明 |
|------|----------|------|
| **后端命令** | 修改 | `create_group` 重名检查增加类型范围，允许跨类型同名 |
| **前端视图** | 修改 | `App.vue` 侧边栏分组子项支持右键重命名/删除；右键菜单分离(入口/分组)，修复新增弹窗catch错误 |
| **配置** | 修改 | README版本表格拆分为v1.2.2和v1.2.3两条 |

### v1.2.2（相对于 v1.2.0）

| 模块 | 变更类型 | 说明 |
|------|----------|------|
| **数据库** | 新增 | v4迁移：av_actors补created_at列，刮削源扩展至13个 |
| **前端视图** | 重写 | `App.vue` 侧边栏自定义导航：入口右键分组CRUD，分组子项显示+折叠，收藏影片入口 |
| **前端视图** | 重写 | `ActressTable.vue` 支持列宽拖动/排序/勾选/分页固定，去除ID/首字母/头像/本地文件夹列，增加别名列，来源简化为网络/本地，批量合并/删除 |
| **前端视图** | 修改 | `Scan.vue` 全屏加载遮罩，表格resizable/stripe/sortable |
| **前端视图** | 修改 | `Settings.vue` Tab页签布局(刮削源/代理/缓存/界面/常规)，对比度CSS修复 |
| **前端视图** | 新增 | `Favorites.vue` 收藏影片页面，右键菜单 |
| **前端视图** | 修改 | `PosterWall.vue` 右键菜单(收藏/分组/隐藏) |
| **前端视图** | 修改 | `Actress.vue` 右键菜单(分组/查看影片)，分组管理对话框 |
| **后端服务** | 修改 | `pan115.rs` 扫码API修正(PNG→base64)，Set-Cookie提取 |
| **后端命令** | 修改 | `get_groups` 支持按类型筛选 |
| **配置** | 修改 | CSP添加data:和style-src，刮削源默认13个 |

### v1.2.0（相对于 v1.1.0）

| 模块 | 变更类型 | 说明 |
|------|----------|------|
| **数据库** | 新增字段 | `av_actors` 表增加 `local_folder_name`, `is_pending`, `source` |
| **数据库** | 新增配置项 | 7 个新配置项 |
| **后端服务** | 新增 | 演员文件夹管理器 `actress_folder_manager.rs` |
| **后端服务** | 修改 | `actress_sync.rs`：增加本地文件夹扫描逻辑 |
| **后端命令** | 新增 | 9 个演员文件夹相关命令 |
| **后端命令** | 修改 | `update_actress` 支持新字段；`sync_actress_data` 逻辑修改 |
| **前端类型** | 修改 | `ActressItem` 增加 `local_folder_name`, `is_pending`, `source` |
| **前端状态** | 修改 | `actress.ts` store 增加方法 |
| **前端视图** | 修改 | `ActressTable.vue`, `Settings.vue` |
| **事件** | 新增 | `actress-folder-scanned`, `actresses-merged`, `merge-progress` |

---

## 20. 代码清理与维护规范

### 20.1 问题场景
- AI 或多人协作修改代码后，旧的函数、模块、配置文件可能未被删除。
- 数据库迁移脚本累积过多，废弃的表或字段仍保留在 schema 中。
- 前端组件重构后，旧组件文件残留。
- 临时文件夹、缓存目录未自动清理。

### 20.2 解决方案总览

| 层面 | 措施 | 工具/方法 |
|------|------|------------|
| **版本控制** | 每次提交前 `git status` 检查未跟踪文件，提交后 Code Review 必须确认文件删除列表 | Git |
| **静态分析** | 检测未使用的函数、变量、导入、文件 | Rust: `cargo deadlinks`, `cargo udeps`<br>TS: `eslint-plugin-unused-imports`, `knip` |
| **构建清理** | 每次构建前清理旧输出目录，避免残留 | `cargo clean` (仅针对后端), `rm -rf dist` |
| **数据库维护** | 定期运行 `PRAGMA integrity_check`；提供清理脚本删除孤立数据；迁移脚本中明确标记废弃表和字段 | SQLite 内置；自定义 `cargo db clean` |
| **AI 协作规范** | AI 修改时必须输出：新增文件列表、修改文件列表、删除文件列表；人工确认后方可合并 | 提交信息模板 |
| **自动化 CI 检查** | CI 中运行未使用代码检测，如果发现则构建失败 | GitHub Actions + `knip` / `cargo udeps` |

### 20.3 具体实施步骤

#### 20.3.1 开发者本地检查（含 AI 生成代码）
每次提交前执行：
```bash
# 前端：检测未使用的导出、文件、依赖
npx knip --include-libs

# 后端：检测未使用的依赖和死链接
cargo udeps
cargo deadlinks --check-http

# 通用：查找可能未使用的文件（基于 git 跟踪）
git ls-files --others --exclude-standard  # 列出未跟踪文件，确认是否需要
```

将这些命令加入 `package.json` 的 `scripts` 和 `Cargo.toml` 的 `[package.metadata]`。

#### 20.3.2 数据库清理规范
- **迁移脚本命名**：`v{version}_{description}.sql`，且每个脚本只做增量变更，不修改历史表结构（除非重大版本）。
- **废弃表/字段处理**：
  - 在最新版本的迁移脚本中，添加 `-- DEPRECATED` 注释，并在配置表 `meta` 中记录 `deprecated_since_version`。
  - 提供独立的 `cleanup.sql` 脚本（不自动运行），由 DBA 或高级用户手动执行，删除废弃对象。
- **自动清理孤立数据**：在应用启动时可选运行 `PRAGMA foreign_keys=ON` 后执行 `DELETE FROM ... WHERE NOT EXISTS` 清理孤儿记录。

#### 20.3.3 AI 变更清单模板
当 AI 提交修改时，必须附带以下格式的变更说明（可写入 PR 描述）：

```markdown
## 变更清单
### 新增文件
- src-tauri/services/actress_folder_manager.rs
- src/views/ActressMergeDialog.vue

### 修改文件
- src-tauri/db/migrations/v3.sql
- src/stores/actress.ts

### 删除文件
- src/old_components/DeprecatedActorCard.vue
- src-tauri/services/old_scrape.rs

### 需要手动清理的残留项
- 数据库表 `temp_import_queue` 已不再使用，请手动执行 `DROP TABLE temp_import_queue;`
- 配置项 `old_proxy_enabled` 无引用，可删除
```

#### 20.3.4 自动化 CI 检查示例（GitHub Actions）
```yaml
name: Lint and Cleanliness
on: [push, pull_request]
jobs:
  check_unused:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Setup Node
        uses: actions/setup-node@v4
        with: { node-version: 20 }
      - run: npm ci
      - run: npx knip --no-exit-code --max-warnings 0
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with: { toolchain: stable }
      - run: cargo install cargo-udeps --locked
      - run: cargo udeps
```

### 20.4 清理策略总结表

| 清理对象 | 触发时机 | 执行方式 | 是否自动 |
|----------|----------|----------|----------|
| 构建输出目录 | 每次构建前 | `cargo clean` / `rm -rf dist` | 是 |
| 未跟踪的临时文件 | 提交前提示 | `git clean -nd` 预览，人工确认 | 手动 |
| 未使用的 Rust 代码 | CI 检查 | `cargo udeps` | 检查但不自动删除 |
| 未使用的 TS 导出 | 开发时 | IDE 插件 / `knip` | 提示 |
| 废弃的数据库表/字段 | 重大版本发布 | 手动执行 `cleanup.sql` | 手动 |
| 孤立的数据记录 | 应用启动时 | 运行清理 SQL（可选开关） | 可配置 |

### 20.5 对 AI 开发的强制要求
- 在每次修改的最终输出中，必须提供**变更清单**（新增/修改/删除文件列表）。
- 如果涉及数据库 schema 变更，必须提供**迁移脚本**和**回滚脚本**（如果可回滚）。
- 任何删除操作（文件、目录、配置项）都需要在变更清单中明确标出，并说明理由。
- 如果不确定是否还有代码引用某功能，应使用 `grep` 或 `ripgrep` 全局搜索后再决定删除。

---

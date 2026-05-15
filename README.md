# Actor-Information

智能网盘影视库 (smart-media-vault) — 管理 115 网盘中的影视文件，自动刮削元数据（海报、演员、简介等），提供海报墙浏览、演员库管理、播放进度追踪、分组管理等功能。

## 技术栈

| 类别 | 技术 |
|------|------|
| 桌面框架 | Tauri 2.0 |
| 前端 | Vue 3 + TypeScript + Element Plus |
| 后端 | Rust 2021 |
| 数据库 | SQLite (WAL 模式) |
| 构建 | Vite 5.4 |

## 版本记录

| 版本号 | 修改内容 | 时间 |
|--------|----------|------|
| v1.2.2 | 修复二维码CSP拦截(base64不显示)、av_actors缺少created_at列、刮削源扩展至13个(tmdb/imdb/douban/javbus/javdb/fanza/airav/xcity/mgstage/fc2/jav321/javlibrary/arzon)、设置页改为Tab页签(刮削源/代理/缓存/界面/常规) | 2026-05-15 |
| v1.2.0 | 演员本地文件夹管理：扫描/合并/重复检测/待审核/同步；9个新Tauri命令；av_actors新增3字段；Cookie直接登录 | 2026-05-15 |
| v1.1.0 | 初始版本：完整的海报墙、影片详情、在线播放、演员库管理、演员表格视图、扫描管理、设置页面；60+ Tauri 命令接口；SQLite 数据库含 13 张核心表；115 网盘扫码登录；多源刮削框架；AES-256-GCM 安全存储 | 2026-05-15 |

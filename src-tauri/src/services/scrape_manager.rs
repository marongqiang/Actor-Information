use crate::db;
use crate::utils::error::{CommandError, CommandResult};
use crate::utils::filename_parser;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScrapeResult {
    pub source: String,
    pub title: String,
    pub year: Option<i32>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub overview: Option<String>,
    pub rating: Option<f64>,
    pub runtime: Option<i32>,
    pub director: Option<String>,
    pub genre: Option<Vec<String>>,
    pub actors: Option<Vec<String>>,
    pub score: i32,
}

fn build_scrape_client() -> reqwest::blocking::Client {
    // This client only connects to MetaTube on localhost:9588.
    // MetaTube server handles external proxy via environment variables.
    // Do NOT set proxy on this client — it would intercept localhost.
    reqwest::blocking::Client::builder()
        .cookie_store(true)
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/148.0.0.0 Safari/537.36 Edg/148.0.0.0")
        .build()
        .expect("Failed to build scrape client")
}

/// Client for external downloads (with proxy) — posters, covers, etc.
fn get_download_client() -> &'static reqwest::blocking::Client {
    use std::sync::OnceLock;
    static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        let mut builder = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/148.0.0.0 Safari/537.36 Edg/148.0.0.0");
        if let Ok(Some(enabled)) = db::with_db(|c| crate::db::queries::get_config(c, "proxy_enabled")) {
            if enabled == "true" {
                if let Ok(Some(host)) = db::with_db(|c| crate::db::queries::get_config(c, "proxy_host")) {
                    let port = db::with_db(|c| crate::db::queries::get_config(c, "proxy_port"))
                        .ok().flatten().and_then(|p| p.parse().ok()).unwrap_or(1080);
                    let proxy_url = format!("http://{}:{}", host, port);
                    if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
                        builder = builder.proxy(proxy);
                    }
                }
            }
        }
        builder.build().expect("Failed to build download client")
    })
}

fn get_client() -> &'static reqwest::blocking::Client {
    use std::sync::OnceLock;
    static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    CLIENT.get_or_init(|| build_scrape_client())
}

/// Client with proxy — for HTML scrapers that access external websites
fn get_external_client() -> &'static reqwest::blocking::Client {
    use std::sync::OnceLock;
    static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        let mut builder = reqwest::blocking::Client::builder()
            .cookie_store(true)
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/148.0.0.0 Safari/537.36 Edg/148.0.0.0");
        if let Ok(Some(enabled)) = db::with_db(|c| crate::db::queries::get_config(c, "proxy_enabled")) {
            if enabled == "true" {
                if let Ok(Some(host)) = db::with_db(|c| crate::db::queries::get_config(c, "proxy_host")) {
                    let port = db::with_db(|c| crate::db::queries::get_config(c, "proxy_port"))
                        .ok().flatten().and_then(|p| p.parse().ok()).unwrap_or(1080);
                    if let Ok(proxy) = reqwest::Proxy::all(&format!("http://{}:{}", host, port)) {
                        builder = builder.proxy(proxy);
                    }
                }
            }
        }
        builder.build().expect("Failed to build external client")
    })
}

/// Synchronous scrape - call from spawn_blocking context
pub fn scrape_file(file_id: &str, sources: &[String]) -> Result<Vec<ScrapeResult>, CommandError> {
    let file_name = db::with_db(|conn| {
        conn.query_row("SELECT file_name FROM movies WHERE file_id=?1", [file_id], |r| r.get::<_, String>(0))
            .map_err(|_| CommandError::not_found("文件不存在"))
    })?;

    let parsed = filename_parser::parse_filename(&file_name);
    let query = parsed.id_number.as_deref().unwrap_or(&parsed.cleaned);

    // Priority: individual scrapers first, MetaTube as fallback
    let mut ordered_sources: Vec<&String> = sources.iter().collect();
    ordered_sources.sort_by_key(|s| if s.as_str() == "metatube" { 2 } else { 0 });

    let mut results = Vec::new();
    let mut got_good_data = false;
    for source in ordered_sources {
        if got_good_data && source.as_str() == "metatube" {
            log::debug!("刮削 {}: 跳过(独立刮削器已完成)", source);
            continue;
        }

        log::info!("刮削 {}: 开始查询 [{}]", source, query);
        let r = match source.as_str() {
            "metatube" => scrape_metatube(query),
            "tmdb" => scrape_tmdb(query),
            "imdb" => scrape_imdb(query),
            "douban" => scrape_douban(query),
            "javbus" => scrape_javbus(query),
            "javdb" => scrape_javdb(query),
            "javlibrary" => scrape_javlib(query),
            "fanza" => scrape_fanza(query),
            "arzon" => scrape_arzon(query),
            "mgstage" => scrape_mgstage(query),
            "fc2" => scrape_fc2(query),
            "jphoo" => scrape_jphoo(query),
            "airav" => scrape_airav(query),
            "jav321" => scrape_jav321(query),
            "xcity" => scrape_xcity(query),
            "prestige" => scrape_prestige(query),
            "avsox" => scrape_avsox(query),
            "njav" => scrape_njav(query),
            "getav" => scrape_getav(query),
            "whostv" => scrape_whostv(query),
            "fc2ppvdb" => scrape_fc2ppvdb(query),
            "faleno" => scrape_faleno(query),
            "duga" => scrape_duga(query),
            "sod" => scrape_sod(query),
            "dahlia" => scrape_dahlia(query),
            "1pondo" => scrape_1pondo(query),
            "10musume" => scrape_10musume(query),
            "caribbeancom" => scrape_caribbeancom(query),
            "caribbeancompr" => scrape_caribbeancompr(query),
            "c0930" => scrape_c0930(query),
            "gcolle" => scrape_gcolle(query),
            "getchu" => scrape_getchu(query),
            "h0930" => scrape_h0930(query),
            "h4610" => scrape_h4610(query),
            "heydouga" => scrape_heydouga(query),
            "heyzo" => scrape_heyzo(query),
            "javfree" => scrape_javfree(query),
            "kin8" => scrape_kin8(query),
            "muramura" => scrape_muramura(query),
            "mywife" => scrape_mywife(query),
            "pacopacomama" => scrape_pacopacomama(query),
            "pcolle" => scrape_pcolle(query),
            "fc2hub" => scrape_fc2hub(query),
            "tokyohot" => scrape_tokyohot(query),
            "ave" => scrape_ave(query),
            _ => { log::debug!("刮削源 {} 未实现", source); continue; }
        };
        match &r {
            Ok(res) => {
                // Check if it's actually a 404/error page
                let is_bogus = res.title.contains("見つかりません") || res.title.contains("Service Unavailable")
                    || res.title.contains("Not Found") || res.title.contains("404")
                    || res.title.contains("Search Results") || res.title.contains("エラー")
                    || res.title.contains("全動画") || res.title.contains("JAVten")
                    || res.title.contains("セレブ") || res.title.contains("AVエンターテインメント")
                    || res.title.len() < 4;
                if is_bogus {
                    log::warn!("刮削 {}: 返回错误页面, 忽略", source);
                    continue;
                }
                log::info!("刮削 {}: 成功, title={}", source, res.title);
                // Only skip MetaTube if scraper returned genuinely good data
                let has_good_data = source.as_str() != "metatube"
                    && res.poster_url.is_some()
                    && res.actors.as_ref().map_or(0, |a| a.len()) >= 2
                    && res.genre.as_ref().map_or(false, |g| !g.is_empty());
                if has_good_data {
                    got_good_data = true;
                    log::info!("独立刮削器 {} 返回完整数据, 跳过MetaTube", source);
                }
                if source.as_str() == "metatube" { got_good_data = true; }
                results.push(r.unwrap());
            }
            Err(e) => {
                log::warn!("刮削 {}: 失败 - {}", source, e);
            },
        }
    }
    results.sort_by(|a, b| b.score.cmp(&a.score));
    Ok(results)
}

pub fn apply_scrape_result(file_id: &str, result: &ScrapeResult) -> CommandResult<()> {
    let now = db::now_ts();

    // Get file_name to derive the movie code for folder naming
    let file_name: String = db::with_db(|conn| {
        conn.query_row("SELECT file_name FROM movies WHERE file_id=?1", [file_id], |r| r.get(0))
            .map_err(|e| CommandError::db(&e.to_string()))
    }).unwrap_or_default();
    let parsed = filename_parser::parse_filename(&file_name);
    let code = parsed.id_number.as_deref().unwrap_or(&parsed.cleaned).to_string();

    // Download poster — save to images/posters/{code}/poster.jpg
    let mut poster_local = None;
    if let Some(ref poster_url) = result.poster_url {
        if poster_url.starts_with("http") {
            let code_dir = db::get_data_dir().join("images").join("posters").join(&code);
            let _ = std::fs::create_dir_all(&code_dir);
            // Determine extension from URL or default to jpg
            let ext = poster_url.rsplit('.').next().and_then(|e| {
                let e = e.split('?').next().unwrap_or("jpg");
                if e.len() <= 4 && e.chars().all(|c| c.is_ascii_alphabetic()) { Some(e) } else { None }
            }).unwrap_or("jpg");
            let filename = format!("poster.{}", ext);
            let filepath = code_dir.join(&filename);
            log::info!("下载海报: {} -> {}", poster_url, filepath.display());
            match get_download_client().get(poster_url)
                .header("Referer", "https://www.javbus.com/")
                .send() {
                Ok(resp) => {
                    if let Ok(bytes) = resp.bytes() {
                        // Check it's actually an image (not HTML error page)
                        let is_image = bytes.len() > 1000 && (
                            bytes.starts_with(b"\xff\xd8\xff") || // JPEG
                            bytes.starts_with(b"\x89PNG") ||     // PNG
                            bytes.starts_with(b"RIFF") ||         // WEBP
                            bytes.starts_with(b"GIF8")            // GIF
                        );
                        if is_image {
                            let _ = std::fs::write(&filepath, &bytes);
                            poster_local = Some(filepath.to_string_lossy().to_string());
                            log::info!("海报下载成功: {} bytes", bytes.len());
                        } else {
                            log::warn!("海报不是图片格式(可能是反爬页面), 大小={} bytes", bytes.len());
                        }
                    }
                }
                Err(e) => log::warn!("海报下载失败: {}", e),
            }
        }
    }

    // Save scraped info — title goes to original_title, never overwrite the filename-based title
    db::with_db(|conn| {
        conn.execute(
            "UPDATE movies SET original_title=?1,year=?2,poster_url=?3,poster_local=COALESCE(?4,poster_local),overview=?5,rating=?6,runtime=?7,genre=?8,scrape_status=2,updated_at=?9 WHERE file_id=?10",
            rusqlite::params![result.title, result.year, result.poster_url, poster_local, result.overview, result.rating, result.runtime,
                result.genre.as_ref().map(|g| serde_json::to_string(g).unwrap_or_default()), now, file_id])?;
        if let Some(actors) = &result.actors {
            for name in actors {
                conn.execute("INSERT OR IGNORE INTO actors (name) VALUES (?1)", [name])?;
                let actor_id: i64 = conn.query_row("SELECT id FROM actors WHERE name=?1", [name], |r| r.get(0))?;
                conn.execute("INSERT OR IGNORE INTO movie_actors (movie_id,actor_id) VALUES (?1,?2)", rusqlite::params![file_id, actor_id])?;
            }
        }
        Ok(())
    })?;

    // Auto-translate title and genres
    if !result.title.is_empty() {
        auto_translate_movie(file_id, &result.title, result.overview.as_deref(), result.genre.as_deref());
    }

    Ok(())
}

/// Translate title + overview + genres after scraping
fn auto_translate_movie(file_id: &str, title: &str, overview: Option<&str>, genres: Option<&[String]>) {
    auto_translate_title(file_id, title);
    if let Some(ov) = overview {
        if !ov.is_empty() { auto_translate_overview(file_id, ov); }
    }
    if let Some(genres) = genres {
        if !genres.is_empty() { auto_translate_genres(file_id, genres); }
    }
}

fn auto_translate_overview(file_id: &str, overview: &str) {
    if overview.len() < 20 { return; } // too short, skip
    let deepseek_key = crate::services::secure_config::get_secure_config("deepseek_api_key")
        .ok().flatten().unwrap_or_default();
    let prompt = format!(
        "你是日本AV影片简介本地化翻译专家。将以下日文AV剧情简介改写成中文。\n\n要求：\n1. 完全按中文母语者阅读习惯改写，不要日语句式\n2. 使用AV常用的中文词汇，保持色情张力\n3. 不要直译，要有画面感和冲击力\n4. 只返回翻译结果\n\n原文：\n{}",
        overview
    );
    let cn = if !deepseek_key.is_empty() {
        translate_via_deepseek(&prompt, &deepseek_key)
    } else {
        None
    }.or_else(|| translate_via_google(overview));

    if let Some(cn) = cn {
        let cn = cn.trim().to_string();
        if !cn.is_empty() && cn.len() > 10 {
            let _ = db::with_db(|conn| {
                conn.execute("UPDATE movies SET chinese_overview=?1 WHERE file_id=?2",
                    rusqlite::params![cn, file_id])?;
                Ok(())
            });
            log::info!("简介翻译完成: {}... -> {}...", truncate_log(overview, 30), truncate_log(&cn, 30));
        }
    }
}

fn auto_translate_genres(file_id: &str, genres: &[String]) {
    let mut translated: Vec<String> = Vec::new();
    let mut missing: Vec<(usize, String)> = Vec::new();

    // Step 1: Check library, collect missing
    for (i, g) in genres.iter().enumerate() {
        if let Ok(Some(cn)) = db::with_db(|conn| crate::db::queries::get_genre_translation(conn, g)) {
            if !cn.is_empty() { translated.push(cn); continue; }
        }
        translated.push(String::new());
        missing.push((i, g.clone()));
    }

    // Step 2: Batch translate missing via API
    if !missing.is_empty() {
        let list: Vec<String> = missing.iter().enumerate()
            .map(|(n, (_, g))| format!("{}.{}", n+1, g)).collect();
        let deepseek_key = crate::services::secure_config::get_secure_config("deepseek_api_key")
            .ok().flatten().unwrap_or_default();
        let prompt = format!(
            "将以下日本AV标签翻译成简体中文，严格按编号格式输出：\n{}\n\n输出格式（每行一个）：\n1.中文\n2.中文\n...",
            list.join("\n")
        );
        let cn_text = if !deepseek_key.is_empty() {
            translate_via_deepseek(&prompt, &deepseek_key)
        } else {
            None
        }.or_else(|| translate_via_google(&list.join("\n")));

        if let Some(ref text) = cn_text {
            for line in text.lines() {
                let line = line.trim();
                if let Some(dot) = line.find('.') {
                    let num: usize = line[..dot].trim().parse().unwrap_or(0);
                    let cn = line[dot+1..].trim().to_string();
                    if num > 0 && num <= missing.len() && !cn.is_empty() {
                        let (idx, ref ja) = missing[num - 1];
                        translated[idx] = cn.clone();
                        let _ = db::with_db(|conn| {
                            crate::db::queries::set_genre_translation(conn, ja, &cn)
                        });
                    }
                }
            }
        }

        // Fallback: for any still missing, try Google individually
        for (idx, ja) in &missing {
            if translated[*idx].is_empty() {
                if let Some(cn) = translate_via_google(ja) {
                    let cn = cn.trim().to_string();
                    if !cn.is_empty() && &cn != ja {
                        translated[*idx] = cn.clone();
                        let _ = db::with_db(|conn| {
                            crate::db::queries::set_genre_translation(conn, ja, &cn)
                        });
                    }
                }
            }
        }
    }

    // Step 3: Remove still-empty (use original as fallback)
    for (i, g) in genres.iter().enumerate() {
        if translated[i].is_empty() { translated[i] = g.clone(); }
    }

    let json = serde_json::to_string(&translated).unwrap_or_default();
    let _ = db::with_db(|conn| {
        conn.execute("UPDATE movies SET genre=?1 WHERE file_id=?2",
            rusqlite::params![json, file_id])?;
        Ok(())
    });
    log::info!("类型翻译: {:?} -> {:?}", genres, translated);
}

/// Auto-translate original_title to chinese_name after scraping
fn auto_translate_title(file_id: &str, original_title: &str) {
    if original_title.is_empty() { return; }
    // Check if already has chinese_name
    let has_cn: bool = db::with_db(|conn| {
        Ok(conn.query_row(
            "SELECT chinese_name IS NOT NULL FROM movies WHERE file_id=?1",
            [file_id],
            |r| r.get(0),
        ).unwrap_or(false))
    }).unwrap_or(false);
    if has_cn { return; }

    log::info!("自动翻译片名: {} -> {}", file_id, original_title);
    // Try DeepSeek first, then Google
    let deepseek_key = crate::services::secure_config::get_secure_config("deepseek_api_key")
        .ok().flatten().unwrap_or_default();
    let cn = if !deepseek_key.is_empty() {
        translate_via_deepseek(original_title, &deepseek_key)
    } else {
        None
    }.or_else(|| translate_via_google(original_title));

    if let Some(cn) = cn {
        if !cn.is_empty() && cn != original_title {
            let _ = db::with_db(|conn| {
                conn.execute("UPDATE movies SET chinese_name=?1 WHERE file_id=?2",
                    rusqlite::params![cn, file_id])?;
                Ok(())
            });
            log::info!("自动翻译完成: {} -> {}", original_title, cn);
        }
    }
}

fn translate_via_deepseek(text: &str, api_key: &str) -> Option<String> {
    let client = reqwest::blocking::Client::new();
    let body = serde_json::json!({
        "model": "deepseek-chat",
        "messages": [
            {"role": "system", "content": "你是日本AV影片片名本地化翻译专家。将用户输入的日文片名改写成简体中文。\n\n规则：\n1. 完全按中文母语者阅读习惯改写，不要保留日语句式（如'被放置'、'てしまう'等）。\n2. 语序改为中文叙事顺序，通常先说结果或主视角动作（如'发现……'、'妻子被……'）。\n3. 使用AV标题常用中文词汇：'丈夫'或'家暴男'、'人妻'、'全裸'、'鬼鬼祟祟'、'视而不见'。\n4. 保留原文的感叹号、问号、省略号，强化冲击力和悬念感。\n5. 不要直译，不要出现过多'～了'，不要有日语腔。\n\n只返回翻译结果，不要任何解释或评价。"},
            {"role": "user", "content": text}
        ],
        "temperature": 0.3
    });
    match client.post("https://api.deepseek.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(15))
        .send()
    {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>() {
                json["choices"][0]["message"]["content"].as_str().map(|s| s.trim().to_string())
            } else { None }
        }
        Err(e) => { log::warn!("DeepSeek翻译失败: {}", e); None }
    }
}

fn translate_via_google(text: &str) -> Option<String> {
    let url = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl=zh-CN&dt=t&q={}",
        encode(text)
    );
    match reqwest::blocking::get(&url) {
        Ok(resp) => {
            if let Ok(body) = resp.text() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                    json[0][0][0].as_str().map(|s| s.to_string())
                } else { None }
            } else { None }
        }
        Err(e) => { log::warn!("Google翻译失败: {}", e); None }
    }
}

// ─── TMDB (blocking HTTP) ───

fn scrape_tmdb(query: &str) -> Result<ScrapeResult, CommandError> {
    let api_key = crate::services::secure_config::get_secure_config("tmdb_api_key").ok().flatten()
        .or_else(|| db::with_db(|conn| crate::db::queries::get_config(conn, "tmdb_api_key")).ok().flatten())
        .or_else(|| std::env::var("TMDB_API_KEY").ok())
        .ok_or_else(|| CommandError::scrape_failed("TMDB API Key未配置，请在设置→刮削源中填入"))?;

    log::info!("TMDB 搜索: query={}", query);
    let url = format!("https://api.themoviedb.org/3/search/movie?api_key={}&query={}&language=zh-CN", api_key, encode(query));
    let resp = get_external_client().get(&url).send().map_err(|e| {
        log::warn!("TMDB HTTP错误: {}", e);
        CommandError::network(&e.to_string())
    })?;
    let json: serde_json::Value = resp.json().map_err(|e| CommandError::network(&e.to_string()))?;
    let results = json["results"].as_array().ok_or_else(|| CommandError::scrape_failed("TMDB无结果"))?;
    if results.is_empty() { return Err(CommandError::scrape_failed("TMDB无匹配")); }

    let item = &results[0];
    let tmdb_id = item["id"].as_i64().unwrap_or(0);
    let title = item["title"].as_str().unwrap_or(query).to_string();
    let year = item["release_date"].as_str().and_then(|d| d[..4].parse().ok());
    let overview = item["overview"].as_str().map(|s| s.to_string());
    let rating = item["vote_average"].as_f64();
    let poster = item["poster_path"].as_str().map(|p| format!("https://image.tmdb.org/t/p/w500{}", p));
    let backdrop = item["backdrop_path"].as_str().map(|p| format!("https://image.tmdb.org/t/p/w1280{}", p));
    let genre_ids: Vec<i64> = item["genre_ids"].as_array().map(|a| a.iter().filter_map(|v| v.as_i64()).collect()).unwrap_or_default();

    let mut director = None;
    let mut actors = Vec::new();
    if tmdb_id > 0 {
        let cu = format!("https://api.themoviedb.org/3/movie/{}/credits?api_key={}&language=zh-CN", tmdb_id, api_key);
        if let Ok(resp) = get_external_client().get(&cu).send() {
            if let Ok(cj) = resp.json::<serde_json::Value>() {
                if let Some(crew) = cj["crew"].as_array() {
                    for c in crew { if c["job"].as_str() == Some("Director") { director = c["name"].as_str().map(|s| s.to_string()); break; } }
                }
                if let Some(cast) = cj["cast"].as_array() {
                    for c in cast.iter().take(5) { if let Some(n) = c["name"].as_str() { actors.push(n.to_string()); } }
                }
            }
        }
    }

    let genre_names = if !genre_ids.is_empty() {
        let gu = format!("https://api.themoviedb.org/3/genre/movie/list?api_key={}&language=zh-CN", api_key);
        get_external_client().get(&gu).send().ok().and_then(|r| r.json::<serde_json::Value>().ok()).and_then(|gj|
            gj["genres"].as_array().map(|a| a.iter().filter_map(|g|
                if genre_ids.contains(&g["id"].as_i64().unwrap_or(0)) { g["name"].as_str().map(|s| s.to_string()) } else { None }
            ).collect())
        )
    } else { None };

    Ok(ScrapeResult { source: "tmdb".into(), title, year, poster_url: poster, backdrop_url: backdrop, overview, rating, runtime: None, director, genre: genre_names, actors: Some(actors), score: 80 })
}

// ─── JavBus (blocking HTML parse) ───

fn scrape_javbus(query: &str) -> Result<ScrapeResult, CommandError> {
    let r = crate::services::javbus_scraper::search_javbus(query)?;
    Ok(ScrapeResult {
        source: "javbus".into(), title: r.title, year: r.year,
        poster_url: r.poster_url, backdrop_url: None,
        overview: r.overview, rating: r.rating, runtime: r.runtime,
        director: r.director,
        genre: if r.genres.is_empty() { None } else { Some(r.genres) },
        actors: if r.actors.is_empty() { None } else { Some(r.actors) },
        score: 75,
    })
}

// ─── JAV321 (ported from MetaTube Go) ───
fn scrape_jav321(query: &str) -> Result<ScrapeResult, CommandError> {
    let r = crate::services::jav321_scraper::search_jav321(query)?;
    Ok(ScrapeResult {
        source: "jav321".into(), title: r.title, year: r.year,
        poster_url: r.poster_url, backdrop_url: None,
        overview: r.overview, rating: r.rating, runtime: r.runtime,
        director: None,
        genre: if r.genres.is_empty() { None } else { Some(r.genres) },
        actors: if r.actors.is_empty() { None } else { Some(r.actors) },
        score: 70,
    })
}

// ─── JavDB (blocking HTML parse) ───

fn scrape_javdb(query: &str) -> Result<ScrapeResult, CommandError> {
    let code = query.to_uppercase().replace(['-', '_', ' '], "");
    let url = format!("https://javdb.com/search?q={}&f=all", code);
    let resp = get_external_client().get(&url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    if !resp.status().is_success() { return Err(CommandError::scrape_failed("JavDB搜索失败")); }
    let html = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&html);

    let s_item = scraper::Selector::parse(".movie-list .item a.box").unwrap();
    let detail_url = doc.select(&s_item).next().and_then(|e| e.value().attr("href").map(|s| format!("https://javdb.com{}", s)));
    let detail_url = match detail_url { Some(u) => u, None => return Err(CommandError::scrape_failed("JavDB无结果")) };

    let resp = get_external_client().get(&detail_url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    let html = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&html);

    let s_title = scraper::Selector::parse(".video-title strong").unwrap();
    let s_cover = scraper::Selector::parse(".video-cover img").unwrap();
    let s_panel = scraper::Selector::parse(".panel-block").unwrap();
    let s_actor = scraper::Selector::parse(".actress-name a").unwrap();

    let title = doc.select(&s_title).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_else(|| query.to_string());
    let poster = doc.select(&s_cover).next().and_then(|e| e.value().attr("src").map(|s| s.to_string()));
    let mut year = None; let mut director = None; let mut runtime = None;
    for b in doc.select(&s_panel) {
        let t = b.text().collect::<String>();
        if t.contains("日期:") { year = t.split("日期:").nth(1).and_then(|d| d.trim()[..4].parse().ok()); }
        if t.contains("導演:") || t.contains("导演:") { director = t.split("導演:").nth(1).or_else(|| t.split("导演:").nth(1)).map(|s| s.trim().to_string()); }
        if t.contains("時長:") || t.contains("时长:") { runtime = t.split("時長:").nth(1).or_else(|| t.split("时长:").nth(1)).and_then(|s| s.trim().parse().ok()); }
    }
    let actors: Vec<String> = doc.select(&s_actor).filter_map(|a| { let n = a.text().collect::<String>().trim().to_string(); if n.is_empty() { None } else { Some(n) } }).collect();

    Ok(ScrapeResult { source: "javdb".into(), title, year, poster_url: poster, backdrop_url: None, overview: None, rating: None, runtime, director, genre: None, actors: if actors.is_empty() { None } else { Some(actors) }, score: 70 })
}

// ─── IMDb (blocking HTML parse) ───

fn scrape_imdb(query: &str) -> Result<ScrapeResult, CommandError> {
    let url = format!("https://www.imdb.com/find/?q={}", encode(query));
    let resp = get_external_client().get(&url).header("Accept-Language", "en-US,en;q=0.9").send().map_err(|e| CommandError::network(&e.to_string()))?;
    let html = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&html);
    let s_item = scraper::Selector::parse(".ipc-metadata-list-summary-item__t a").unwrap();
    let detail_url = doc.select(&s_item).next().and_then(|e| e.value().attr("href"));
    if detail_url.is_none() { return Err(CommandError::scrape_failed("IMDb无结果")); }
    let detail_url = format!("https://www.imdb.com{}", detail_url.unwrap());
    let resp = get_external_client().get(&detail_url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    let html = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&html);
    let s_title = scraper::Selector::parse("h1").unwrap();
    let s_year = scraper::Selector::parse(".sc-afe43def-1 a.ipc-inline-list__item").unwrap();
    let s_rating = scraper::Selector::parse(".sc-bde20123-1 span").unwrap();
    let s_poster = scraper::Selector::parse(".ipc-media img").unwrap();
    let title = doc.select(&s_title).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_else(|| query.to_string());
    let year = doc.select(&s_year).next().and_then(|e| e.text().collect::<String>().trim().parse().ok());
    let rating = doc.select(&s_rating).next().and_then(|e| e.text().collect::<String>().trim().parse().ok());
    let poster = doc.select(&s_poster).next().and_then(|e| e.value().attr("src").map(|s| s.to_string()));
    Ok(ScrapeResult { source: "imdb".into(), title, year, poster_url: poster, backdrop_url: None, overview: None, rating, runtime: None, director: None, genre: None, actors: None, score: 75 })
}

// ─── Douban (blocking HTML parse) ───

fn scrape_douban(query: &str) -> Result<ScrapeResult, CommandError> {
    let url = format!("https://movie.douban.com/subject_search?search_text={}", encode(query));
    let resp = get_external_client().get(&url).header("Accept-Language", "zh-CN,zh;q=0.9").send().map_err(|e| CommandError::network(&e.to_string()))?;
    if !resp.status().is_success() { return Err(CommandError::scrape_failed("豆瓣不可用")); }
    let html = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&html);
    let s_item = scraper::Selector::parse(".item-root a.cover-link").unwrap();
    let detail_url = doc.select(&s_item).next().and_then(|e| e.value().attr("href"));
    if detail_url.is_none() { return Err(CommandError::scrape_failed("豆瓣无结果")); }
    let detail_url = format!("https://movie.douban.com{}", detail_url.unwrap().split('?').next().unwrap_or(""));
    let resp = get_external_client().get(&detail_url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    let html = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&html);
    let s_title = scraper::Selector::parse("h1 span").unwrap();
    let s_year = scraper::Selector::parse(".year").unwrap();
    let s_rating = scraper::Selector::parse(".rating_num").unwrap();
    let s_poster = scraper::Selector::parse("#mainpic img").unwrap();
    let s_overview = scraper::Selector::parse("#link-report span[property='v:summary']").unwrap();
    let title = doc.select(&s_title).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_else(|| query.to_string());
    let year = doc.select(&s_year).next().and_then(|e| e.text().collect::<String>().trim().replace(['(', ')'], "").parse().ok());
    let rating = doc.select(&s_rating).next().and_then(|e| e.text().collect::<String>().trim().parse().ok());
    let poster = doc.select(&s_poster).next().and_then(|e| e.value().attr("src").map(|s| s.to_string()));
    let overview = doc.select(&s_overview).next().map(|e| e.text().collect::<String>().trim().to_string());
    Ok(ScrapeResult { source: "douban".into(), title, year, poster_url: poster, backdrop_url: None, overview, rating, runtime: None, director: None, genre: None, actors: None, score: 70 })
}

// ─── JavLibrary (blocking HTML parse) ───

fn scrape_javlib(query: &str) -> Result<ScrapeResult, CommandError> {
    let code = query.to_uppercase().replace(['-', '_', ' '], "");
    let url = format!("https://www.javlibrary.com/en/vl_searchbyid.php?keyword={}", code);
    let resp = get_external_client().get(&url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    if !resp.status().is_success() { return Err(CommandError::scrape_failed("JavLibrary不可用")); }
    let html = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&html);
    let s_item = scraper::Selector::parse(".video a").unwrap();
    let detail_url = doc.select(&s_item).next().and_then(|e| e.value().attr("href"));
    if detail_url.is_none() { return Err(CommandError::scrape_failed("JavLibrary无结果")); }
    let detail_url = format!("https://www.javlibrary.com{}", detail_url.unwrap().trim_start_matches('.'));
    let resp = get_external_client().get(&detail_url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    let html = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&html);
    let s_title = scraper::Selector::parse("h3.post-title").unwrap();
    let s_cover = scraper::Selector::parse("#video_jacket_img").unwrap();
    let s_info = scraper::Selector::parse("#video_info .item").unwrap();
    let s_actor = scraper::Selector::parse(".cast a").unwrap();
    let s_genre = scraper::Selector::parse(".genre a").unwrap();
    let title = doc.select(&s_title).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_else(|| query.to_string());
    let poster = doc.select(&s_cover).next().and_then(|e| e.value().attr("src").map(|s| s.to_string()));
    let mut year = None; let mut director = None; let mut runtime = None;
    for item in doc.select(&s_info) {
        let t = item.text().collect::<String>();
        if t.contains("Release Date:") { year = t.split("Release Date:").nth(1).and_then(|d| d.trim().split('/').next().and_then(|y| y.trim().parse().ok())); }
        if t.contains("Director:") { director = t.split("Director:").nth(1).map(|s| s.trim().to_string()); }
        if t.contains("Length:") { runtime = t.split("Length:").nth(1).and_then(|s| s.trim().split_whitespace().next().and_then(|n| n.parse().ok())); }
    }
    let actors: Vec<String> = doc.select(&s_actor).filter_map(|a| { let n = a.text().collect::<String>().trim().to_string(); if n.is_empty() { None } else { Some(n) } }).collect();
    let genres: Vec<String> = doc.select(&s_genre).filter_map(|g| { let n = g.text().collect::<String>().trim().to_string(); if n.is_empty() { None } else { Some(n) } }).collect();
    Ok(ScrapeResult { source: "javlibrary".into(), title, year, poster_url: poster, backdrop_url: None, overview: None, rating: None, runtime, director, genre: if genres.is_empty() { None } else { Some(genres) }, actors: if actors.is_empty() { None } else { Some(actors) }, score: 70 })
}

// ─── Fanza/DMM (blocking HTML parse) ───

fn scrape_fanza(query: &str) -> Result<ScrapeResult, CommandError> {
    let code = query.to_uppercase().replace(['-', '_', ' '], "");
    let url = format!("https://www.dmm.co.jp/mono/dvd/-/search/=/searchstr={}", encode(&code));
    let resp = get_external_client().get(&url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    if !resp.status().is_success() { return Err(CommandError::scrape_failed("Fanza不可用")); }
    let html = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&html);
    let s_item = scraper::Selector::parse(".tmb a").unwrap();
    let detail_url = doc.select(&s_item).next().and_then(|e| e.value().attr("href"));
    if detail_url.is_none() {
        log::warn!("Fanza 选择器.tmb a未匹配, 页面{}字节, 前200字: {}", html.len(), &html[..html.len().min(200)]);
        return Err(CommandError::scrape_failed("Fanza无结果"));
    }
    let detail_url = detail_url.unwrap().to_string();
    let resp = get_external_client().get(&detail_url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    let html = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&html);
    let s_title = scraper::Selector::parse("h1#title").unwrap();
    let s_cover = scraper::Selector::parse("#sample-image1 img").unwrap();
    let s_actor = scraper::Selector::parse("#performer a").unwrap();
    let s_genre = scraper::Selector::parse(".genreTag a").unwrap();
    let s_info = scraper::Selector::parse("table.mg-b20 tr").unwrap();
    let title = doc.select(&s_title).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_else(|| query.to_string());
    let poster = doc.select(&s_cover).next().and_then(|e| e.value().attr("src").map(|s| s.to_string()));
    let mut year = None; let mut runtime = None;
    for tr in doc.select(&s_info) {
        let t = tr.text().collect::<String>();
        if t.contains("発売日") || t.contains("配信開始日") { year = t.split_whitespace().last().and_then(|d| d[..4].parse().ok()); }
        if t.contains("収録時間") { runtime = t.split_whitespace().last().and_then(|s| s.trim().replace("min", "").parse().ok()); }
    }
    let actors: Vec<String> = doc.select(&s_actor).filter_map(|a| { let n = a.text().collect::<String>().trim().to_string(); if n.is_empty() { None } else { Some(n) } }).collect();
    let genres: Vec<String> = doc.select(&s_genre).filter_map(|g| { let n = g.text().collect::<String>().trim().to_string(); if n.is_empty() { None } else { Some(n) } }).collect();
    Ok(ScrapeResult { source: "fanza".into(), title, year, poster_url: poster, backdrop_url: None, overview: None, rating: None, runtime, director: None, genre: if genres.is_empty() { None } else { Some(genres) }, actors: if actors.is_empty() { None } else { Some(actors) }, score: 65 })
}

// ─── Arzon (blocking HTML parse) ───

fn scrape_arzon(query: &str) -> Result<ScrapeResult, CommandError> {
    let code = query.to_uppercase().replace(['-', '_', ' '], "");
    let url = format!("https://www.arzon.jp/itemlist.html?q={}", encode(&code));
    let resp = get_external_client().get(&url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    if !resp.status().is_success() { return Err(CommandError::scrape_failed("Arzon不可用")); }
    let html = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&html);
    let s_item = scraper::Selector::parse(".item img").unwrap();
    let detail = doc.select(&s_item).next().and_then(|e| e.value().attr("alt").map(|s| s.to_string()));
    if detail.is_none() { return Err(CommandError::scrape_failed("Arzon无结果")); }
    let title = detail.unwrap();
    let s_cover = scraper::Selector::parse(".item img").unwrap();
    let poster = doc.select(&s_cover).next().and_then(|e| e.value().attr("src").map(|s| format!("https:{}", s)));
    Ok(ScrapeResult { source: "arzon".into(), title, year: None, poster_url: poster, backdrop_url: None, overview: None, rating: None, runtime: None, director: None, genre: None, actors: None, score: 50 })
}

// ─── MGStage (blocking HTML parse) ───

fn scrape_mgstage(query: &str) -> Result<ScrapeResult, CommandError> {
    let code = query.to_uppercase().replace(['-', '_', ' '], "");
    let url = format!("https://www.mgstage.com/search/cSearch.php?search_word={}", encode(&code));
    let resp = get_external_client().get(&url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    if !resp.status().is_success() { return Err(CommandError::scrape_failed("MGStage不可用")); }
    let html = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&html);
    let s_item = scraper::Selector::parse(".search_list h2 a").unwrap();
    let detail_url = doc.select(&s_item).next().and_then(|e| e.value().attr("href"));
    if detail_url.is_none() { return Err(CommandError::scrape_failed("MGStage无结果")); }
    let detail_url = detail_url.unwrap().to_string();
    let resp = get_external_client().get(&detail_url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    let html = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&html);
    let s_title = scraper::Selector::parse("h1.tag").unwrap();
    let s_cover = scraper::Selector::parse("#EnlargeImage").unwrap();
    let s_actor = scraper::Selector::parse(".detail_data a[href*='actress']").unwrap();
    let s_genre = scraper::Selector::parse(".detail_data a[href*='genre']").unwrap();
    let s_info = scraper::Selector::parse(".detail_data tr").unwrap();
    let title = doc.select(&s_title).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_else(|| query.to_string());
    let poster = doc.select(&s_cover).next().and_then(|e| e.value().attr("href").map(|s| s.to_string()));
    let mut year = None; let mut runtime = None;
    for tr in doc.select(&s_info) {
        let t = tr.text().collect::<String>();
        if t.contains("発売日") { year = t.split_whitespace().last().and_then(|d| d[..4].parse().ok()); }
        if t.contains("収録時間") { runtime = t.split_whitespace().last().and_then(|s| s.replace("分", "").trim().parse().ok()); }
    }
    let actors: Vec<String> = doc.select(&s_actor).filter_map(|a| { let n = a.text().collect::<String>().trim().to_string(); if n.is_empty() { None } else { Some(n) } }).collect();
    let genres: Vec<String> = doc.select(&s_genre).filter_map(|g| { let n = g.text().collect::<String>().trim().to_string(); if n.is_empty() { None } else { Some(n) } }).collect();
    Ok(ScrapeResult { source: "mgstage".into(), title, year, poster_url: poster, backdrop_url: None, overview: None, rating: None, runtime, director: None, genre: if genres.is_empty() { None } else { Some(genres) }, actors: if actors.is_empty() { None } else { Some(actors) }, score: 60 })
}

// ─── FC2 (blocking HTML parse) ───

fn scrape_fc2(query: &str) -> Result<ScrapeResult, CommandError> {
    let code = query.to_uppercase().replace(['-', '_', ' '], "");
    // FC2 uses numeric IDs
    if !code.chars().all(|c| c.is_ascii_digit()) {
        return Err(CommandError::scrape_failed("FC2需要纯数字ID"));
    }
    let url = format!("https://adult.contents.fc2.com/article/{}/", code);
    let resp = get_external_client().get(&url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    if !resp.status().is_success() { return Err(CommandError::scrape_failed("FC2未找到")); }
    let html = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&html);
    let s_title = scraper::Selector::parse("h3.items_article_headerInfo_title").unwrap();
    let s_cover = scraper::Selector::parse(".items_article_Left img").unwrap();
    let s_info = scraper::Selector::parse(".items_article_Info tr").unwrap();
    let title = doc.select(&s_title).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_else(|| query.to_string());
    let poster = doc.select(&s_cover).next().and_then(|e| e.value().attr("src").map(|s| s.to_string()));
    let mut year = None;
    for tr in doc.select(&s_info) {
        let t = tr.text().collect::<String>();
        if t.contains("販売開始日") { year = t.split_whitespace().last().and_then(|d| d[..4].parse().ok()); }
    }
    Ok(ScrapeResult { source: "fc2".into(), title, year, poster_url: poster, backdrop_url: None, overview: None, rating: None, runtime: None, director: None, genre: None, actors: None, score: 55 })
}

// ─── JpHoo (JSON API) ───

fn scrape_jphoo(query: &str) -> Result<ScrapeResult, CommandError> {
    let code = query.trim().to_uppercase().replace(['-', '_', ' '], "");

    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis().to_string();
    let nonce = (1_000_000_000u64 + rand::random::<u64>() % 9_000_000_000).to_string();

    let secret = db::with_db(|c| crate::db::queries::get_config(c, "jphoo_secret")).ok().flatten().unwrap_or_default().trim().to_string();
    let refreshtoken = db::with_db(|c| crate::db::queries::get_config(c, "jphoo_refreshtoken")).ok().flatten().unwrap_or_default().trim().to_string();
    let guest_id = db::with_db(|c| crate::db::queries::get_config(c, "jphoo_guestid")).ok().flatten().unwrap_or_default().trim().to_string();

    if secret.is_empty() || refreshtoken.is_empty() || guest_id.is_empty() {
        return Err(CommandError::scrape_failed("JpHoo未配置secret/refreshtoken/guestid，请在设置→刮削源中配置"));
    }

    let url = format!("https://www.jphoo1.com/prod-api/v2/search/list?pageNum=1&pageSize=24&operationName=works&keyword={}", code);
    log::info!("JpHoo ══════════════════════════════════════");
    log::info!("JpHoo URL: {}", url);
    log::info!("JpHoo 请求头:");
    log::info!("  guestid: {}", guest_id);
    log::info!("  istoken: true");
    log::info!("  loading: true");
    log::info!("  nonce: {}", nonce);
    log::info!("  priority: u=1, i");
    log::info!("  refreshtoken: {}", refreshtoken);
    log::info!("  secret: {}", secret);
    log::info!("  timestamp: {}", ts);
    log::info!("JpHoo ══════════════════════════════════════");

    let resp = get_external_client().get(&url)
        .header("guestid", &guest_id)
        .header("istoken", "true")
        .header("loading", "true")
        .header("nonce", &nonce)
        .header("priority", "u=1, i")
        .header("refreshtoken", &refreshtoken)
        .header("secret", &secret)
        .header("timestamp", &ts)
        .send()
        .map_err(|e| CommandError::network(&e.to_string()))?;
    let body = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    log::info!("JpHoo API响应: {}", truncate_log(&body, 500));

    let json_str = body.find('{').map(|i| &body[i..]).unwrap_or(&body);
    let json: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| CommandError::scrape_failed(&format!("JpHoo JSON解析失败: {}", e)))?;

    let list = json["data"]["list"].as_array()
        .or_else(|| json["data"]["rows"].as_array())
        .or_else(|| json["data"].as_array())
        .ok_or_else(|| CommandError::scrape_failed("JpHoo返回格式异常"))?;

    if list.is_empty() { return Err(CommandError::scrape_failed("JpHoo无结果")); }

    let item = &list[0];
    let title = item["title"].as_str()
        .or_else(|| item["name"].as_str())
        .unwrap_or(query).to_string();
    let year = item["releaseDate"].as_str().or_else(|| item["year"].as_str())
        .and_then(|d| d[..4].parse().ok());
    let poster = item["cover"].as_str().or_else(|| item["img"].as_str()).or_else(|| item["image"].as_str())
        .map(|s| if s.starts_with("http") { s.to_string() } else { format!("https://www.jphoo1.com{}", s) });
    let overview = item["description"].as_str().map(|s| s.to_string());
    let runtime = item["duration"].as_i64().map(|v| v as i32);
    let mut actors = Vec::new();
    if let Some(arr) = item["actors"].as_array() {
        for a in arr { if let Some(n) = a["name"].as_str() { actors.push(n.to_string()); } }
    }
    if let Some(arr) = item["actressList"].as_array() {
        for a in arr { if let Some(n) = a.as_str() { actors.push(n.to_string()); } }
    }
    let mut genres = Vec::new();
    if let Some(arr) = item["tags"].as_array() {
        for t in arr { if let Some(n) = t.as_str() { genres.push(n.to_string()); } }
    }

    Ok(ScrapeResult {
        source: "jphoo".into(), title, year, poster_url: poster, backdrop_url: None,
        overview, rating: None, runtime, director: None,
        genre: if genres.is_empty() { None } else { Some(genres) },
        actors: if actors.is_empty() { None } else { Some(actors) },
        score: 70,
    })
}

// ─── Airav ───
fn scrape_airav(query: &str) -> Result<ScrapeResult, CommandError> {
    let code = query.trim().to_uppercase().replace(['-', '_', ' '], "");
    let url = format!("https://www.airav.wiki/search?q={}", code);
    let resp = get_external_client().get(&url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    let body = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&body);
    let sel_title = scraper::Selector::parse(".video-title, .item-title, h2 a").unwrap();
    let sel_poster = scraper::Selector::parse(".video-cover img, .item-cover img, .poster img").unwrap();
    let sel_actors = scraper::Selector::parse(".actress a, .actor a, .star a").unwrap();
    let title = doc.select(&sel_title).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_else(|| query.to_string());
    let poster = doc.select(&sel_poster).next().and_then(|e| e.attr("src").map(|s| s.to_string()));
    let actors: Vec<String> = doc.select(&sel_actors).map(|e| e.text().collect::<String>().trim().to_string()).filter(|n| !n.is_empty()).collect();
    Ok(ScrapeResult { source: "airav".into(), title, year: None, poster_url: poster, backdrop_url: None, overview: None, rating: None, runtime: None, director: None, genre: None, actors: if actors.is_empty() { None } else { Some(actors) }, score: 50 })
}

// ─── XCITY ───
fn scrape_xcity(query: &str) -> Result<ScrapeResult, CommandError> {
    let code = query.trim().to_uppercase().replace(['-', '_', ' '], "");
    let url = format!("https://www.xcity.jp/avod/list/?keyword={}", code);
    let resp = get_external_client().get(&url).header("Referer", "https://www.xcity.jp/").send().map_err(|e| CommandError::network(&e.to_string()))?;
    let body = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&body);
    let sel_title = scraper::Selector::parse(".itemTitle a, .videoTitle, .titleArea h3").unwrap();
    let sel_poster = scraper::Selector::parse(".itemPhoto img, .videoPhoto img, .packageImage img").unwrap();
    let sel_actors = scraper::Selector::parse(".actressName a, .performer a, .starName").unwrap();
    let title = doc.select(&sel_title).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_else(|| query.to_string());
    let poster = doc.select(&sel_poster).next().and_then(|e| e.attr("src").map(|s| if s.starts_with("http") { s.to_string() } else { format!("https:{}", s) }));
    let actors: Vec<String> = doc.select(&sel_actors).map(|e| e.text().collect::<String>().trim().to_string()).filter(|n| !n.is_empty()).collect();
    Ok(ScrapeResult { source: "xcity".into(), title, year: None, poster_url: poster, backdrop_url: None, overview: None, rating: None, runtime: None, director: None, genre: None, actors: if actors.is_empty() { None } else { Some(actors) }, score: 45 })
}

// ─── Prestige ───
fn scrape_prestige(query: &str) -> Result<ScrapeResult, CommandError> {
    let code = query.trim().to_uppercase().replace(['-', '_', ' '], "");
    let url = format!("https://www.prestige-av.com/goods/goods_list.php?search_word={}", code);
    let resp = get_external_client().get(&url).header("Referer", "https://www.prestige-av.com/").send().map_err(|e| CommandError::network(&e.to_string()))?;
    let body = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&body);
    let sel_title = scraper::Selector::parse(".goods_name, .goods-title, .item-name a").unwrap();
    let sel_poster = scraper::Selector::parse(".goods_image img, .goods-photo img, .item-img img").unwrap();
    let sel_actors = scraper::Selector::parse(".actress_name a, .performer a, .cast_name").unwrap();
    let title = doc.select(&sel_title).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_else(|| query.to_string());
    let poster = doc.select(&sel_poster).next().and_then(|e| e.attr("src").map(|s| if s.starts_with("http") { s.to_string() } else { format!("https://www.prestige-av.com{}", s) }));
    let actors: Vec<String> = doc.select(&sel_actors).map(|e| e.text().collect::<String>().trim().to_string()).filter(|n| !n.is_empty()).collect();
    Ok(ScrapeResult { source: "prestige".into(), title, year: None, poster_url: poster, backdrop_url: None, overview: None, rating: None, runtime: None, director: None, genre: None, actors: if actors.is_empty() { None } else { Some(actors) }, score: 45 })
}

// ─── Avsox ───
fn scrape_avsox(query: &str) -> Result<ScrapeResult, CommandError> {
    let code = query.trim().to_uppercase().replace(['-', '_', ' '], "");
    let url = format!("https://avsox.cyou/cn/search/{}", code);
    let resp = get_external_client().get(&url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    let body = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&body);
    let sel_title = scraper::Selector::parse(".movie-title h3, .video-title, .item-title a").unwrap();
    let sel_poster = scraper::Selector::parse(".movie-cover img, .video-img img, .thumb img").unwrap();
    let sel_actors = scraper::Selector::parse(".star-name a, .actress a, .cast span").unwrap();
    let title = doc.select(&sel_title).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_else(|| query.to_string());
    let poster = doc.select(&sel_poster).next().and_then(|e| e.attr("src").map(|s| s.to_string()));
    let actors: Vec<String> = doc.select(&sel_actors).map(|e| e.text().collect::<String>().trim().to_string()).filter(|n| !n.is_empty()).collect();
    Ok(ScrapeResult { source: "avsox".into(), title, year: None, poster_url: poster, backdrop_url: None, overview: None, rating: None, runtime: None, director: None, genre: None, actors: if actors.is_empty() { None } else { Some(actors) }, score: 50 })
}

// ─── Njav ───
fn scrape_njav(query: &str) -> Result<ScrapeResult, CommandError> {
    let code = query.trim().to_uppercase().replace(['-', '_', ' '], "");
    let url = format!("https://njav.tv/search/{}", code);
    let resp = get_external_client().get(&url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    let body = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&body);
    let sel_title = scraper::Selector::parse(".video-title, .movie-title, h2 a, .item-title").unwrap();
    let sel_poster = scraper::Selector::parse(".video-img img, .poster img, .thumb img").unwrap();
    let sel_actors = scraper::Selector::parse(".actress-name, .star a, .actor-tag").unwrap();
    let sel_genres = scraper::Selector::parse(".tag a, .category a, .genre span").unwrap();
    let title = doc.select(&sel_title).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_else(|| query.to_string());
    let poster = doc.select(&sel_poster).next().and_then(|e| e.attr("src").map(|s| s.to_string()));
    let actors: Vec<String> = doc.select(&sel_actors).map(|e| e.text().collect::<String>().trim().to_string()).filter(|n| !n.is_empty()).collect();
    let genres: Vec<String> = doc.select(&sel_genres).map(|e| e.text().collect::<String>().trim().to_string()).filter(|g| !g.is_empty()).collect();
    Ok(ScrapeResult { source: "njav".into(), title, year: None, poster_url: poster, backdrop_url: None, overview: None, rating: None, runtime: None, director: None, genre: if genres.is_empty() { None } else { Some(genres) }, actors: if actors.is_empty() { None } else { Some(actors) }, score: 45 })
}

// ─── GetAV ───
fn scrape_getav(query: &str) -> Result<ScrapeResult, CommandError> {
    let code = query.trim().to_uppercase().replace(['-', '_', ' '], "");
    let url = format!("https://getav.info/search?keyword={}", code);
    let resp = get_external_client().get(&url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    let body = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&body);
    let sel_title = scraper::Selector::parse(".video-title, .movie-title, h3 a, .item-name").unwrap();
    let sel_poster = scraper::Selector::parse(".video-cover img, .poster-img img, .thumb img").unwrap();
    let sel_actors = scraper::Selector::parse(".actress a, .star-name, .cast-item").unwrap();
    let title = doc.select(&sel_title).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_else(|| query.to_string());
    let poster = doc.select(&sel_poster).next().and_then(|e| e.attr("src").map(|s| s.to_string()));
    let actors: Vec<String> = doc.select(&sel_actors).map(|e| e.text().collect::<String>().trim().to_string()).filter(|n| !n.is_empty()).collect();
    Ok(ScrapeResult { source: "getav".into(), title, year: None, poster_url: poster, backdrop_url: None, overview: None, rating: None, runtime: None, director: None, genre: None, actors: if actors.is_empty() { None } else { Some(actors) }, score: 45 })
}

// ─── WhosTV ───
fn scrape_whostv(query: &str) -> Result<ScrapeResult, CommandError> {
    let code = query.trim().to_uppercase().replace(['-', '_', ' '], "");
    let url = format!("https://whostv.net/search?keyword={}", code);
    let resp = get_external_client().get(&url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    let body = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&body);
    let sel_title = scraper::Selector::parse(".video-title, .movie-title, h2 a, .item-title").unwrap();
    let sel_poster = scraper::Selector::parse(".video-img img, .poster img, .cover-img img").unwrap();
    let sel_actors = scraper::Selector::parse(".actress-name, .star a, .performer-name").unwrap();
    let title = doc.select(&sel_title).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_else(|| query.to_string());
    let poster = doc.select(&sel_poster).next().and_then(|e| e.attr("src").map(|s| s.to_string()));
    let actors: Vec<String> = doc.select(&sel_actors).map(|e| e.text().collect::<String>().trim().to_string()).filter(|n| !n.is_empty()).collect();
    Ok(ScrapeResult { source: "whostv".into(), title, year: None, poster_url: poster, backdrop_url: None, overview: None, rating: None, runtime: None, director: None, genre: None, actors: if actors.is_empty() { None } else { Some(actors) }, score: 40 })
}

// ─── FC2PPVDB ───
fn scrape_fc2ppvdb(query: &str) -> Result<ScrapeResult, CommandError> {
    let code = query.trim().to_uppercase().replace(['-', '_', ' '], "");
    let url = format!("https://fc2ppvdb.com/search?keyword={}", code);
    let resp = get_external_client().get(&url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    let body = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&body);
    let sel_title = scraper::Selector::parse(".video-title, .item-title a, .movie-title, h2").unwrap();
    let sel_poster = scraper::Selector::parse(".video-img img, .thumbnail img, .cover img").unwrap();
    let sel_actors = scraper::Selector::parse(".actress a, .actor-name, .seller-name").unwrap();
    let sel_desc = scraper::Selector::parse(".description, .video-desc, .item-desc").unwrap();
    let title = doc.select(&sel_title).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_else(|| query.to_string());
    let poster = doc.select(&sel_poster).next().and_then(|e| e.attr("src").map(|s| s.to_string()));
    let actors: Vec<String> = doc.select(&sel_actors).map(|e| e.text().collect::<String>().trim().to_string()).filter(|n| !n.is_empty()).collect();
    let overview = doc.select(&sel_desc).next().map(|e| e.text().collect::<String>().trim().to_string());
    Ok(ScrapeResult { source: "fc2ppvdb".into(), title, year: None, poster_url: poster, backdrop_url: None, overview, rating: None, runtime: None, director: None, genre: None, actors: if actors.is_empty() { None } else { Some(actors) }, score: 45 })
}

// ─── FALENO ───
fn scrape_faleno(query: &str) -> Result<ScrapeResult, CommandError> {
    let code = query.trim().to_lowercase().replace(['-', '_', ' '], "");
    let url = format!("https://faleno.jp/top/works/{}/", code);
    let resp = get_external_client().get(&url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    let body = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    if body.len() < 500 { return Err(CommandError::scrape_failed("FALENO 无结果")); }
    let doc = scraper::Html::parse_document(&body);
    let title = doc.select(&scraper::Selector::parse("h1, .works-title, .p-work__title, title").unwrap()).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_else(|| query.to_string());
    let poster = doc.select(&scraper::Selector::parse("img").unwrap()).filter_map(|e| e.attr("src")).find(|s| s.contains("works") || s.contains("package")).map(|s| s.to_string());
    let actors: Vec<String> = doc.select(&scraper::Selector::parse("a").unwrap()).filter_map(|e| { let h = e.value().attr("href").unwrap_or(""); if h.contains("/actress/") { let n = e.text().collect::<String>().trim().to_string(); if n.is_empty() { None } else { Some(n) } } else { None } }).collect();
    Ok(ScrapeResult { source: "faleno".into(), title, year: None, poster_url: poster, backdrop_url: None, overview: None, rating: None, runtime: None, director: None, genre: None, actors: if actors.is_empty() { None } else { Some(actors) }, score: 55 })
}

// ─── DUGA ───
fn scrape_duga(query: &str) -> Result<ScrapeResult, CommandError> {
    let code = query.trim().to_lowercase().replace(['-', '_', ' '], "");
    let url = format!("https://duga.jp/ppv/{}/", code);
    let resp = get_external_client().get(&url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    let body = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    if body.len() < 500 { return Err(CommandError::scrape_failed("DUGA 无结果")); }
    let doc = scraper::Html::parse_document(&body);
    let title = doc.select(&scraper::Selector::parse("h1, .item-title, .p-work__title, title").unwrap()).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_else(|| query.to_string());
    let poster = doc.select(&scraper::Selector::parse("img").unwrap()).filter_map(|e| e.attr("src")).find(|s| s.contains("jacket") || s.contains("package")).map(|s| s.to_string());
    let overview = doc.select(&scraper::Selector::parse("p, .description, .summary").unwrap()).filter_map(|e| { let t = e.text().collect::<String>().trim().to_string(); if t.len() > 50 { Some(t) } else { None } }).next();
    let actors: Vec<String> = doc.select(&scraper::Selector::parse("a").unwrap()).filter_map(|e| { let h = e.value().attr("href").unwrap_or(""); if h.contains("/actress/") { let n = e.text().collect::<String>().trim().to_string(); if n.is_empty() { None } else { Some(n) } } else { None } }).collect();
    Ok(ScrapeResult { source: "duga".into(), title, year: None, poster_url: poster, backdrop_url: None, overview, rating: None, runtime: None, director: None, genre: None, actors: if actors.is_empty() { None } else { Some(actors) }, score: 45 })
}

// ─── SOD ───
fn scrape_sod(query: &str) -> Result<ScrapeResult, CommandError> {
    let code = query.trim().to_lowercase().replace(['-', '_', ' '], "");
    let url = format!("https://ec.sod.co.jp/prime/videos/?id={}", code);
    let resp = get_external_client().get(&url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    let body = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    if body.len() < 500 { return Err(CommandError::scrape_failed("SOD 无结果")); }
    let doc = scraper::Html::parse_document(&body);
    let title = doc.select(&scraper::Selector::parse("h1, .item-title, title").unwrap()).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_else(|| query.to_string());
    let poster = doc.select(&scraper::Selector::parse("img").unwrap()).filter_map(|e| e.attr("src")).find(|s| s.contains("jacket") || s.contains("package")).map(|s| s.to_string());
    let actors: Vec<String> = doc.select(&scraper::Selector::parse("a").unwrap()).filter_map(|e| { let h = e.value().attr("href").unwrap_or(""); if h.contains("actress") || h.contains("actor") { let n = e.text().collect::<String>().trim().to_string(); if n.is_empty() { None } else { Some(n) } } else { None } }).collect();
    Ok(ScrapeResult { source: "sod".into(), title, year: None, poster_url: poster, backdrop_url: None, overview: None, rating: None, runtime: None, director: None, genre: None, actors: if actors.is_empty() { None } else { Some(actors) }, score: 50 })
}

// ─── DAHLIA ───
fn scrape_dahlia(query: &str) -> Result<ScrapeResult, CommandError> {
    let code = query.trim().to_lowercase().replace(['-', '_', ' '], "");
    let url = format!("https://dahlia-av.jp/works/{}/", code);
    let resp = get_external_client().get(&url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    let body = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    if body.len() < 500 { return Err(CommandError::scrape_failed("DAHLIA 无结果")); }
    let doc = scraper::Html::parse_document(&body);
    let title = doc.select(&scraper::Selector::parse("h1, .works-title, title").unwrap()).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_else(|| query.to_string());
    let poster = doc.select(&scraper::Selector::parse("img").unwrap()).filter_map(|e| e.attr("src")).find(|s| s.contains("works") || s.contains("package")).map(|s| s.to_string());
    let actors: Vec<String> = doc.select(&scraper::Selector::parse("a").unwrap()).filter_map(|e| { let h = e.value().attr("href").unwrap_or(""); if h.contains("/actress/") { let n = e.text().collect::<String>().trim().to_string(); if n.is_empty() { None } else { Some(n) } } else { None } }).collect();
    Ok(ScrapeResult { source: "dahlia".into(), title, year: None, poster_url: poster, backdrop_url: None, overview: None, rating: None, runtime: None, director: None, genre: None, actors: if actors.is_empty() { None } else { Some(actors) }, score: 50 })
}

// ─── Remaining MetaTube providers (simple HTML scrapers) ───

macro_rules! simple_scraper {
    ($name:ident, $source:expr, $site:expr, $url_template:expr) => {
        fn $name(query: &str) -> Result<ScrapeResult, CommandError> {
            let code = query.trim().to_lowercase().replace(['-','_',' '], "");
            let url = format!($url_template, code);
            let resp = get_external_client().get(&url).send().map_err(|e| CommandError::network(&e.to_string()))?;
            let body = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
            if body.len() < 500 || body.contains("404") || body.contains("Not Found") { return Err(CommandError::scrape_failed(concat!($source, " 无结果"))); }
            let doc = scraper::Html::parse_document(&body);
            let title = doc.select(&scraper::Selector::parse("h1, h2, h3, .title, .item-title, title").unwrap()).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_else(|| query.to_string());
            let poster = doc.select(&scraper::Selector::parse("img").unwrap()).filter_map(|e| e.attr("src")).find(|s| s.contains("jacket")||s.contains("package")||s.contains("thumb")||s.contains("cover")).map(|s| s.to_string());
            let actors: Vec<String> = doc.select(&scraper::Selector::parse("a").unwrap()).filter_map(|e| { let h=e.value().attr("href").unwrap_or(""); if h.contains("actress")||h.contains("actor")||h.contains("star")||h.contains("model") { let n=e.text().collect::<String>().trim().to_string(); if n.is_empty() {None} else {Some(n)} } else {None} }).collect();
            let overview = doc.select(&scraper::Selector::parse("p, .description, .summary").unwrap()).filter_map(|e| { let t=e.text().collect::<String>().trim().to_string(); if t.len()>50 {Some(t)} else {None} }).next();
            Ok(ScrapeResult { source: $source.into(), title, year: None, poster_url: poster, backdrop_url: None, overview, rating: None, runtime: None, director: None, genre: None, actors: if actors.is_empty() { None } else { Some(actors) }, score: 40 })
        }
    };
}

simple_scraper!(scrape_1pondo, "1pondo", "1pondo.tv", "https://www.1pondo.tv/movies/{}/");
simple_scraper!(scrape_10musume, "10musume", "10musume.com", "https://www.10musume.com/detail/=/cid={}/");
simple_scraper!(scrape_caribbeancom, "caribbeancom", "caribbeancom.com", "https://www.caribbeancom.com/moviepages/{}/");
simple_scraper!(scrape_caribbeancompr, "caribbeancompr", "caribbeancompr.com", "https://www.caribbeancompr.com/moviepages/{}/");
simple_scraper!(scrape_c0930, "c0930", "c0930.com", "https://www.c0930.com/moviepages/{}/");
simple_scraper!(scrape_gcolle, "gcolle", "gcolle.net", "https://gcolle.net/product_info.php/products_id/{}");
simple_scraper!(scrape_getchu, "getchu", "getchu.com", "https://dl.getchu.com/item/{}/");
simple_scraper!(scrape_h0930, "h0930", "h0930.com", "https://www.h0930.com/moviepages/{}/");
simple_scraper!(scrape_h4610, "h4610", "h4610.com", "https://www.h4610.com/moviepages/{}/");
simple_scraper!(scrape_heydouga, "heydouga", "heydouga.com", "https://www.heydouga.com/moviepages/{}/");
simple_scraper!(scrape_heyzo, "heyzo", "heyzo.com", "https://www.heyzo.com/moviepages/{}/");
simple_scraper!(scrape_javfree, "javfree", "javfree.me", "https://javfree.me/?s={}");
simple_scraper!(scrape_kin8, "kin8", "kin8tengoku.com", "https://www.kin8tengoku.com/moviepages/{}/");
simple_scraper!(scrape_muramura, "muramura", "muramura.tv", "https://www.muramura.tv/moviepages/{}/");
simple_scraper!(scrape_mywife, "mywife", "mywife.cc", "https://mywife.cc/works/{}/");
simple_scraper!(scrape_pacopacomama, "pacopacomama", "pacopacomama.com", "https://www.pacopacomama.com/moviepages/{}/");
simple_scraper!(scrape_pcolle, "pcolle", "pcolle.com", "https://www.pcolle.com/product/detail/?id={}");
simple_scraper!(scrape_fc2hub, "fc2hub", "fc2hub.com", "https://javten.com/search?q={}");
simple_scraper!(scrape_tokyohot, "tokyohot", "tokyo-hot.com", "https://my.tokyo-hot.com/product/?q={}");
simple_scraper!(scrape_ave, "ave", "aventertainments.com", "https://www.aventertainments.com/product.aspx?keyword={}");

// ─── MetaTube (local HTTP API) ───

fn scrape_metatube(query: &str) -> Result<ScrapeResult, CommandError> {
    let base = crate::services::metatube_service::base_url();

    if !crate::services::metatube_service::health_check() {
        return Err(CommandError::scrape_failed("MetaTube 服务未运行，请在设置中启动"));
    }

    // Step 1: Search
    let search_url = format!("{}/v1/movies/search?q={}&fallback=true", base, encode(query));
    log::info!("MetaTube 搜索: {}", search_url);
    let resp = get_client().get(&search_url)
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .map_err(|e| CommandError::network(&e.to_string()))?;
    let body = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    log::info!("MetaTube 搜索响应: {}", truncate_log(&body, 300));

    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| CommandError::scrape_failed(&format!("MetaTube JSON: {}", e)))?;
    if let Some(err) = json["error"]["message"].as_str() {
        return Err(CommandError::scrape_failed(&format!("MetaTube: {}", err)));
    }
    let results = json["data"].as_array()
        .ok_or_else(|| CommandError::scrape_failed("MetaTube 返回格式异常"))?;
    if results.is_empty() {
        return Err(CommandError::scrape_failed("MetaTube 无结果"));
    }

    // Try ALL results and merge data from multiple providers
    // Each field picks the best value across all sources
    log::info!("═══ MetaTube 开始(共{}源) ═══", results.len());

    let mut best_title = query.to_string();
    let mut best_poster = None;
    let mut best_year = None;
    let mut best_rating: Option<f64> = None;
    let mut best_overview = None;
    let mut best_runtime = None;
    let mut all_actors: Vec<String> = Vec::new();
    let mut all_genres: Vec<String> = Vec::new();
    let mut sources_used: Vec<String> = Vec::new();

    for (idx, item) in results.iter().enumerate() {
        let provider_name = item["provider"].as_str().unwrap_or("metatube");
        let movie_id = item["id"].as_str().unwrap_or("");
        let title_s = item["title"].as_str().unwrap_or("-");

        // Merge search-level fields
        if title_s.len() > best_title.len() { best_title = title_s.to_string(); }
        if best_year.is_none() {
            if let Some(d) = item["release_date"].as_str() { best_year = d[..4].parse().ok(); }
        }
        if best_rating.is_none() {
            if let Some(s) = item["score"].as_f64() { best_rating = Some(s); }
        }
        if best_poster.is_none() {
            best_poster = item["cover_url"].as_str().or_else(|| item["thumb_url"].as_str()).map(|s| s.to_string());
        }
        if let Some(arr) = item["actors"].as_array() {
            for a in arr { if let Some(n) = a.as_str() { let n = n.trim().to_string(); if !all_actors.iter().any(|x| x == &n) { all_actors.push(n); } } }
        }

        // Fetch info API
        if provider_name.is_empty() || movie_id.is_empty() {
            log::info!("  {:>2}. {:<20} │ 无ID,跳过", idx+1, provider_name);
            continue;
        }
        let info_url = format!("{}/v1/movies/{}/{}", base, provider_name, movie_id);
        let mut contributed = Vec::new();
        match get_client().get(&info_url).timeout(std::time::Duration::from_secs(8)).send() {
            Ok(resp) => {
                if let Ok(body2) = resp.text() {
                    if let Ok(json2) = serde_json::from_str::<serde_json::Value>(&body2) {
                        if json2["error"]["message"].is_null() {
                            let info = &json2["data"];
                            let rt = info["runtime"].as_i64().or_else(|| info["duration"].as_i64()).unwrap_or(0);
                            let ac = info["actors"].as_array().map_or(0, |a| a.len());
                            let gc = info["genres"].as_array().map_or(0, |g| g.len());
                            let has_cover = info["big_cover_url"].as_str().or(info["cover_url"].as_str()).is_some();

                            // Apply contributions (skip overview — MetaTube data is unreliable)
                            let rt = normalize_runtime(rt);
                            // Only trust FANZA, JavBus, JAV321 for runtime. DUGA returns fake values (300, 295, 971)
                            let is_reliable = matches!(provider_name, "FANZA" | "JavBus" | "JAV321");
                            if is_reliable && rt >= 30 && rt <= 300 && rt > best_runtime.unwrap_or(0) as i64 {
                                best_runtime = Some(rt as i32);
                                contributed.push("时长");
                            }
                            if let Some(arr) = info["actors"].as_array() {
                                for a in arr { if let Some(n) = a.as_str() { let n = n.trim().to_string(); if !all_actors.iter().any(|x| x == &n) { all_actors.push(n.clone()); contributed.push("演员"); } } }
                            }
                            if let Some(arr) = info["genres"].as_array() {
                                for g in arr { if let Some(n) = g.as_str() { let n = n.trim().to_string(); if !all_genres.iter().any(|x| x == &n) { all_genres.push(n); contributed.push("类型"); } } }
                            }
                            if best_poster.is_none() || best_poster.as_ref().map_or(true, |p| p.contains("thumb")) {
                                if let Some(big) = info["big_cover_url"].as_str() { if !big.is_empty() { best_poster = Some(big.to_string()); contributed.push("封面"); } }
                                else if let Some(cov) = info["cover_url"].as_str() { if !cov.is_empty() { best_poster = Some(cov.to_string()); contributed.push("封面"); } }
                            }
                            if best_rating.is_none() { if let Some(s) = info["score"].as_f64() { best_rating = Some(s); contributed.push("评分"); } }

                            let contrib_str = if contributed.is_empty() {"无新贡献".to_string()} else {format!("贡献: {}", contributed.join(","))};
                            log::info!("  {:>2}. {:<20} │ 演员={} 类型={} 时长={} 封面={} │ {}",
                                idx+1, provider_name, ac, gc, rt, if has_cover {"有"} else {"无"}, contrib_str);
                            if !contributed.is_empty() { sources_used.push(provider_name.to_string()); }
                        } else {
                            log::info!("  {:>2}. {:<20} │ API返回错误", idx+1, provider_name);
                        }
                    }
                }
            }
            Err(e) => log::info!("  {:>2}. {:<20} │ 请求超时/失败", idx+1, provider_name),
        }
    }

    log::info!("═══ MetaTube 合并: 标题={} 演员={}人 类型={}个 时长={:?} 来源={:?} ═══",
        truncate_log(&best_title, 30), all_actors.len(), all_genres.len(), best_runtime, sources_used);

    if best_title == query && all_actors.is_empty() && best_poster.is_none() {
        return Err(CommandError::scrape_failed("MetaTube 所有源均无有效数据"));
    }

    // Step 3: Always try rich providers for genres/actors/runtime boost
    {
        let code_lower = query.trim().to_lowercase().replace(['-', '_', ' '], "");
        let fallback_providers = ["FANZA", "JAV321", "MGS"];
        for &fb_name in &fallback_providers {
            if sources_used.iter().any(|s| s.contains(fb_name)) { continue; } // already tried
            let fb_url = format!("{}/v1/movies/{}/{}", base, fb_name, code_lower);
            log::info!("MetaTube 补全尝试: {}", fb_url);
            match get_client().get(&fb_url).timeout(std::time::Duration::from_secs(8)).send() {
                Ok(resp) => {
                    if let Ok(body) = resp.text() {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                            if json["error"]["message"].is_null() {
                                let info = &json["data"];
                                if let Some(r) = info["runtime"].as_i64().or_else(|| info["duration"].as_i64()) {
                                    let r = normalize_runtime(r);
                                    let is_reliable = matches!(fb_name, "FANZA" | "JAV321" | "MGS");
                                    if is_reliable && r >= 30 && r <= 300 && r > best_runtime.unwrap_or(0) as i64 {
                                        best_runtime = Some(r as i32);
                                    }
                                }
                                if let Some(arr) = info["actors"].as_array() {
                                    for a in arr { if let Some(n) = a.as_str() { let n = n.trim().to_string(); if !all_actors.iter().any(|x| x == &n) { all_actors.push(n); } } }
                                }
                                if let Some(arr) = info["genres"].as_array() {
                                    for g in arr { if let Some(n) = g.as_str() { let n = n.trim().to_string(); if !all_genres.iter().any(|x| x == &n) { all_genres.push(n); } } }
                                }
                                if best_rating.is_none() {
                                    if let Some(s) = info["score"].as_f64() { best_rating = Some(s); }
                                }
                                if best_poster.is_none() || best_poster.as_ref().map_or(true, |p| p.contains("thumb")) {
                                    if let Some(big) = info["big_cover_url"].as_str() { if !big.is_empty() { best_poster = Some(big.to_string()); } }
                                    else if let Some(cov) = info["cover_url"].as_str() { if !cov.is_empty() { best_poster = Some(cov.to_string()); } }
                                }
                                sources_used.push(fb_name.to_string());
                                log::info!("MetaTube 补全 {}: 演员{} 类型{} 时长{:?}", fb_name, all_actors.len(), all_genres.len(), best_runtime);
                            }
                        }
                    }
                }
                Err(e) => log::info!("MetaTube 补全 {} 失败: {}", fb_name, e),
            }
        }
    }

    // Filter blacklisted genres
    let blacklist: Vec<String> = db::with_db(|conn| crate::db::queries::get_blacklisted_genres(conn)).unwrap_or_default();
    all_genres.retain(|g| !blacklist.contains(g));
    log::info!("MetaTube 过滤黑名单后: 标签{}个 (黑名单{}个)", all_genres.len(), blacklist.len());

    Ok(ScrapeResult {
        source: format!("metatube({})", sources_used.join(",")),
        title: best_title, year: best_year, poster_url: best_poster, backdrop_url: None,
        overview: best_overview, rating: best_rating, runtime: best_runtime, director: None,
        genre: if all_genres.is_empty() { None } else { Some(all_genres) },
        actors: if all_actors.is_empty() { None } else { Some(all_actors) },
        score: 65,
    })
}

/// Normalize runtime: some providers report seconds (e.g. 971), others minutes (e.g. 120).
/// Returns minutes, capped at 600 (10 hours)
fn normalize_runtime(raw: i64) -> i64 {
    if raw > 1000 { raw / 60 } else { raw }
}

fn truncate_log(s: &str, max: usize) -> &str {
    if s.len() <= max { return s; }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) { end -= 1; }
    &s[..end]
}

fn encode(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() || "-_.~".contains(c) { c.to_string() } else { format!("%{:02X}", c as u8) }).collect()
}

use anyhow::{anyhow, Result};
use clap::Parser;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

mod client;
mod config;
mod convert;
mod state;

static CANCELLED: AtomicBool = AtomicBool::new(false);

#[derive(Parser)]
#[command(
    name = "werss-cli",
    version,
    about = "Fetch articles from a WeRSS (WeChat RSS) server and save as Markdown files.",
    long_about = "\
Fetches articles from a WeRSS API server (WeChat public accounts) and saves \
them as Markdown with YAML frontmatter.

USAGE: werss-cli [--mp all|ID1,ID2] [--output DIR] [--workspace DIR]
  werss-cli --mp all                    # fetch all public accounts
  werss-cli --mp MP_WXS_123,MP_WXS_456  # fetch specific accounts
  werss-cli --init-config               # generate werss.toml template
  werss-cli                             # reads werss.toml + .env

Config priority: CLI flags > env vars > werss.toml > defaults.
Output: <output>/<mp_id>/YYYYMMDD/01/<slug>.md
State: each mp_id dir has state.jsonl tracking fetched/failed articles.
  Re-running skips fetched, retries failed. Exit 0=ok 1=fatal. Logs to stderr."
)]
struct Cli {
    #[arg(
        long,
        env = "WE_API_BASE",
        value_name = "URL",
        hide_env = true,
        help = "WeRSS API base URL. Env: WE_API_BASE",
        help_heading = "API Connection"
    )]
    api_base: Option<String>,

    #[arg(
        long,
        env = "WE_API_USERNAME",
        hide_env = true,
        help = "WeRSS API username. Env: WE_API_USERNAME",
        help_heading = "API Connection"
    )]
    username: Option<String>,

    #[arg(
        long,
        env = "WE_API_PASSWORD",
        hide_env = true,
        help = "WeRSS API password. Env: WE_API_PASSWORD",
        help_heading = "API Connection"
    )]
    password: Option<String>,

    #[arg(
        long,
        env = "WE_TARGET_MPS",
        value_name = "IDS",
        hide_env = true,
        help = "Comma-separated MP IDs, or \"all\". Env: WE_TARGET_MPS",
        help_heading = "Sync"
    )]
    mp: Option<String>,

    #[arg(
        long,
        env = "WE_OUTPUT_DIR",
        value_name = "DIR",
        hide_env = true,
        help = "Output directory for articles (created if missing). Env: WE_OUTPUT_DIR",
        help_heading = "Sync"
    )]
    output: Option<String>,

    #[arg(
        long,
        env = "WE_WORKSPACE_DIR",
        value_name = "DIR",
        hide_env = true,
        help = "Also publish to <DIR>/published/YYYYMMDD/<slug>/. Env: WE_WORKSPACE_DIR",
        help_heading = "Sync"
    )]
    workspace: Option<String>,

    #[arg(
        long,
        env = "WE_SINCE",
        value_name = "DATE",
        hide_env = true,
        help = "Only fetch articles published since DATE (YYYY-MM-DD). Env: WE_SINCE",
        help_heading = "Sync"
    )]
    since: Option<String>,

    #[arg(
        long,
        env = "WE_UNTIL",
        value_name = "DATE",
        hide_env = true,
        help = "Only fetch articles published until DATE (YYYY-MM-DD). Env: WE_UNTIL",
        help_heading = "Sync"
    )]
    until: Option<String>,

    #[arg(
        long,
        env = "WE_LIMIT",
        value_name = "N",
        hide_env = true,
        help = "Max articles to fetch per run (0 = no limit). Env: WE_LIMIT",
        help_heading = "Sync"
    )]
    limit: Option<u32>,

    #[arg(
        long,
        env = "WE_START_PAGE",
        hide_env = true,
        help = "Start page for MP sync. Env: WE_START_PAGE",
        help_heading = "Sync"
    )]
    start_page: Option<i64>,

    #[arg(
        long,
        env = "WE_END_PAGE",
        hide_env = true,
        help = "End page for MP sync. Env: WE_END_PAGE",
        help_heading = "Sync"
    )]
    end_page: Option<i64>,

    #[arg(long, default_value = "werss.toml", help_heading = "Config")]
    /// TOML config file path (use --init-config to generate)
    config: String,

    #[arg(long, help_heading = "Config")]
    /// Generate a werss.toml template at --config path and exit
    init_config: bool,
}

struct Resolved {
    api_base: String,
    username: String,
    password: String,
    mp: String,
    output: String,
    workspace: String,
    max_failures: u32,
    since_ts: Option<i64>,
    until_ts: Option<i64>,
    limit: u32,
    start_page: i64,
    end_page: i64,
}

fn parse_date(s: &str) -> Option<i64> {
    if s.trim().is_empty() {
        return None;
    }
    chrono::NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .and_then(|dt| dt.and_utc().timestamp().into())
}

fn resolve(cli: &Cli) -> Resolved {
    let cfg = config::Config::load_optional(Path::new(&cli.config));
    let since_str = cli.since.as_deref().unwrap_or(cfg.sync.since.as_str());
    let until_str = cli.until.as_deref().unwrap_or(cfg.sync.until.as_str());
    Resolved {
        api_base: cli.api_base.clone().unwrap_or_else(|| cfg.api.base.clone()),
        username: cli
            .username
            .clone()
            .unwrap_or_else(|| cfg.api.username.clone()),
        password: cli
            .password
            .clone()
            .unwrap_or_else(|| cfg.api.password.clone()),
        mp: cli
            .mp
            .clone()
            .unwrap_or_else(|| cfg.sync.target_mps.to_comma_string()),
        output: cli
            .output
            .clone()
            .unwrap_or_else(|| cfg.sync.output_dir.clone()),
        workspace: cli
            .workspace
            .clone()
            .unwrap_or_else(|| cfg.sync.workspace_dir.clone()),
        max_failures: cfg.sync.max_failures,
        since_ts: parse_date(since_str),
        until_ts: parse_date(until_str).map(|ts| ts + 86399),
        limit: cli.limit.unwrap_or(cfg.sync.limit),
        start_page: cli.start_page.unwrap_or(cfg.sync.start_page),
        end_page: cli.end_page.unwrap_or(cfg.sync.end_page),
    }
}

fn preflight(r: &Resolved) -> Result<()> {
    if r.mp.trim().is_empty() {
        return Err(anyhow!("No target MPs specified. Use --mp <IDs>, set WE_TARGET_MPS, or configure target_mps in werss.toml"));
    }
    let out = Path::new(&r.output);
    if out.exists() && !out.is_dir() {
        return Err(anyhow!(
            "--output '{}' exists but is not a directory",
            r.output
        ));
    }
    if !r.workspace.is_empty() {
        let ws = Path::new(&r.workspace);
        if ws.exists() && !ws.is_dir() {
            return Err(anyhow!(
                "--workspace '{}' exists but is not a directory",
                r.workspace
            ));
        }
    }
    if out.exists() {
        let probe = out.join(".werss_write_test");
        if std::fs::write(&probe, b"").is_err() {
            return Err(anyhow!(
                "No write permission to output directory '{}'",
                r.output
            ));
        }
        let _ = std::fs::remove_file(&probe);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    let _ = dotenvy::from_path(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../werss/.env"
    )));
    let cli = Cli::parse();

    if cli.init_config {
        let example = config::generate_example();
        let path = Path::new(&cli.config);
        if path.exists() {
            return Err(anyhow!(
                "{} already exists, not overwriting",
                path.display()
            ));
        }
        std::fs::write(path, &example)?;
        eprintln!("Created {}", path.display());
        return Ok(());
    }

    let r = resolve(&cli);
    preflight(&r)?;

    eprintln!("=== WeRSS Article Fetcher ===");
    eprintln!("API: {}", r.api_base);
    eprintln!("Output: {}", r.output);

    tokio::spawn(async {
        tokio::signal::ctrl_c().await.ok();
        CANCELLED.store(true, Ordering::Relaxed);
        eprintln!("\nCaught Ctrl+C, finishing current article...");
    });

    let c = Arc::new(client::WeClient::new(&r.api_base, &r.username, &r.password).await?);
    eprintln!("Login successful!");

    let mps = resolve_mps(&c, &r.mp).await?;
    if mps.is_empty() {
        return Err(anyhow!(
            "No matching public accounts found for --mp '{}'",
            r.mp
        ));
    }

    let (mut fetched, mut skipped, mut failed, mut exhausted, mut ws_failed) =
        (0u32, 0u32, 0u32, 0u32, 0u32);
    let sem = Arc::new(tokio::sync::Semaphore::new(3));

    for mp in &mps {
        if CANCELLED.load(Ordering::Relaxed) {
            eprintln!("Cancelled.");
            break;
        }

        eprintln!("\n── {} ({}) ──", mp.mp_name, mp.id);
        let mut store = state::StateStore::new(&mp.id, &r.output)?;

        eprintln!("  Syncing (pages {}-{})...", r.start_page, r.end_page);

        let mut retries = 0u32;
        loop {
            if CANCELLED.load(Ordering::Relaxed) {
                break;
            }
            match c.update_mp(&mp.id, r.start_page, r.end_page).await {
                Ok(_) => {
                    eprintln!("  Sync done.");
                    break;
                }
                Err(e) if retries < 3 => {
                    retries += 1;
                    eprintln!("  [WARN] Sync attempt {}/3 failed: {}", retries, e);
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
                Err(e) => {
                    eprintln!("  [WARN] Sync failed after 3 retries: {}", e);
                    break;
                }
            }
        }

        let articles = c.list_articles(&mp.id).await?;
        let articles: Vec<_> = articles
            .into_iter()
            .filter(|a| {
                if let Some(s) = r.since_ts {
                    if a.publish_time < s {
                        return false;
                    }
                }
                if let Some(u) = r.until_ts {
                    if a.publish_time > u {
                        return false;
                    }
                }
                true
            })
            .collect();
        eprintln!("  {} articles (filtered by time range)", articles.len());

        if articles.is_empty() {
            continue;
        }

        let mut pending = Vec::new();
        for art in &articles {
            if store.is_fetched(&art.id) {
                skipped += 1;
            } else if store.is_exhausted(&art.id, r.max_failures) {
                exhausted += 1;
            } else {
                pending.push(art.clone());
            }
        }
        eprintln!(
            "  {} skipped, {} exhausted, {} to fetch",
            articles.len() - pending.len() - exhausted as usize,
            exhausted,
            pending.len()
        );

        if pending.is_empty() {
            continue;
        }

        if r.limit > 0 && pending.len() > r.limit as usize {
            pending.truncate(r.limit as usize);
        }

        let mut tasks = Vec::new();
        for art in &pending {
            if CANCELLED.load(Ordering::Relaxed) {
                break;
            }
            match store.resolve_article_dir(&art.id, &art.title, art.publish_time) {
                Ok(dir) => tasks.push((art.clone(), dir)),
                Err(e) => {
                    failed += 1;
                    if let Err(se) =
                        store.record(&art.id, &art.title, art.publish_time, "failed", "")
                    {
                        eprintln!("  [WARN] Failed to record state: {}", se);
                    }
                    eprintln!(
                        "  FAIL dir: {} — {}",
                        art.title.chars().take(40).collect::<String>(),
                        e
                    );
                }
            }
        }

        let mp_dir = store.mp_dir().to_path_buf();
        let mut handles = Vec::new();
        for (art, dir) in tasks {
            if CANCELLED.load(Ordering::Relaxed) {
                break;
            }
            let permit = Arc::clone(&sem).acquire_owned().await?;
            let c = Arc::clone(&c);
            let mp_name = mp.mp_name.clone();
            let workspace = r.workspace.clone();
            let mp_dir = mp_dir.clone();
            let spawn_art = art.clone();
            let spawn_dir = dir.clone();

            let handle = tokio::spawn(async move {
                let _permit = permit;
                fetch_and_write(&c, &spawn_art, &mp_name, &spawn_dir, &mp_dir, &workspace).await
            });
            handles.push((art, handle, dir));
        }

        for (art, handle, dir) in handles {
            let short: String = art.title.chars().take(40).collect();
            match handle.await {
                Ok(Ok((rel_path, ws_ok))) => {
                    fetched += 1;
                    if !ws_ok {
                        ws_failed += 1;
                    }
                    if let Err(e) =
                        store.record(&art.id, &art.title, art.publish_time, "success", &rel_path)
                    {
                        eprintln!("  [WARN] Failed to record state: {}", e);
                    }
                    eprintln!("  OK: {}", short);
                }
                Ok(Err(e)) => {
                    failed += 1;
                    if let Err(se) =
                        store.record(&art.id, &art.title, art.publish_time, "failed", "")
                    {
                        eprintln!("  [WARN] Failed to record state: {}", se);
                    }
                    eprintln!("  FAIL: {} — {}", short, e);
                    clean_empty_dir(&dir);
                }
                Err(e) => {
                    failed += 1;
                    if let Err(se) =
                        store.record(&art.id, &art.title, art.publish_time, "failed", "")
                    {
                        eprintln!("  [WARN] Failed to record state: {}", se);
                    }
                    eprintln!("  FAIL: {} — task error: {}", short, e);
                    clean_empty_dir(&dir);
                }
            }
        }
    }

    eprintln!(
        "\n=== Done: fetched={} skipped={} failed={} exhausted={} ws_failed={} ===",
        fetched, skipped, failed, exhausted, ws_failed
    );
    if failed > 0 {
        eprintln!("Note: re-run to retry failed articles (they are tracked in state.jsonl)");
    }
    if exhausted > 0 {
        eprintln!(
            "Note: {} articles skipped after {} failures (config: max_failures)",
            exhausted, r.max_failures
        );
    }
    if CANCELLED.load(Ordering::Relaxed) {
        std::process::exit(0);
    }
    Ok(())
}

async fn resolve_mps(c: &client::WeClient, mp: &str) -> Result<Vec<client::MpInfo>> {
    if mp.trim().eq_ignore_ascii_case("all") {
        eprintln!("Fetching all public accounts...");
        let all = c.list_mps().await?;
        eprintln!("Found {} accounts", all.len());
        return Ok(all);
    }
    let ids: Vec<&str> = mp
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if ids.is_empty() {
        return Err(anyhow!("No valid MP IDs in --mp value"));
    }
    let all = c.list_mps().await?;
    let filtered: Vec<_> = all
        .into_iter()
        .filter(|m| ids.contains(&m.id.as_str()))
        .collect();
    let missing: Vec<_> = ids
        .into_iter()
        .filter(|id| !filtered.iter().any(|m| m.id == *id))
        .collect();
    if !missing.is_empty() {
        eprintln!("  [WARN] MP IDs not found on server: {:?}", missing);
    }
    eprintln!("Using {} configured accounts", filtered.len());
    Ok(filtered)
}

async fn fetch_and_write(
    c: &client::WeClient,
    art: &client::ArticleInfo,
    mp_name: &str,
    dir: &Path,
    mp_dir: &Path,
    workspace: &str,
) -> Result<(String, bool)> {
    let mut retries = 0u32;
    let tid = loop {
        match c.refresh_article(&art.id).await {
            Ok(Some(tid)) => break tid,
            Ok(None) => anyhow::bail!("No task_id returned for article {}", art.id),
            Err(e) if retries < 3 => {
                retries += 1;
                eprintln!(
                    "  [WARN] Refresh attempt {}/3 for '{}': {}",
                    retries,
                    art.title.chars().take(40).collect::<String>(),
                    e
                );
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
            Err(e) => return Err(e),
        }
    };
    let st = c.poll_task(&tid, 3, 180).await?;
    if st != "success" {
        anyhow::bail!("Refresh task {}", st);
    }

    let detail = c.get_article_detail(&art.id).await?;
    let html = if !detail.content.is_empty() {
        detail.content.clone()
    } else if !detail.content_html.is_empty() {
        convert::strip_content_html_header(&detail.content_html)
    } else {
        String::new()
    };
    if html.trim().is_empty() {
        anyhow::bail!("Empty content after refresh");
    }
    if detail.title.is_empty() {
        anyhow::bail!("Empty title in article detail");
    }

    let md = convert::article_to_md(
        &detail.title,
        mp_name,
        &detail.mp_id,
        &detail.url,
        &detail.pic_url,
        &detail.description,
        detail.publish_time,
        &html,
    );
    let slug = convert::slugify(&detail.title);
    let path = dir.join(format!("{}.md", slug));
    std::fs::write(&path, &md).map_err(|e| anyhow!("Failed to write {}: {}", path.display(), e))?;

    let rel = path
        .strip_prefix(mp_dir)
        .map_err(|_| {
            anyhow!(
                "Path mismatch: {} not under {}",
                path.display(),
                mp_dir.display()
            )
        })?
        .to_string_lossy()
        .to_string();

    let ws_ok = if !workspace.is_empty() {
        publish_to_workspace(
            c,
            &detail.pic_url,
            detail.publish_time,
            &slug,
            &md,
            workspace,
        )
        .await
    } else {
        true
    };

    Ok((rel, ws_ok))
}

async fn publish_to_workspace(
    c: &client::WeClient,
    pic_url: &str,
    ts: i64,
    slug: &str,
    md: &str,
    workspace: &str,
) -> bool {
    let ws = Path::new(workspace).join("published");
    let ds = chrono::DateTime::from_timestamp(ts, 0)
        .map(|d| d.format("%Y%m%d").to_string())
        .unwrap_or_else(|| "unknown".into());
    let td = ws.join(&ds).join(slug);
    if std::fs::create_dir_all(td.join("imgs")).is_err() {
        eprintln!("  [WARN] Failed to create workspace dir {}", td.display());
        return false;
    }
    if !pic_url.is_empty() {
        if let Ok(bytes) = c.download_image(pic_url).await {
            let ext = infer_image_ext(pic_url);
            let img_path = td.join("imgs").join(format!("cover.{}", ext));
            let _ = std::fs::write(&img_path, &bytes);
        }
    }
    let md_path = td.join(format!("{}.md", slug));
    if let Err(e) = std::fs::write(&md_path, md) {
        eprintln!("  [WARN] Workspace write failed: {}", e);
        return false;
    }
    eprintln!("  [WORKSPACE] -> {}/{}/{}", ds, slug, slug);
    true
}

fn infer_image_ext(url: &str) -> &'static str {
    let path = url.split('?').next().unwrap_or(url);
    let path_lower = path.to_lowercase();
    if path_lower.ends_with(".jpg") || path_lower.ends_with(".jpeg") {
        "jpg"
    } else if path_lower.ends_with(".webp") {
        "webp"
    } else if path_lower.ends_with(".gif") {
        "gif"
    } else {
        "png"
    }
}

fn clean_empty_dir(dir: &Path) {
    if dir.is_dir()
        && std::fs::read_dir(dir)
            .map(|mut r| r.next().is_none())
            .unwrap_or(false)
    {
        let _ = std::fs::remove_dir(dir);
    }
}

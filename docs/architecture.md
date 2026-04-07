# Architecture

## Module overview

```
src/
├── main.rs     # CLI parsing, orchestration, config resolution, graceful shutdown
├── config.rs   # Config: TOML parsing, TomlVecOrString type, defaults, example generation
├── client.rs   # WeClient: HTTP client, auth, API methods, token refresh
├── convert.rs  # HTML-to-Markdown conversion, slug generation, frontmatter, cleanup
└── state.rs    # StateStore: JSONL state file, record/lookup/compact logic
```

### main.rs

Entry point. Responsibilities:

- Parses CLI arguments via `clap`
- Resolves configuration from CLI/env/TOML/defaults using `resolve()`
- Runs preflight checks (MP list non-empty, output dir writable)
- Sets up graceful shutdown via `tokio::signal` + `AtomicBool`
- Iterates over MPs, fetches articles in parallel with `Semaphore(3)`
- Tracks counters: fetched, skipped, failed, exhausted, ws_failed

### client.rs

HTTP client for the WeRSS API. Key type: `WeClient`.

- Manages authentication with Bearer token stored in `Mutex<String>`
- Auto-refreshes token on 401 responses (re-login transparently)
- Methods: `list_mps`, `update_mp`, `list_articles`, `refresh_article`, `poll_task`, `get_article_detail`, `download_image`
- All API responses are JSON, validated before returning
- Connection errors, timeouts, and non-JSON responses are handled with descriptive errors

### config.rs

Configuration management. Key types: `Config`, `ApiConfig`, `SyncConfig`, `TomlVecOrString`.

- `Config::load_optional()` — loads from TOML file, falls back to defaults on error
- `TomlVecOrString` — allows `target_mps` to be either `"all"` or `["ID1", "ID2"]`
- `generate_example()` — produces the template for `--init-config`
- All fields have sensible defaults via `Default` trait implementations

### convert.rs

Content conversion. Key functions:

- `article_to_md()` — produces full Markdown with YAML frontmatter
- `slugify()` — generates URL-safe filenames from titles
- `html_to_md()` — wraps `html2md::parse_html()` with code block dedenting
- `strip_content_html_header()` — removes cover image, h1, and author line from `content_html`
- `clean_tail()` — removes WeChat UI artifacts from the end of articles
- `dedent_code_blocks()` — removes common leading whitespace from fenced code blocks

### state.rs

Incremental state tracking. Key type: `StateStore`.

- JSONL format — one JSON record per line, append-only
- `is_fetched()` — checks if article has `status: "success"`
- `is_exhausted()` — checks if article has `status: "failed"` and `failed_count >= max_failures`
- `record()` — appends a state record, manages `failed_count` (increments on failure, resets on success)
- `compact()` — rewrites the JSONL file keeping only the latest record per article (triggered when line count > 2× unique IDs)
- `resolve_article_dir()` — determines the output directory for an article, handling existing directories and sequence numbering

## Execution flow

```
werss-cli
  │
  ├─ Load config (CLI > env vars > werss.toml > .env > defaults)
  ├─ Preflight checks
  │    ├─ MP list non-empty
  │    ├─ Output directory writable
  │    └─ Workspace path valid (if specified)
  ├─ Login API → obtain Bearer token (auto re-login on 401)
  ├─ Resolve target MP list
  │    ├─ "all" → fetch all public accounts
  │    └─ "ID1,ID2" → filter by exact ID
  └─ Per-MP processing
       ├─ update_mp (trigger WeChat sync, 3 retries with 5s sleep)
       ├─ List articles (auto-paginated)
       ├─ Filter by since/until date range
       └─ Per-article
            ├─ state: success → SKIP
            ├─ state: failed_count >= max_failures → EXHAUSTED (skip)
            └─ Otherwise → refresh → poll → detail → HTML→MD → write file
                 └─ Truncate to --limit (if specified)
```

## Concurrency model

- **Runtime**: `tokio` multi-threaded runtime (`#[tokio::main]`)
- **MP processing**: Sequential — each MP is processed one at a time
- **Article fetching**: Parallel within each MP — up to 3 articles fetched concurrently via `tokio::sync::Semaphore`
- **Cancellation**: `AtomicBool` (`CANCELLED`) checked before each MP and each article. Ctrl+C sets the flag, and the main loop exits after the current work completes.

## Authentication and token management

1. On startup, `WeClient::new()` calls `POST /api/v1/wx/auth/login` with form-encoded credentials
2. The returned `access_token` is stored in `Mutex<String>`
3. All subsequent API requests include `Authorization: Bearer <token>`
4. On 401 responses, `WeClient::req()` automatically re-logs in and retries the request with the new token

## Retry and resilience

| Layer | Retries | Delay | Notes |
|-------|---------|-------|-------|
| `update_mp` | 3 | 5s | Per-MP sync from WeChat |
| `refresh_article` | 3 | 5s | Per-article content refresh |
| `poll_task` | 5 consecutive errors | 3s interval, 180s max | Waits for refresh task completion |
| Article-level | `max_failures` (default 3) | On next run | Controlled by state tracking |

## Key dependencies

| Crate | Purpose |
|-------|---------|
| `reqwest` | HTTP client with JSON and native-tls |
| `tokio` | Async runtime (multi-thread, macros, time, signal) |
| `clap` | CLI argument parsing with derive and env support |
| `html2md` | HTML to Markdown conversion |
| `serde` / `serde_json` | Serialization/deserialization |
| `chrono` | Timestamp formatting (no default features, std only) |
| `dotenvy` | `.env` file loading |
| `anyhow` | Error handling with context |
| `regex` | HTML header stripping patterns |
| `toml` | Config file parsing |

### Build note

`html2md` depends on the `panic_unwind` runtime. The release profile cannot use `panic = "abort"`. The current profile uses:

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
```

## Data flow

```
CLI args ──> resolve() ──> Resolved config
                              │
WeClient::new() ──> login ──> Bearer token
                              │
list_mps() ──> Vec<MpInfo> ──> resolve_mps()
                              │
                    ┌── Per-MP ──────────────────┐
                    │ update_mp()                 │
                    │ list_articles() + filter    │
                    │                             │
                    │   ┌── Per-article ────────┐ │
                    │   │ StateStore check      │ │
                    │   │ refresh_article()     │ │
                    │   │ poll_task()           │ │
                    │   │ get_article_detail()  │ │
                    │   │ convert::article_to_md│ │
                    │   │ write .md file        │ │
                    │   │ update state.jsonl    │ │
                    │   │ (optional) publish    │ │
                    │   └───────────────────────┘ │
                    └─────────────────────────────┘
```

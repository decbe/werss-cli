# Architecture

## Module overview

```
src/
├── main.rs     # CLI parsing, orchestration, config resolution, graceful shutdown
├── config.rs   # Config: TOML parsing, TomlVecOrString type, defaults, example generation
├── client.rs   # WeClient: HTTP client, auth, API methods, token refresh
├── auth.rs     # Token management: keyring storage, validation, refresh
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

- Manages authentication with Bearer token stored in `TokenData`
- Auto-refreshes token on 401 responses (attempts refresh, falls back to re-login)
- Methods: `list_mps`, `update_mp`, `list_articles`, `refresh_article`, `poll_task`, `get_article_detail`, `download_image`
- All API responses are JSON, validated before returning
- Connection errors, timeouts, and non-JSON responses are handled with descriptive errors

### auth.rs

Token management and storage. Key type: `TokenData`.

- Securely stores tokens in system keyring (Linux Secret Service, macOS Keychain, Windows Credential Manager)
- Parses API responses to extract `access_token`, optional `refresh_token`, and `expires_in`
- Validates token expiry with 5-minute buffer to prevent edge cases
- Methods: `from_response()` (parse API response), `is_valid()` (expiry check), `save()` (to keyring), `load()` (from keyring), `delete()` (from keyring)
- Supports automatic token refresh before expiry to maintain session continuity

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
- `convert_html()` — wraps `html-to-markdown-rs::convert()` with image URL extraction and rewriting
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

### First Run
1. Check if valid token exists in system keyring
2. If not found or expired, attempt to load credentials from:
   - CLI flags (`--username`, `--password`)
   - Environment variables (`WE_API_USERNAME`, `WE_API_PASSWORD`)
   - Config file (`werss.toml`)
   - Interactive prompt (if no credentials provided)
3. Call `POST /api/v1/wx/auth/login` with credentials
4. Extract `access_token` from response (may include optional `refresh_token`)
5. Store token in system keyring with expiry time
6. All subsequent requests use `Authorization: Bearer <token>`

### Subsequent Runs
1. Check keyring for existing token
2. If found and still valid (with 5-minute buffer), use it immediately
3. If expired, attempt automatic refresh:
   - If `refresh_token` exists and is valid, call token refresh endpoint
   - If refresh fails or no `refresh_token`, fall back to re-authentication with stored/provided credentials
4. Token is automatically saved to keyring for next run

### Token Expiry Handling
- 401 responses trigger automatic token refresh (if `refresh_token` exists)
- Failed refresh falls back to re-authentication with credentials
- Tokens are validated before each use with 5-minute buffer
- System keyring handles cross-platform secure storage (Linux, macOS, Windows)

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
| `html-to-markdown-rs` | HTML to Markdown conversion |
| `serde` / `serde_json` | Serialization/deserialization |
| `chrono` | Timestamp formatting (no default features, std only) |
| `dotenvy` | `.env` file loading |
| `anyhow` | Error handling with context |
| `regex` | HTML header stripping patterns |
| `toml` | Config file parsing |
| `keyring` | Cross-platform secure token storage (Linux/macOS/Windows) |

### Build note

Release profile is optimized for size:

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

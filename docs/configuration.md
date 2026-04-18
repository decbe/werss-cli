# Configuration

werss-cli supports multiple configuration sources with a clear priority order:

```
CLI flags > Environment variables > werss.toml > .env > Built-in defaults
```

Higher-priority sources override lower ones. All settings are optional — you only need to provide what differs from the defaults.

## Authentication

werss-cli uses a secure token management system for authentication:

### First Run
On the first run, you must provide credentials (username and password). These can be provided via:
- CLI flags: `werss-cli --username admin --password secret`
- Environment variables: `WE_API_USERNAME=admin WE_API_PASSWORD=secret`
- Config file: set `api.username` and `api.password` in `werss.toml`
- Interactive prompt: if no credentials are found, werss-cli will prompt for them

### Subsequent Runs
After the first successful authentication:
1. werss-cli automatically loads the saved token from system keyring
2. The token is reused for all API requests
3. No credentials need to be provided (though you can override them if desired)

### Token Storage
- **Linux**: Stored in GNOME Secret Service or KDE Wallet
- **macOS**: Stored in Keychain
- **Windows**: Stored in Credential Manager

This is more secure than file-based storage as the system handles encryption.

### Token Expiry and Refresh
- If a token expires, werss-cli automatically attempts to refresh it
- If refresh fails, the tool falls back to re-authentication using your credentials
- This ensures uninterrupted operation even when tokens expire

### Configuration Priority for Auth
```
CLI flags > Environment variables > werss.toml > .env > System keyring > Interactive prompt
```

## CLI options

### API Connection

| Flag | Env variable | Type | Default | Description |
|------|-------------|------|---------|-------------|
| `-a, --api-base <URL>` | `WE_API_BASE` | string | *(empty)* | wechat RSS API base URL |
| `-u, --username <USERNAME>` | `WE_API_USERNAME` | string | *(empty)* | wechat RSS API username |
| `-p, --password <PASSWORD>` | `WE_API_PASSWORD` | string | *(empty)* | wechat RSS API password |

### Sync

| Flag | Env variable | Type | Default | Description |
|------|-------------|------|---------|-------------|
| `-m, --mp <IDS>` | `WE_TARGET_MPS` | string | `all` | comma-separated MP IDs, or "all" |
| `-o, --output <DIR>` | `WE_OUTPUT_DIR` | string | `./articles` | output directory for articles |
| `-w, --workspace <DIR>` | `WE_WORKSPACE_DIR` | string | *(empty)* | also publish to `<DIR>/published/YYYYMMDD/<slug>/` |
| `-s, --since <DATE>` | `WE_SINCE` | date | *(empty)* | only fetch articles published since DATE (YYYY-MM-DD) |
| `-e, --until <DATE>` | `WE_UNTIL` | date | *(empty)* | only fetch articles published until DATE (YYYY-MM-DD) |
| `-l, --limit <N>` | `WE_LIMIT` | u32 | `0` | max articles to fetch per run (0 = unlimited) |
| `-D, --download-images` | `WE_DOWNLOAD_IMAGES` | bool | `false` | download images from HTML content to local imgs/ directory |
| `-S, --start-page <N>` | `WE_START_PAGE` | i64 | `0` | start page for MP sync |
| `-E, --end-page <N>` | `WE_END_PAGE` | i64 | `1` | end page for MP sync |

### Config

| Flag | Description |
|------|-------------|
| `-c, --config <CONFIG>` | TOML config file path (default: `werss.toml`) |
| `-I, --init-config` | generate a `werss.toml` template and exit |

## werss.toml

Generate a starter config:

```bash
werss-cli --init-config                    # creates werss.toml in CWD
werss-cli --init-config --config /path/to/werss.toml
```

Full annotated example:

```toml
# werss-cli configuration

[api]
base = "http://your-server:8001"
username = "your-username"
password = "your-password"

[sync]
# "all" to fetch every MP, or a list of IDs
target_mps = "all"
# target_mps = ["MP_WXS_3079106503", "MP_WXS_3540720447"]

output_dir = "./articles"
workspace_dir = ""

# Max retry count for failed articles (0 = retry forever)
max_failures = 3

# Only fetch articles published since/until this date (YYYY-MM-DD)
since = ""
until = ""

# Max number of articles to fetch per run (0 = no limit)
limit = 0

# Page range for sync (start_page=0, end_page=1 means only latest page)
start_page = 0
end_page = 1
```

### Field details

- **`target_mps`**: Accepts a string (`"all"`) or an array of IDs (`["MP_WXS_123", "MP_WXS_456"]`). This flexibility is handled by the `TomlVecOrString` type.
- **`max_failures`**: Maximum retry attempts for failed articles. Default is `3`. Set to `0` to retry forever.
- **`since` / `until`**: Date filtering in `YYYY-MM-DD` format. Articles are filtered by `publish_time`.
- **`start_page` / `end_page`**: Controls the page range passed to the MP sync endpoint. Default `0..1` fetches only the latest page.

## Environment variables

All CLI flags have equivalent environment variables:

```bash
WE_API_BASE=http://10.0.0.1:8001
WE_API_USERNAME=admin
WE_API_PASSWORD=secret
WE_TARGET_MPS=all
WE_OUTPUT_DIR=./articles
WE_WORKSPACE_DIR=
WE_SINCE=2026-01-01
WE_UNTIL=2026-03-31
WE_LIMIT=10
WE_DOWNLOAD_IMAGES=false
WE_START_PAGE=0
WE_END_PAGE=1
```

Usage:

```bash
WE_API_BASE=http://10.0.0.1:8001 WE_TARGET_MPS=all werss-cli
```

## .env file (legacy)

werss-cli automatically loads `.env` from:
1. The current working directory
2. `../werss/.env` (relative to the project root)

Example `.env`:

```bash
WE_API_BASE=http://192.168.110.2:8001
WE_API_USERNAME=admin
WE_API_PASSWORD=admin@123
WE_TARGET_MPS=all
WE_OUTPUT_DIR=./articles
```

> **Note:** Prefer `werss.toml` or environment variables for new setups. The `.env` support exists for backwards compatibility.

## Minimal configuration

The absolute minimum required to run werss-cli:

```bash
werss-cli --api-base http://localhost:8001 --username admin --password secret
```

Or via `werss.toml`:

```toml
[api]
base = "http://localhost:8001"
username = "admin"
password = "secret"
```

With `target_mps = "all"` (the default), werss-cli will fetch all public accounts from the server.

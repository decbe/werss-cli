# Usage Guide

## First run

1. **Build** the binary:

   ```bash
   cargo build --release
   ```

2. **Generate** a config template:

   ```bash
   ./target/release/werss-cli --init-config
   ```

3. **Edit** `werss.toml` — set your API base URL, username, and password.

4. **Run** (first time requires credentials):

   ```bash
   ./target/release/werss-cli
   ```
   
   On the first run:
   - werss-cli reads credentials from `werss.toml`, environment variables, or CLI flags
   - If no credentials are found, it will prompt you to enter them interactively
   - After successful authentication, the token is automatically saved to your system keyring
   - The tool then proceeds to fetch articles as normal

## Subsequent runs

Simply run the tool again:

```bash
./target/release/werss-cli
```

On subsequent runs:
- werss-cli automatically loads the saved token from your system keyring
- No credentials are needed
- The token is reused for all API requests
- If the token expires, werss-cli automatically refreshes it or prompts for re-authentication if needed

## Common workflows

### Fetch all accounts

```bash
werss-cli
```

Uses `werss.toml` or defaults. With `target_mps = "all"`, fetches all public accounts from the WeRSS server.

### Fetch specific accounts

```bash
werss-cli --mp MP_WXS_3079106503,MP_WXS_3540720447
```

Comma-separated MP IDs. Unknown IDs produce a warning but don't stop execution.

### Custom output directory

```bash
werss-cli --output ./data
```

The directory is created if it doesn't exist.

### Date range filtering

```bash
# Articles since a date
werss-cli --since 2026-01-01

# Articles in a specific range
werss-cli --since 2026-03-01 --until 2026-03-31
```

Dates use `YYYY-MM-DD` format. Filtering applies to the article's `publish_time`.

### Limit article count

```bash
werss-cli --limit 10
```

Limits the number of articles fetched per run. Useful for testing or rate-limiting.

### Page range control

```bash
werss-cli --start-page 0 --end-page 3
```

Controls which pages to request from the WeRSS server during MP sync. Default is `0..1` (latest page only).

### With workspace publishing

```bash
werss-cli --workspace ./workspace
```

When set, each article is also published to:

```
{workspace}/published/YYYYMMDD/{slug}/{slug}.md
{workspace}/published/YYYYMMDD/{slug}/imgs/cover.{ext}
```

### All via environment variables

```bash
WE_API_BASE=http://10.0.0.1:8001 WE_TARGET_MPS=all werss-cli
```

### Custom config file path

```bash
werss-cli --config /etc/werss.toml
```

## What happens when you run werss-cli

1. **Config resolution** — merges CLI flags, env vars, werss.toml, and defaults
2. **Authentication** — checks keyring for valid token; prompts for credentials if needed
3. **Preflight checks** — validates MP list, output directory write permissions
4. **Token management** — saves token to keyring for next run
5. **MP resolution** — `"all"` fetches all accounts; comma-separated IDs are filtered
6. **Per-MP processing**:
   - Triggers MP sync from WeChat (`update_mp`, retried 3 times)
   - Lists articles with date filtering
   - Checks state: skips fetched, identifies exhausted, queues pending
   - Fetches pending articles in parallel (max 3 concurrent)
7. **Output** — writes Markdown files and updates state

### Summary line

```
=== Done: fetched=5 skipped=3 failed=1 exhausted=0 ws_failed=0 ===
```

| Counter | Meaning |
|---------|---------|
| `fetched` | Successfully downloaded and saved |
| `skipped` | Already fetched (status=success in state.jsonl) |
| `failed` | Failed this run (will be retried next run) |
| `exhausted` | Failed too many times (≥ max_failures), skipped |
| `ws_failed` | Fetched OK but workspace publish failed |

## Graceful shutdown

Press **Ctrl+C** to signal cancellation. werss-cli will:

1. Finish the current article being processed
2. Stop fetching new articles
3. Exit cleanly

All state is preserved. Simply re-run to continue from where it left off.

## Automated / cron usage

Example crontab entry (every hour):

```cron
0 * * * * cd /path/to/werss-cli && ./target/release/werss-cli >> /var/log/werss.log 2>&1
```

Recommended flags for unattended use:

```bash
werss-cli --config /etc/werss.toml --limit 50
```

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success (may include failed/exhausted articles) |
| 1 | Fatal error (config, auth, connection) |

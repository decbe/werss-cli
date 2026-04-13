# werss-cli

Fetch articles from WeChat public accounts (微信公众号) via a WeRSS API server and save them as Markdown files with YAML frontmatter.

## Features

- Fetches articles from WeChat public accounts through WeRSS API
- Converts HTML to clean Markdown with YAML frontmatter
- Incremental sync — tracks fetched/failed articles, skips duplicates, retries failures
- Parallel fetching with concurrency control (max 3 simultaneous)
- Graceful shutdown on Ctrl+C — finishes current article, state is preserved
- Configurable via CLI flags, environment variables, or TOML config file
- Optional workspace publishing with cover image downloads
- Date range filtering and article count limits
- **Secure token management** — automatic credential storage and refresh with system keyring
- Interactive authentication — falls back to password prompt when needed

## Authentication

### First Run
```bash
./target/release/werss-cli --api-base http://your-server:8001 \
                          --username your-username \
                          --password your-password \
                          --mp all
```
→ Credentials are validated and token is automatically saved to system keyring

### Subsequent Runs
```bash
./target/release/werss-cli --mp all
```
→ Token is automatically loaded from keyring, no credentials needed

### How It Works
1. Check system keyring for existing token
2. If found and valid → use immediately
3. If found but expired → automatically refresh
4. If refresh fails or no token → use provided credentials or prompt for password
5. Token is automatically saved for next run

## Quick start

```bash
cargo build --release

# generate config template
./target/release/werss-cli --init-config

# edit werss.toml with your API credentials, then run
./target/release/werss-cli
```

## Configuration priority

```
CLI flags > Environment variables > werss.toml > .env > Built-in defaults
```

Minimal `werss.toml`:

```toml
[api]
base = "http://your-server:8001"
username = "your-username"
password = "your-password"

[sync]
target_mps = "all"   # or ["MP_WXS_123", "MP_WXS_456"]
```

## Examples

```bash
werss-cli                                    # fetch all (uses werss.toml + saved token)
werss-cli --mp MP_WXS_123,MP_WXS_456         # specific accounts
werss-cli --output ./data                     # custom output directory
werss-cli --since 2026-01-01 --until 2026-03-31  # date range
werss-cli --limit 10                          # max 10 articles
werss-cli --workspace ./workspace             # publish to workspace
```

## Documentation

| Page | Description |
|------|-------------|
| [Installation](docs/installation.md) | Build from source, binary releases |
| [Configuration](docs/configuration.md) | CLI flags, env vars, werss.toml reference |
| [Usage Guide](docs/usage.md) | Common workflows and examples |
| [Incremental Sync](docs/incremental-sync.md) | State tracking and retry behavior |
| [Output Format](docs/output-format.md) | Directory structure, frontmatter, workspace |
| [Architecture](docs/architecture.md) | Module design, data flow, concurrency model |
| [API Reference](docs/api-reference.md) | WeRSS API endpoints used by the CLI |
| [Troubleshooting](docs/troubleshooting.md) | Error messages, common issues, FAQ |

## Output

Articles are saved as:

```
articles/{mp_id}/YYYYMMDD/{seq}/{slug}.md
```

Each file has YAML frontmatter (title, author, coverImage, url, mp_id, description, publish_time) followed by the converted Markdown body.

## Dependencies

- [reqwest](https://crates.io/crates/reqwest) — HTTP client
- [tokio](https://crates.io/crates/tokio) — async runtime
- [clap](https://crates.io/crates/clap) — CLI argument parsing
- [html2md](https://crates.io/crates/html2md) — HTML to Markdown conversion
- [serde](https://crates.io/crates/serde) / [serde_json](https://crates.io/crates/serde_json) — serialization
- [chrono](https://crates.io/crates/chrono) — date/time handling
- [toml](https://crates.io/crates/toml) — config file parsing
- [anyhow](https://crates.io/crates/anyhow) — error handling
- [keyring](https://crates.io/crates/keyring) — secure credential storage (cross-platform)

> **Note:** `html2md` requires the `panic_unwind` runtime, so `panic = "abort"` cannot be used in the release profile.

## License

MIT

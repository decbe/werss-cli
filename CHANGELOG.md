# Changelog

All notable changes to werss-cli will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-04-07

### Added

- Fetch WeChat public account articles via WeRSS API
- HTML to Markdown conversion with YAML frontmatter
- Incremental sync with JSONL state tracking
- Parallel article fetching with concurrency control (max 3)
- Graceful shutdown on Ctrl+C
- Retry logic with configurable `max_failures`
- Automatic Bearer token refresh on 401
- Optional workspace publishing with cover image downloads
- Configuration via CLI flags, environment variables, TOML file, and `.env`
- Date range filtering (`--since`, `--until`)
- Article count limit (`--limit`)
- Page range control (`--start-page`, `--end-page`)
- Config template generation (`--init-config`)
- Automatic state file compaction
- Code block dedenting in converted Markdown
- WeChat UI artifact cleanup from article tails

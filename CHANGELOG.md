# Changelog

All notable changes to werss-cli will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- Complete authentication token management with system keyring storage
- Token refresh mechanism with automatic expiration detection (5-minute buffer)
- Interactive password input when credentials unavailable
- Graceful fallback authentication flow: saved token → refresh → credentials → interactive input
- Cross-platform support for secure token storage (Linux Secret Service, macOS Keychain, Windows Credential Manager)

### Changed

- Authentication system refactored to use `keyring` crate for secure token storage
- Login process now automatically saves tokens for reuse on subsequent runs
- Bearer token handling improved with optional refresh token support
- API compatibility enhanced to handle optional `refresh_token` field in login responses

### Fixed

- Handle APIs that don't provide `refresh_token` field in login response
- Proper error handling when refresh token operations fail

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

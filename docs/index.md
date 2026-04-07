# werss-cli Documentation

werss-cli is a Rust command-line tool that fetches articles from WeChat public accounts (微信公众号) via a WeRSS API server and saves them as Markdown files with YAML frontmatter.

## Table of Contents

| Page | Description |
|------|-------------|
| [Installation](installation.md) | Prerequisites, building from source, binary compression |
| [Configuration](configuration.md) | Full reference for CLI flags, env vars, werss.toml, and .env |
| [Usage Guide](usage.md) | Step-by-step guide, common workflows, and examples |
| [Incremental Sync](incremental-sync.md) | State tracking, retry logic, manual state operations |
| [Output Format](output-format.md) | Directory structure, frontmatter fields, workspace publishing |
| [Architecture](architecture.md) | Module design, execution flow, concurrency model |
| [API Reference](api-reference.md) | WeRSS API endpoints used by werss-cli |
| [Troubleshooting](troubleshooting.md) | Error diagnostics, common issues, FAQ |

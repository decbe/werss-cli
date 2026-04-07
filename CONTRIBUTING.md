# Contributing to werss-cli

Thank you for your interest in contributing! This guide covers the basics.

## Development setup

```bash
git clone https://github.com/your-org/werss-cli.git
cd werss-cli
cargo build
```

Run the binary directly:

```bash
cargo run -- --help
```

## Code structure

```
src/
├── main.rs     # CLI parsing, orchestration, graceful shutdown
├── config.rs   # Configuration loading and defaults
├── client.rs   # WeRSS API client (HTTP, auth, retry)
├── convert.rs  # HTML → Markdown conversion, slug, frontmatter
└── state.rs    # Incremental state tracking (JSONL)
```

See [docs/architecture.md](docs/architecture.md) for detailed module documentation.

## Making changes

1. **Fork** the repository
2. **Create a branch** for your change
3. **Make your changes** with clear, focused commits
4. **Test manually** — run against a WeRSS server and verify output
5. **Submit a pull request**

### Coding conventions

- Follow `rustfmt` formatting: `cargo fmt`
- Pass `clippy` with no warnings: `cargo clippy`
- Error handling via `anyhow` — use `.map_err()` to add context
- Keep the single-binary CLI philosophy: no external runtime dependencies

### Design principles

- **Zero-config by default** — sensible defaults, only require API credentials
- **Idempotent** — re-running should be safe and efficient
- **No external state** — all state is in the output directory (JSONL files)

## Submitting changes

When opening a pull request, please include:

- **What** changed
- **Why** it was needed
- **How** you tested it

## Reporting issues

Open a [GitHub issue](https://github.com/your-org/werss-cli/issues) with:

- werss-cli version
- Operating system and Rust version
- Steps to reproduce
- Expected vs actual behavior
- Relevant logs (redact any credentials)

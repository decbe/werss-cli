# AGENTS.md — werss-cli

## Build & run

```bash
cargo build --release          # binary at target/release/werss-cli
cargo run -- --help            # dev run
```

No test suite exists. Verify changes by building and running against a WeRSS server.

## Lint

```bash
cargo fmt    # formatting
cargo clippy # lints — must pass with zero warnings
```

## Architecture

Single-binary Rust CLI (edition 2021). All code lives in `src/`:

| File | Purpose |
|------|---------|
| `main.rs` | CLI parsing (clap), orchestration, graceful shutdown, workspace publishing |
| `config.rs` | Config loading (werss.toml → defaults), `--init-config` generation |
| `client.rs` | WeRSS API HTTP client: auth, retry, article listing/detail, image download |
| `convert.rs` | HTML → Markdown, slug generation, YAML frontmatter assembly |
| `state.rs` | Incremental state tracking (JSONL per MP directory) |

Data flow: `config → login → list MPs → sync pages → list articles → filter → fetch detail → convert → write .md → record state`

## Key constraints

- **`panic = "abort"` is forbidden** — the `html2md` crate requires the `panic_unwind` runtime. Do not add it to the release profile.
- **Config priority**: CLI flags > env vars (`WE_*`) > `werss.toml` > `.env` > defaults.
- **No external runtime dependencies** — keep the single-binary philosophy.
- **Error handling**: use `anyhow` with `.map_err()` for context; no custom error types.
- **State is JSONL** in the output directory (`articles/{mp_id}/state.jsonl`). State auto-compacts when lines exceed 2× unique articles.

## Config file

- `werss.toml` is gitignored (contains credentials). Use `werss.toml.example` as template.
- The binary also loads `.env` from the project root and `../werss/.env` (sibling workspace).
- `--init-config` generates a template; refuses to overwrite.

## Output layout

```
articles/{mp_id}/YYYYMMDD/{seq:02d}/{slug}.md
```

Frontmatter: title, author, coverImage, url, mp_id, description, publish_time (ISO 8601).
Slug: lowercase, alphanumeric + hyphens + CJK preserved, max 200 bytes.

## Documentation

Full docs in `docs/` (architecture, configuration, usage, incremental sync, output format, API reference, troubleshooting).
Agent skill definition in `skills/werss-cli/SKILL.md`.

# AGENTS.md — werss-cli

## Build & run

```bash
cargo build --release          # release artifacts remain under target/, binary at target/release/werss-cli
cargo run -- --help            # dev run
./target/release/werss-cli     # run built binary
```

Since there's no formal test suite, verification is done by building and running against a WeRSS server.

## Lint

```bash
cargo fmt    # formatting
cargo clippy # lints — must pass with zero warnings
```

## Code Style Guidelines

### Imports
- Group imports: standard library first, external crates, then local modules
- Use `use` declarations at the top of files
- Prefer importing specific items over glob imports when clarity is improved
- For related items from same module, use braces: `use std::{fs, path::Path};`
- External crates should be imported with explicit versions in Cargo.toml
- Avoid wildcard imports (`use crate::*`) in library code

### Formatting
- Run `cargo fmt` before committing code
- Follow Rustfmt defaults (4-space tabs)
- Maximum line length: 100 characters (standard Rust convention)
- Use explicit returns only when necessary for clarity
- Match existing code style when modifying files
- Use trailing commas in multi-line struct/enum definitions and function calls

### Types & Naming
- Use descriptive names for variables and functions
- Types and traits: PascalCase
- Functions and variables: snake_case
- Constants: SCREAMING_SNAKE_CASE
- Boolean variables: prefix with `is_`, `has_`, `should_`, etc.
- Error types: use `anyhow::Result<T>` for fallible operations
- Avoid single-letter names except for short-lived loop indices (i, j, k)
- Use meaningful names for configuration values and constants

### Error Handling
- Use `anyhow` crate for error handling: `anyhow::Result<T>`
- Add context with `.map_err()` or `.context()` when propagating errors
- Don't create custom error types unless absolutely necessary
- Handle errors at appropriate levels - don't just unwrap or expect in production code
- Use `?` operator for propagating errors
- For user-facing errors, provide clear, actionable messages
- Log unexpected errors at appropriate levels (error/warn/info/debug)

### Comments & Documentation
- Use line comments (`//`) for explanatory comments
- Use doc comments (`///`) for public API documentation
- Avoid obvious comments that just restate the code
- Document why something is done, not what is done
- Keep comments up-to-date when modifying code
- For complex logic, add brief explanations of the approach
- Use block comments (`/* ... */`) sparingly, mainly for license headers

### Async/Tokio
- Prefer async/await syntax over manual future combinators
- Use `tokio::spawn` for concurrent tasks
- Limit concurrency with semaphores when needed (see main.rs:398)
- Handle cancellation gracefully (see main.rs:379-383)
- Don't block the async runtime with synchronous operations
- Use appropriate tokio synchronization primitives (Mutex, Semaphore, etc.)
- Handle timeouts for external API calls
- Use tokio's sleep for delays rather than thread::sleep

### Testing
- While no formal test suite exists, add unit tests for complex logic
- Tests live in the same file as the code they test (inline modules)
- Use `#[cfg(test)]` and `mod tests {}` convention
- Focus on testing pure functions and edge cases
- For integration testing, verify behavior by running against a test WeRSS server
- Test both success and failure cases
- Use meaningful test names that describe what is being tested

### Configuration
- Follow the priority order: CLI flags > env vars (`WE_*`) > `werss.toml` > `.env` > defaults
- Use clap for CLI argument parsing with appropriate help text
- Provide sensible defaults for configuration values
- Document all configuration options in the documentation
- Use environment variable prefixes consistently (`WE_`)
- Make config loading robust to missing or malformed files

### Security
- Never log sensitive information (passwords, tokens)
- Use system keyring for credential storage when available
- Clear sensitive data from memory when no longer needed
- Validate all inputs, especially those from external sources
- Use HTTPS for API connections when possible
- Handle authentication failures gracefully

## Architecture

Single-binary Rust CLI (edition 2021). All code lives in `src/`:

| File | Purpose |
|------|---------|
| `main.rs` | CLI parsing (clap), orchestration, graceful shutdown, workspace publishing |
| `config.rs` | Config loading (werss.toml → defaults), `--init-config` generation |
| `client.rs` | WeRSS API HTTP client: auth, retry, article listing/detail, image download |
| `convert.rs` | HTML → Markdown, slug generation, YAML frontmatter assembly |
| `state.rs` | Incremental state tracking (JSONL per MP directory) |
| `auth.rs` | Authentication token management with keyring integration |

Data flow: `config → login → list MPs → sync pages → list articles → filter → fetch detail → convert → write .md → record state`

## Key Constraints

- **`panic = "abort"` is forbidden** — the `html2md` crate requires the `panic_unwind` runtime. Do not add it to the release profile.
- **Config priority**: CLI flags > env vars (`WE_*`) > `werss.toml` > `.env` > defaults.
- **No external runtime dependencies** — keep the single-binary philosophy.
- **Error handling**: use `anyhow` with `.map_err()` for context; no custom error types.
- **State is JSONL** in the output directory (`articles/{mp_id}/state.jsonl`). State auto-compacts when lines exceed 2× unique articles.

## Config File

- `werss.toml` is gitignored (contains credentials). Use `werss.toml.example` as template.
- The binary also loads `.env` from the project root and `../werss/.env` (sibling workspace).
- `--init-config` generates a template; refuses to overwrite.

## Output Layout

```
articles/{mp_id}/YYYYMMDD/{seq:02d}/{slug}.md
```

Frontmatter: title, author, coverImage, url, mp_id, description, publish_time (ISO 8601).
Slug: lowercase, alphanumeric + hyphens + CJK preserved, max 200 bytes.

## Documentation

Full docs in `docs/` (architecture, configuration, usage, incremental sync, output format, API reference, troubleshooting).
Agent skill definition in `skills/werss-cli/SKILL.md`.

## Development Workflow

1. Make changes to code
2. Run `cargo fmt` to format code
3. Run `cargo clippy` to check for lint warnings (must pass with zero warnings)
4. Build with `cargo build --release`
5. Test changes by running against a WeRSS server
6. Commit changes with descriptive message

When adding new features:
- Follow existing code patterns and conventions
- Update documentation in appropriate `docs/` files if needed
- Ensure error handling follows established patterns
- Consider incremental sync implications for new functionality
- Test thoroughly with both new and existing functionality
# Installation

## Prerequisites

- **Rust toolchain** — 1.70 or later. Install via [rustup](https://rustup.rs/):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **OpenSSL development headers** — required by the `native-tls` feature in `reqwest`:
  ```bash
  # Debian/Ubuntu
  sudo apt install libssl-dev

  # macOS (via Homebrew)
  brew install openssl
  ```
- **UPX** (optional) — for binary compression after building.

## Build from source

```bash
git clone https://github.com/your-org/werss-cli.git
cd werss-cli
cargo build --release
```

The binary is at `target/release/werss-cli`.

### Binary size

| Stage | Size |
|-------|------|
| Release build (`opt-level = "z"`, LTO, strip) | ~3.8 MB |
| After UPX compression | ~1.3 MB |

```bash
upx --best target/release/werss-cli
```

### Build notes

The `html2md` dependency requires the `panic_unwind` runtime. **Do not** set `panic = "abort"` in `Cargo.toml` — it will cause a compile error:

```
the linked panic runtime is not compiled with this crate's panic strategy
```

The project uses the default `panic = "unwind"`.

## Verifying the installation

```bash
werss-cli --version
werss-cli --help
```

## Cross-compilation

For targeting other platforms (e.g., `aarch64-unknown-linux-gnu`), use [cross](https://github.com/cross-rs/cross) or the standard Rust cross-compilation workflow:

```bash
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu
```

Note: `native-tls` may require target-specific OpenSSL libraries when cross-compiling.

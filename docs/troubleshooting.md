# Troubleshooting

## Error messages

| Error message | Cause | Solution |
|--------------|-------|----------|
| `Cannot connect to API at <url>` | API server is not running or URL is wrong | Check the server is running and verify `api.base` in config |
| `API returned non-JSON (HTTP ...)` | URL points to a non-API endpoint | Confirm the API base URL is correct (e.g., `http://host:8001`) |
| `Login failed (code=x)` | Wrong username or password | Check `api.username` and `api.password` |
| `No credentials provided and none found in config` | Credentials missing from all sources | Provide via CLI, environment, config file, or respond to the interactive prompt |
| `Failed to save token to keyring` | Keyring service unavailable or access denied | Ensure your system keyring service is running (GNOME Keyring on Linux, Keychain on macOS, Credential Manager on Windows) |
| `Failed to load token from keyring` | Keyring service unavailable or corruption | Try re-authenticating with `--username` and `--password` to resave the token |
| `Token refresh failed, re-authenticating` | Token expired and refresh was unsuccessful | The tool will automatically re-authenticate. This is normal behavior. |
| `No matching public accounts found` | MP IDs don't exist on the server | Use `target_mps = "all"` to discover available IDs |
| `No write permission to output directory` | Output directory is not writable | Change permissions with `chmod` or set a different `output_dir` |
| `Refresh task timeout` | WeChat content fetch took longer than 3 minutes | Re-run to retry. The article may become available later |
| `Refresh task failed` | Article content is unavailable on WeChat | The article will be retried until `max_failures` is reached |
| `No target MPs specified` | No MP IDs configured | Set `--mp`, `WE_TARGET_MPS`, or `target_mps` in werss.toml |
| `API base URL is empty` | No API URL provided | Set `--api-base` or `WE_API_BASE` or `api.base` in werss.toml |
| `API base URL must start with http:// or https://` | Invalid URL format | Ensure the URL includes the scheme (e.g., `http://localhost:8001`) |
| `Empty content after refresh` | Article body is empty | The article is marked as failed and retried on next run |
| `Empty title in article detail` | API returned an article with no title | The article is marked as failed |

## Common issues

### Binary is too large

The release build is optimized for size (`opt-level = "z"`, LTO, strip). You can further reduce it with UPX:

```bash
upx --best target/release/werss-cli
```

Or cross-compile with the `musl` target for a fully static binary.

### Build fails with panic strategy error

```
the linked panic runtime is not compiled with this crate's panic strategy
```

Do not set `panic = "abort"` in `Cargo.toml`. The `html2md` dependency requires `panic = "unwind"`.

### Articles not appearing

Check:
1. **Date filter** — `--since` / `--until` may be excluding articles
2. **Page range** — `start_page`/`end_page` may be too narrow (default is only page 0-1)
3. **State** — articles may already be `success` or `exhausted` in `state.jsonl`
4. **API sync** — `update_mp` may have failed (check for "Sync failed after 3 retries" in logs)

### State file growing large

State files are automatically compacted when the line count exceeds 2× the number of unique articles. No manual intervention needed.

### Token expiry

werss-cli automatically manages token expiry:
1. Checks token validity before each run (with 5-minute buffer)
2. If token is expired, attempts automatic refresh (if `refresh_token` is available)
3. If refresh fails, falls back to re-authentication using stored credentials
4. New token is automatically saved to system keyring

No manual intervention needed. The process is transparent to you.

### Credentials not saved between runs

If werss-cli prompts for credentials every run:
1. Check your system keyring service is running:
   - **Linux**: `systemctl status gnome-keyring-daemon` or `systemctl status kwalletd`
   - **macOS**: Keychain is typically always running
   - **Windows**: Credential Manager should be available
2. Try explicitly providing credentials to force resave:
   ```bash
   werss-cli --username admin --password secret
   ```
3. The token should now be saved and available on next run

## FAQ

**Q: Do I need to provide credentials every time?**

No. On the first run, you provide credentials once. They are used to obtain a token, which is then saved to your system keyring. On subsequent runs, werss-cli loads the token automatically and doesn't need credentials.

**Q: What if my keyring is not available?**

werss-cli will prompt you for credentials whenever the keyring is unavailable. You can also explicitly provide credentials via CLI flags or environment variables, which will always work.

**Q: Can I use werss-cli without a keyring system?**

The keyring integration is automatic and transparent. If your system doesn't have a keyring, werss-cli will fall back to prompting for credentials, but will still function normally.

**Q: Can I run werss-cli without a WeRSS server?**

No. werss-cli is a client for the WeRSS API. You need a running WeRSS server that handles WeChat scraping.

**Q: Can I fetch articles from a public WeChat URL directly?**

No. werss-cli works through the WeRSS API, which handles the WeChat scraping and provides structured data.

**Q: What if an article's content is empty?**

The article is marked as `failed` in the state file and retried on the next run (up to `max_failures`).

**Q: How do I reset everything and start fresh?**

Delete the output directory or individual `state.jsonl` files:

```bash
rm -rf articles/MP_ID/state.jsonl   # reset one MP
rm -rf articles/                     # reset everything
```

**Q: Can I change the concurrency limit?**

Not currently. The concurrency limit is hardcoded to 3 (`Semaphore::new(3)` in `main.rs`).

**Q: What does "exhausted" mean?**

An article is "exhausted" when it has failed `max_failures` times (default 3). It is permanently skipped to avoid wasting time on articles that are consistently unavailable. You can reset it by editing `state.jsonl` or deleting the file.

## Getting help

Open a [GitHub issue](https://github.com/your-org/werss-cli/issues) with:

- werss-cli version (`werss-cli --version`)
- Rust version (`rustc --version`)
- Operating system
- Relevant configuration (redact credentials)
- Full error output

# Troubleshooting

## Error messages

| Error message | Cause | Solution |
|--------------|-------|----------|
| `Cannot connect to API at <url>` | API server is not running or URL is wrong | Check the server is running and verify `api.base` in config |
| `API returned non-JSON (HTTP ...)` | URL points to a non-API endpoint | Confirm the API base URL is correct (e.g., `http://host:8001`) |
| `Login failed (code=x)` | Wrong username or password | Check `api.username` and `api.password` |
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

werss-cli automatically re-authenticates on 401 responses. No configuration or manual intervention needed.

## FAQ

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

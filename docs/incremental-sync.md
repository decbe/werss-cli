# Incremental Sync

werss-cli tracks the state of each article to avoid redundant work. State is stored per-MP in a JSONL file.

## How it works

Each MP has a state file at:

```
{output_dir}/{mp_id}/state.jsonl
```

Every line is a JSON record:

```json
{
  "article_id": "3079106503-2451582093_1",
  "title": "Article Title",
  "publish_time": 1773847920,
  "status": "success",
  "file_path": "20260402/01/article-slug.md",
  "failed_count": 0,
  "updated_at": 1744012800
}
```

### Status flow

```
(not in state) ──fetch──> success  (skip on future runs)
                 │
                 └──fail──> failed  (retry on future runs)
                              │
                    failed_count >= max_failures
                              │
                              └──> exhausted  (skip permanently)
```

- **`success`** — Article has been fetched and saved. Future runs skip it.
- **`failed`** — Last fetch attempt failed. Future runs retry it (up to `max_failures`).
- **exhausted** (implied) — A failed article where `failed_count >= max_failures`. Skipped permanently.

## Retry behavior

The `max_failures` config (default: `3`) controls how many times a failed article is retried:

```toml
[sync]
max_failures = 3   # give up after 3 failures
max_failures = 0   # retry forever (never exhaust)
```

- Each failed fetch increments `failed_count` by 1.
- A successful fetch resets `failed_count` to 0.
- When `max_failures = 0`, articles are never marked as exhausted.

## Automatic compaction

State files are append-only JSONL. On load, if the total line count exceeds 2× the number of unique article IDs, werss-cli automatically compacts the file — keeping only the latest record per article.

This happens transparently and requires no user action.

## Manual state operations

### Retry all failed articles

Just re-run. Articles with `status: "failed"` and `failed_count < max_failures` are automatically retried.

### Force re-fetch a specific article

Remove its record from the state file:

```bash
sed -i '/ARTICLE_ID/d' articles/MP_ID/state.jsonl
werss-cli
```

### Reset failure count for an article

Allow an exhausted article to be retried:

```bash
sed -i 's/"failed_count":3/"failed_count":0/' articles/MP_ID/state.jsonl
werss-cli
```

### Full re-fetch for an MP

Delete the entire state file:

```bash
rm articles/MP_ID/state.jsonl
werss-cli
```

> **Warning:** Do not edit `state.jsonl` while werss-cli is running. Wait for the process to complete first.

## State file location

State files are always located relative to the output directory:

```
{output_dir}/{mp_id}/state.jsonl
```

This cannot be configured separately. The output directory is set via `--output`, `WE_OUTPUT_DIR`, or `werss.toml` `sync.output_dir`.

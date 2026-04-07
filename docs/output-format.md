# Output Format

## Directory structure

Articles are saved under the output directory:

```
{output_dir}/{mp_id}/YYYYMMDD/{seq:02d}/{slug}.md
```

| Component | Description |
|-----------|-------------|
| `mp_id` | WeRSS internal ID (e.g., `MP_WXS_3079106503`) |
| `YYYYMMDD` | Article publish date |
| `seq` | Zero-based sequence number per date (increments for each article on the same date) |
| `slug` | URL-safe title, max 200 bytes, Chinese characters preserved |

Example:

```
articles/
└── MP_WXS_3079106503/
    ├── 20260402/
    │   └── 01/
    │       └── ai-agent-skill-攻击真实示例与防御措施.md
    ├── 20260405/
    │   ├── 01/
    │   │   └── qwen3.5-本地部署实测.md
    │   └── 02/
    │       └── 从零开始搭建知识库.md
    └── state.jsonl
```

## Markdown file format

Each `.md` file contains YAML frontmatter followed by the converted Markdown body:

```markdown
---
title: "AI Agent Skill 攻击真实示例与防御措施"
author: "逻辑仓管AI运营社"
coverImage: "https://mmbiz.qpic.cn/sz_mmbiz_jpg/.../0?wx_fmt=jpeg"
url: "https://mp.weixin.qq.com/s/Q7iTpH4eVMT-TvWAABCJUQ"
mp_id: "MP_WXS_3079106503"
description: "像 openclaw 这种高度自主完成任务的产品..."
publish_time: "2026-04-02T00:30:00"
---

Article body in Markdown...
```

### Frontmatter fields

| Field | Type | Source | Notes |
|-------|------|--------|-------|
| `title` | string | `detail.title` | Quotes escaped |
| `author` | string | MP name | The public account name, not the article author |
| `coverImage` | string | `detail.pic_url` | Direct URL from API; omitted if empty |
| `url` | string | `detail.url` | Original WeChat article link |
| `mp_id` | string | `detail.mp_id` | WeRSS internal ID |
| `description` | string | `detail.description` | May contain escaped HTML tags; omitted if empty |
| `publish_time` | string | `detail.publish_time` | ISO 8601 format (`YYYY-MM-DDTHH:MM:SS`); omitted if timestamp is 0 |

## HTML to Markdown conversion

### Content priority

The converter tries HTML sources in this order:

1. **`content`** — clean body HTML from the API (preferred)
2. **`content_html`** — full HTML including cover image, h1 title, and author line (fallback; header is stripped)
3. **Empty** — if both are empty, the article is marked as failed

### Header stripping

When using `content_html`, the `strip_content_html_header()` function removes:

- Cover image: `<p><img alt="cover_image" ...></p>`
- H1 title: `<h1>...</h1>`
- Author line: `<p>原创...</p>`

### Tail cleanup

After HTML-to-Markdown conversion, `clean_tail()` removes WeChat UI artifacts:

- "预览时标签不可点"
- "微信扫一扫"
- "使用小程序" / "打开小程序"
- Isolated UI buttons: "知道了", "赞", "在看", "分享", "留言", "收藏", etc.

### Code block dedenting

Code blocks in the converted Markdown are automatically dedented to remove common leading whitespace.

### Slug generation

The `slugify()` function:

1. Converts to lowercase
2. Keeps alphanumeric characters, hyphens, and CJK characters (U+4E00..U+9FFF)
3. Replaces whitespace with hyphens
4. Truncates to 200 bytes (at character boundaries)
5. Falls back to `"article"` if the result is empty

## Workspace publishing (optional)

Enabled by setting `--workspace` or `sync.workspace_dir`. Each article is also published to:

```
{workspace}/published/YYYYMMDD/{slug}/{slug}.md
{workspace}/published/YYYYMMDD/{slug}/imgs/cover.{ext}
```

- The cover image is downloaded from `pic_url` (not extracted from HTML).
- Image extension is inferred from the URL: `.jpg`/`.jpeg` → `jpg`, `.webp` → `webp`, `.gif` → `gif`, default → `png`.
- If the cover image download fails, the article is still published but `ws_failed` is incremented in the summary.

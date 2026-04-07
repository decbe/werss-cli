# WeRSS API Reference

This documents the WeRSS API endpoints that werss-cli actually calls. For the full API schema, see the `openapi.json` file in the repository.

## Authentication

### POST `/api/v1/wx/auth/login`

Form-encoded login. Returns a Bearer token.

**Request:**

```
POST /api/v1/wx/auth/login
Content-Type: application/x-www-form-urlencoded

username=admin&password=secret
```

**Response:**

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
  }
}
```

All subsequent requests use `Authorization: Bearer <access_token>`. werss-cli automatically re-authenticates on 401 responses.

## Public accounts

### GET `/api/v1/wx/mps?limit=100&offset=0`

List all public accounts. Auto-paginated by werss-cli.

**Response:**

```json
{
  "code": 0,
  "data": {
    "list": [
      { "id": "MP_WXS_3079106503", "mp_name": "逻辑仓管AI运营社" }
    ],
    "total": 5
  }
}
```

### GET `/api/v1/wx/mps/update/{mp_id}?start_page=0&end_page=1`

Trigger article sync from WeChat for a specific MP. werss-cli retries this up to 3 times with 5-second delays.

## Articles

### POST `/api/v1/wx/articles?limit=100&offset=0&mp_id={mp_id}`

List articles for a specific MP. Auto-paginated by werss-cli.

**Response:**

```json
{
  "code": 0,
  "data": {
    "list": [
      {
        "id": "3079106503-2451582093_1",
        "title": "Article Title",
        "publish_time": 1773847920
      }
    ],
    "total": 42
  }
}
```

### POST `/api/v1/wx/articles/{article_id}/refresh`

Trigger content refresh for an article. Returns a `task_id` for polling.

**Response:**

```json
{
  "code": 0,
  "data": {
    "task_id": "abc123"
  }
}
```

### GET `/api/v1/wx/articles/refresh/tasks/{task_id}`

Poll the refresh task status. werss-cli polls every 3 seconds with a 180-second timeout.

**Response:**

```json
{
  "data": {
    "status": "success"
  }
}
```

Status values: `"pending"`, `"processing"`, `"success"`, `"failed"`.

### GET `/api/v1/wx/articles/{article_id}?content=true`

Get full article detail including HTML content.

**Response:**

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "id": "3079106503-2451582093_1",
    "mp_id": "MP_WXS_3079106503",
    "title": "对本地养虾不死心，Nanoclaw 实测 Qwen3.5-9B-Claude-4.6-Opus 量化版",
    "pic_url": "https://mmbiz.qpic.cn/sz_mmbiz_jpg/.../0?wx_fmt=jpeg",
    "url": "https://mp.weixin.qq.com/s/TJxmO6uKrtcZ52aczSogow",
    "description": "各大平台 coding plan 纷纷涨价...",
    "content": "<p>Clean body HTML...</p>",
    "content_html": "<p>Full HTML with cover + h1 + author...</p>",
    "publish_time": 1773847920,
    "status": 1,
    "created_at": "2026-03-19T15:33:25.120210",
    "updated_at": 1775401843
  }
}
```

### Content fields

| Field | Description |
|-------|-------------|
| `content` | Clean body HTML (preferred source for conversion) |
| `content_html` | Full HTML including cover image, h1 title, and author line |
| `pic_url` | Cover image URL (used for workspace publishing) |
| `publish_time` | Unix timestamp |

## Self-hosting

werss-cli requires a running WeRSS API server. The server handles WeChat scraping and provides the REST API that werss-cli consumes.

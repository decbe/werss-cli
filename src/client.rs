use anyhow::{anyhow, Result};
use std::time::Duration;

pub struct WeClient {
    base: String,
    http: reqwest::Client,
    user: String,
    pass: String,
    token: std::sync::Mutex<String>,
}

#[derive(Debug, Clone)]
pub struct MpInfo {
    pub id: String,
    pub mp_name: String,
}

#[derive(Debug, Clone)]
pub struct ArticleInfo {
    pub id: String,
    pub title: String,
    pub publish_time: i64,
}

pub struct ArticleDetail {
    #[allow(dead_code)]
    pub id: String,
    pub mp_id: String,
    pub title: String,
    pub url: String,
    pub pic_url: String,
    pub description: String,
    pub content: String,
    pub content_html: String,
    pub publish_time: i64,
}

async fn login(http: &reqwest::Client, base: &str, user: &str, pass: &str) -> Result<String> {
    const MAX_RETRIES: u32 = 3;
    for attempt in 1..=MAX_RETRIES {
        let resp_result = http
            .post(format!("{}/api/v1/wx/auth/login", base))
            .form(&[("username", user), ("password", pass)])
            .send()
            .await;

        let resp = match resp_result {
            Ok(resp) => resp,
            Err(e) => {
                if attempt < MAX_RETRIES && (e.is_connect() || e.is_timeout()) {
                    eprintln!(
                        "[WARN] Login attempt {}/{} failed: {}. Retrying...",
                        attempt, MAX_RETRIES, e
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    continue;
                }
                return Err(if e.is_connect() {
                    anyhow!("Cannot connect to API at {}. Is the server running?", base)
                } else {
                    anyhow!("Login request failed: {}", e)
                });
            }
        };

        let status = resp.status();
        let body = resp.text().await?;
        let json: Result<serde_json::Value> = serde_json::from_str(&body).map_err(|_| {
            anyhow!(
                "API returned non-JSON (HTTP {}): {}...",
                status,
                body.chars().take(200).collect::<String>()
            )
        });

        let resp = match json {
            Ok(value) => value,
            Err(_e) if attempt < MAX_RETRIES && status.is_server_error() => {
                eprintln!(
                    "[WARN] Login returned HTTP {} with invalid body. Retrying...",
                    status
                );
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                continue;
            }
            Err(e) => return Err(e),
        };

        if resp["code"].as_i64() != Some(0) {
            return Err(anyhow!(
                "Login failed (code={}): {}",
                resp["code"],
                resp["message"].as_str().unwrap_or("unknown")
            ));
        }

        return resp
            .pointer("/data/access_token")
            .and_then(|v| v.as_str())
            .map(String::from)
            .ok_or_else(|| anyhow!("Login succeeded but no access_token in response"));
    }

    Err(anyhow!(
        "Login failed after {} attempts due to transient API errors.",
        MAX_RETRIES
    ))
}

impl WeClient {
    pub async fn new(base: &str, user: &str, pass: &str) -> Result<Self> {
        let base = base.trim_end_matches('/');
        if base.is_empty() {
            return Err(anyhow!(
                "API base URL is empty. Set --api-base or WE_API_BASE"
            ));
        }
        if !base.starts_with("http://") && !base.starts_with("https://") {
            return Err(anyhow!(
                "API base URL must start with http:// or https://, got: {}",
                base
            ));
        }
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        let token = login(&http, base, user, pass).await?;
        Ok(Self {
            base: base.to_string(),
            http,
            user: user.into(),
            pass: pass.into(),
            token: std::sync::Mutex::new(token),
        })
    }

    async fn req(&self, method: &str, path: &str, retry: bool) -> Result<serde_json::Value> {
        let token = self.token.lock().unwrap().clone();
        let url = format!("{}{}", self.base, path);
        let b = match method {
            "POST" => self.http.post(&url),
            _ => self.http.get(&url),
        };
        let resp = b.bearer_auth(&token).send().await.map_err(|e| {
            if e.is_connect() {
                anyhow!("Connection lost to {}", self.base)
            } else if e.is_timeout() {
                anyhow!("Request timed out: {} {}", method, path)
            } else {
                anyhow!("Request failed: {}", e)
            }
        })?;
        if resp.status() == 401 && retry {
            let t = login(&self.http, &self.base, &self.user, &self.pass).await?;
            *self.token.lock().unwrap() = t.clone();
            let b = match method {
                "POST" => self.http.post(&url),
                _ => self.http.get(&url),
            };
            let resp = b.bearer_auth(&t).send().await?;
            let status = resp.status();
            let body = resp.text().await?;
            return serde_json::from_str(&body).map_err(|_| {
                anyhow!(
                    "Non-JSON response after re-auth (HTTP {}): {}...",
                    status,
                    body.chars().take(200).collect::<String>()
                )
            });
        }
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!(
                "API error HTTP {} on {} {}: {}",
                status,
                method,
                path,
                body.chars().take(300).collect::<String>()
            ));
        }
        let body = resp.text().await?;
        serde_json::from_str(&body).map_err(|_| {
            anyhow!(
                "Non-JSON response (HTTP {}) from {} {}: {}...",
                status,
                method,
                path,
                body.chars().take(200).collect::<String>()
            )
        })
    }

    pub async fn list_mps(&self) -> Result<Vec<MpInfo>> {
        let mut out = Vec::new();
        let mut off = 0i64;
        loop {
            let d = self
                .req(
                    "GET",
                    &format!("/api/v1/wx/mps?limit=100&offset={}", off),
                    true,
                )
                .await?;
            let list = d
                .pointer("/data/list")
                .or_else(|| d.pointer("/data/mps"))
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let total = d
                .pointer("/data/total")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            for i in &list {
                out.push(MpInfo {
                    id: i["id"].as_str().unwrap_or_default().into(),
                    mp_name: i["mp_name"].as_str().unwrap_or_default().into(),
                });
            }
            if out.len() as i64 >= total || list.len() < 100 {
                break;
            }
            off += 100;
        }
        Ok(out)
    }

    pub async fn update_mp(&self, mp_id: &str, start_page: i64, end_page: i64) -> Result<()> {
        self.req(
            "GET",
            &format!(
                "/api/v1/wx/mps/update/{}?start_page={}&end_page={}",
                mp_id, start_page, end_page
            ),
            true,
        )
        .await?;
        Ok(())
    }

    pub async fn list_articles(&self, mp_id: &str) -> Result<Vec<ArticleInfo>> {
        let mut out = Vec::new();
        let mut off = 0i64;
        loop {
            let d = self
                .req(
                    "POST",
                    &format!(
                        "/api/v1/wx/articles?limit=100&offset={}&mp_id={}",
                        off, mp_id
                    ),
                    true,
                )
                .await?;
            let list = d
                .pointer("/data/list")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let total = d
                .pointer("/data/total")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            for i in &list {
                out.push(ArticleInfo {
                    id: i["id"].as_str().unwrap_or_default().into(),
                    title: i["title"].as_str().unwrap_or_default().into(),
                    publish_time: i["publish_time"].as_i64().unwrap_or(0),
                });
            }
            if out.len() as i64 >= total || list.len() < 100 {
                break;
            }
            off += 100;
        }
        Ok(out)
    }

    pub async fn refresh_article(&self, aid: &str) -> Result<Option<String>> {
        let d = self
            .req(
                "POST",
                &format!("/api/v1/wx/articles/{}/refresh", aid),
                true,
            )
            .await?;
        if d["code"].as_i64() != Some(0) {
            return Ok(None);
        }
        Ok(d.pointer("/data/task_id")
            .and_then(|v| v.as_str())
            .map(String::from))
    }

    pub async fn poll_task(&self, tid: &str, interval: u64, max_wait: u64) -> Result<String> {
        let start = std::time::Instant::now();
        let mut consecutive_errors = 0u32;
        loop {
            match self
                .req(
                    "GET",
                    &format!("/api/v1/wx/articles/refresh/tasks/{}", tid),
                    true,
                )
                .await
            {
                Ok(d) => {
                    consecutive_errors = 0;
                    let s = d
                        .pointer("/data/status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("failed")
                        .to_string();
                    if s == "success" || s == "failed" {
                        return Ok(s);
                    }
                }
                Err(e) => {
                    consecutive_errors += 1;
                    if consecutive_errors >= 5 {
                        return Err(e);
                    }
                    eprintln!(
                        "  [WARN] Poll error ({}/5), retrying: {}",
                        consecutive_errors, e
                    );
                }
            }
            if start.elapsed().as_secs() >= max_wait {
                return Ok("timeout".into());
            }
            tokio::time::sleep(Duration::from_secs(interval)).await;
        }
    }

    pub async fn get_article_detail(&self, aid: &str) -> Result<ArticleDetail> {
        let d = self
            .req(
                "GET",
                &format!("/api/v1/wx/articles/{}?content=true", aid),
                true,
            )
            .await?;
        let d = &d["data"];
        Ok(ArticleDetail {
            id: d["id"].as_str().unwrap_or_default().into(),
            mp_id: d["mp_id"].as_str().unwrap_or_default().into(),
            title: d["title"].as_str().unwrap_or_default().into(),
            url: d["url"].as_str().unwrap_or_default().into(),
            pic_url: d["pic_url"].as_str().unwrap_or_default().into(),
            description: d["description"].as_str().unwrap_or_default().into(),
            content: d["content"].as_str().unwrap_or_default().into(),
            content_html: d["content_html"].as_str().unwrap_or_default().into(),
            publish_time: d["publish_time"].as_i64().unwrap_or(0),
        })
    }

    pub async fn download_image(&self, url: &str) -> Result<Vec<u8>> {
        let resp = self
            .http
            .get(url)
            .timeout(Duration::from_secs(30))
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(anyhow!("Image download failed (HTTP {}): {}", status, url));
        }
        Ok(resp.bytes().await?.to_vec())
    }
}

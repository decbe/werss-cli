use anyhow::Result;
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

struct Rec {
    status: String,
    file_path: String,
    title: String,
    failed_count: u32,
}

pub struct StateStore {
    mp_dir: PathBuf,
    state_file: PathBuf,
    fetched: HashMap<String, Rec>,
}

impl StateStore {
    pub fn new(mp_id: &str, output_dir: &str) -> Result<Self> {
        let mp_dir = Path::new(output_dir).join(mp_id);
        std::fs::create_dir_all(&mp_dir)?;
        let state_file = mp_dir.join("state.jsonl");
        let mut fetched = HashMap::new();
        let mut line_count = 0usize;
        if state_file.exists() {
            for line in std::io::BufReader::new(std::fs::File::open(&state_file)?).lines() {
                let line = line?;
                if line.is_empty() {
                    continue;
                }
                line_count += 1;
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                    let id = v["article_id"].as_str().unwrap_or_default().to_string();
                    fetched.insert(
                        id,
                        Rec {
                            status: v["status"].as_str().unwrap_or_default().into(),
                            file_path: v["file_path"].as_str().unwrap_or_default().into(),
                            title: v["title"].as_str().unwrap_or_default().into(),
                            failed_count: v["failed_count"].as_u64().unwrap_or(0) as u32,
                        },
                    );
                }
            }
        }
        let store = Self {
            mp_dir,
            state_file,
            fetched,
        };
        if line_count > 0 && line_count > store.fetched.len() * 2 {
            store.compact();
        }
        Ok(store)
    }

    pub fn is_fetched(&self, id: &str) -> bool {
        self.fetched
            .get(id)
            .map(|r| r.status == "success")
            .unwrap_or(false)
    }

    pub fn is_exhausted(&self, id: &str, max: u32) -> bool {
        if max == 0 {
            return false;
        }
        self.fetched
            .get(id)
            .map(|r| r.status == "failed" && r.failed_count >= max)
            .unwrap_or(false)
    }

    pub fn record(&mut self, id: &str, title: &str, ts: i64, status: &str, file_path: &str) -> Result<()> {
        let current_count = self.fetched.get(id).map(|r| r.failed_count).unwrap_or(0);
        let new_count = if status == "failed" { current_count + 1 } else { 0 };
        self.fetched.insert(
            id.to_string(),
            Rec {
                status: status.into(),
                file_path: file_path.to_string(),
                title: title.into(),
                failed_count: new_count,
            },
        );
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let rec = serde_json::json!({
            "article_id": id,
            "title": title,
            "publish_time": ts,
            "status": status,
            "file_path": file_path,
            "failed_count": new_count,
            "updated_at": now
        });
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.state_file)?;
        writeln!(f, "{}", rec)?;
        Ok(())
    }

    fn compact(&self) {
        let tmp = self.state_file.with_extension("jsonl.tmp");
        match std::fs::File::create(&tmp) {
            Ok(mut f) => {
                for (id, rec) in &self.fetched {
                    let entry = serde_json::json!({
                        "article_id": id,
                        "title": rec.title,
                        "status": rec.status,
                        "file_path": rec.file_path,
                        "failed_count": rec.failed_count,
                    });
                    let _ = writeln!(f, "{}", entry);
                }
                drop(f);
                if std::fs::rename(&tmp, &self.state_file).is_err() {
                    let _ = std::fs::remove_file(&tmp);
                }
            }
            Err(_) => {}
        }
    }

    pub fn mp_dir(&self) -> &Path {
        &self.mp_dir
    }

    pub fn resolve_article_dir(&self, id: &str, title: &str, ts: i64) -> Result<PathBuf> {
        if let Some(r) = self.fetched.get(id) {
            if !r.file_path.is_empty() {
                let p = self.mp_dir.join(&r.file_path);
                if p.exists() {
                    return Ok(p.parent().unwrap().to_path_buf());
                }
            }
        }
        for rec in self.fetched.values() {
            if rec.title == title && !rec.file_path.is_empty() {
                let p = self.mp_dir.join(&rec.file_path);
                if p.exists() {
                    return Ok(p.parent().unwrap().to_path_buf());
                }
            }
        }
        let date = if ts > 0 {
            chrono::DateTime::from_timestamp(ts, 0)
                .map(|d| d.format("%Y%m%d").to_string())
                .unwrap_or_else(|| "unknown".into())
        } else {
            "unknown".into()
        };
        let dd = self.mp_dir.join(&date);
        let n = if dd.exists() {
            std::fs::read_dir(&dd)
                .map(|r| {
                    r.filter_map(|e| e.ok())
                        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                        .count()
                })
                .unwrap_or(0)
        } else {
            0
        };
        let target = dd.join(format!("{:02}", n + 1));
        std::fs::create_dir_all(&target)?;
        Ok(target)
    }
}

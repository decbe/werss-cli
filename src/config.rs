use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub api: ApiConfig,
    pub sync: SyncConfig,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    pub base: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SyncConfig {
    pub target_mps: TomlVecOrString,
    pub output_dir: String,
    pub workspace_dir: String,
    pub max_failures: u32,
    pub since: String,
    pub until: String,
    pub limit: u32,
    pub start_page: i64,
    pub end_page: i64,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            target_mps: TomlVecOrString::String("all".into()),
            output_dir: "./articles".into(),
            workspace_dir: String::new(),
            max_failures: 3,
            since: String::new(),
            until: String::new(),
            limit: 0,
            start_page: 0,
            end_page: 1,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum TomlVecOrString {
    String(String),
    Array(Vec<String>),
}

impl TomlVecOrString {
    pub fn to_comma_string(&self) -> String {
        match self {
            TomlVecOrString::String(s) => s.clone(),
            TomlVecOrString::Array(v) => v.join(","),
        }
    }
}

impl Config {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let cfg: Self = toml::from_str(&content)?;
        Ok(cfg)
    }

    pub fn load_optional(path: &std::path::Path) -> Self {
        if path.exists() {
            match Self::load(path) {
                Ok(cfg) => {
                    eprintln!("Config loaded from {}", path.display());
                    return cfg;
                }
                Err(e) => {
                    eprintln!(
                        "[WARN] Failed to parse {}: {}. Using defaults.",
                        path.display(),
                        e
                    );
                }
            }
        }
        Self::default()
    }
}

pub fn generate_example() -> String {
    r#"# werss-cli configuration

[api]
base = "http://your-server:8001"
username = "your-username"
password = "your-password"

[sync]
# "all" to fetch every MP, or a list of IDs
target_mps = "all"
# target_mps = ["MP_WXS_3079106503", "MP_WXS_3540720447"]
output_dir = "./articles"
workspace_dir = ""
# Max retry count for failed articles (0 = retry forever)
max_failures = 3
# Only fetch articles published since this date (YYYY-MM-DD)
since = ""
# Only fetch articles published until this date (YYYY-MM-DD)
until = ""
# Max number of articles to fetch per run (0 = no limit)
limit = 0
# Page range for sync (start_page=0, end_page=1 means only latest page)
start_page = 0
end_page = 1
"#
    .trim()
    .to_string()
}

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

const KEYRING_SERVICE: &str = "werss-cli";
const ACCESS_TOKEN_KEY: &str = "access_token";
const REFRESH_TOKEN_KEY: &str = "refresh_token";
const TOKEN_EXPIRES_AT_KEY: &str = "token_expires_at";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}

impl TokenData {
    /// Create a new TokenData from API response
    pub fn from_response(resp: &serde_json::Value) -> Result<Self> {
        let access_token = resp
            .pointer("/data/access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("No access_token in response"))?
            .to_string();

        // refresh_token is optional - some APIs may not provide it
        let refresh_token = resp
            .pointer("/data/refresh_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default();

        let expires_in = resp
            .pointer("/data/expires_in")
            .and_then(|v| v.as_i64())
            .unwrap_or(86400); // default 24 hours

        let now = current_timestamp();
        let expires_at = now + expires_in;

        Ok(TokenData {
            access_token,
            refresh_token,
            expires_at,
        })
    }

    /// Check if token is still valid (with 5-minute buffer)
    pub fn is_valid(&self) -> bool {
        let now = current_timestamp();
        self.expires_at > now + 300 // 5-minute buffer
    }

    /// Save tokens to system keyring
    pub fn save(&self) -> Result<()> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, ACCESS_TOKEN_KEY)
            .map_err(|e| anyhow!("Failed to access keyring: {}", e))?;
        entry
            .set_password(&self.access_token)
            .map_err(|e| anyhow!("Failed to save access_token to keyring: {}", e))?;

        let entry = keyring::Entry::new(KEYRING_SERVICE, REFRESH_TOKEN_KEY)
            .map_err(|e| anyhow!("Failed to access keyring: {}", e))?;
        entry
            .set_password(&self.refresh_token)
            .map_err(|e| anyhow!("Failed to save refresh_token to keyring: {}", e))?;

        let entry = keyring::Entry::new(KEYRING_SERVICE, TOKEN_EXPIRES_AT_KEY)
            .map_err(|e| anyhow!("Failed to access keyring: {}", e))?;
        entry
            .set_password(&self.expires_at.to_string())
            .map_err(|e| anyhow!("Failed to save token_expires_at to keyring: {}", e))?;

        Ok(())
    }

    /// Load tokens from system keyring
    pub fn load() -> Result<Option<Self>> {
        let access_token = match keyring::Entry::new(KEYRING_SERVICE, ACCESS_TOKEN_KEY) {
            Ok(entry) => match entry.get_password() {
                Ok(pwd) => pwd,
                Err(keyring::error::Error::NoEntry) => return Ok(None),
                Err(e) => return Err(anyhow!("Failed to read access_token from keyring: {}", e)),
            },
            Err(e) => return Err(anyhow!("Failed to access keyring: {}", e)),
        };

        let refresh_token = match keyring::Entry::new(KEYRING_SERVICE, REFRESH_TOKEN_KEY) {
            Ok(entry) => match entry.get_password() {
                Ok(pwd) => pwd,
                Err(keyring::error::Error::NoEntry) => return Ok(None),
                Err(e) => return Err(anyhow!("Failed to read refresh_token from keyring: {}", e)),
            },
            Err(e) => return Err(anyhow!("Failed to access keyring: {}", e)),
        };

        let expires_at = match keyring::Entry::new(KEYRING_SERVICE, TOKEN_EXPIRES_AT_KEY) {
            Ok(entry) => match entry.get_password() {
                Ok(pwd) => pwd.parse::<i64>().unwrap_or(0),
                Err(keyring::error::Error::NoEntry) => 0,
                Err(e) => {
                    return Err(anyhow!(
                        "Failed to read token_expires_at from keyring: {}",
                        e
                    ))
                }
            },
            Err(e) => return Err(anyhow!("Failed to access keyring: {}", e)),
        };

        Ok(Some(TokenData {
            access_token,
            refresh_token,
            expires_at,
        }))
    }

    /// Delete tokens from system keyring
    #[allow(dead_code)]
    pub fn delete() -> Result<()> {
        let _ = keyring::Entry::new(KEYRING_SERVICE, ACCESS_TOKEN_KEY)
            .and_then(|e| e.delete_credential());
        let _ = keyring::Entry::new(KEYRING_SERVICE, REFRESH_TOKEN_KEY)
            .and_then(|e| e.delete_credential());
        let _ = keyring::Entry::new(KEYRING_SERVICE, TOKEN_EXPIRES_AT_KEY)
            .and_then(|e| e.delete_credential());
        Ok(())
    }
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

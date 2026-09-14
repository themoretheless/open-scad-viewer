use crate::{invalid, Result};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OctoPrintConfig {
    pub base_url: String,
    pub api_key: String,
    pub printer_name: String,
    pub timeout: Duration,
}

impl OctoPrintConfig {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            api_key: api_key.into(),
            printer_name: "octoprint".into(),
            timeout: Duration::from_secs(30),
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.base_url.is_empty() {
            return Err(invalid(
                "PRINTER_HOST",
                "OctoPrint base_url must be non-empty",
            ));
        }
        if !(self.base_url.starts_with("http://") || self.base_url.starts_with("https://")) {
            return Err(invalid(
                "PRINTER_HOST",
                "OctoPrint base_url must start with http:// or https://",
            ));
        }
        if self.api_key.is_empty() || self.api_key.len() > 128 {
            return Err(invalid(
                "PRINTER_API_KEY",
                "OctoPrint api_key must be 1..128 characters",
            ));
        }
        Ok(())
    }
}

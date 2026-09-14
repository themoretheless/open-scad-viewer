use crate::{invalid, Result};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrusaLinkConfig {
    pub base_url: String,
    /// Prefer API key when set; Digest auth is out of scope for this pass.
    pub api_key: String,
    pub storage: String,
    pub printer_name: String,
    pub timeout: Duration,
    pub overwrite: bool,
    pub print_after_upload: bool,
}

impl PrusaLinkConfig {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            api_key: api_key.into(),
            storage: "local".into(),
            printer_name: "prusa".into(),
            timeout: Duration::from_secs(30),
            overwrite: true,
            print_after_upload: true,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.base_url.is_empty() {
            return Err(invalid(
                "PRINTER_HOST",
                "PrusaLink base_url must be non-empty",
            ));
        }
        if !(self.base_url.starts_with("http://") || self.base_url.starts_with("https://")) {
            return Err(invalid(
                "PRINTER_HOST",
                "PrusaLink base_url must start with http:// or https://",
            ));
        }
        if self.api_key.is_empty() || self.api_key.len() > 128 {
            return Err(invalid(
                "PRINTER_API_KEY",
                "PrusaLink api_key must be 1..128 characters",
            ));
        }
        if self.storage.is_empty()
            || self.storage.contains('/')
            || self.storage.contains('\\')
            || self.storage.contains("..")
        {
            return Err(invalid(
                "PRINTER_ROOT",
                "PrusaLink storage must be a simple name (e.g. local)",
            ));
        }
        Ok(())
    }
}

use crate::{invalid, Result};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapmakerConfig {
    pub base_url: String,
    /// Token from Luban `machine.json` (or approved on-controller pairing).
    pub token: String,
    pub printer_name: String,
    pub timeout: Duration,
    pub print_after_upload: bool,
}

impl SnapmakerConfig {
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            token: token.into(),
            printer_name: "snapmaker".into(),
            timeout: Duration::from_secs(30),
            print_after_upload: true,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.base_url.is_empty() {
            return Err(invalid(
                "PRINTER_HOST",
                "Snapmaker base_url must be non-empty",
            ));
        }
        if !(self.base_url.starts_with("http://") || self.base_url.starts_with("https://")) {
            return Err(invalid(
                "PRINTER_HOST",
                "Snapmaker base_url must start with http:// or https://",
            ));
        }
        if self.token.is_empty() || self.token.len() > 256 {
            return Err(invalid(
                "PRINTER_API_KEY",
                "Snapmaker token must be 1..256 characters",
            ));
        }
        Ok(())
    }
}

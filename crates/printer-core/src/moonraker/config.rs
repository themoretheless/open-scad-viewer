use crate::{invalid, Result};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoonrakerConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub root: String,
    pub printer_name: String,
    pub timeout: Duration,
    pub send_checksum: bool,
}

impl MoonrakerConfig {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            api_key: None,
            root: "gcodes".into(),
            printer_name: "moonraker".into(),
            timeout: Duration::from_secs(30),
            send_checksum: true,
        }
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        let key = key.into();
        self.api_key = if key.is_empty() { None } else { Some(key) };
        self
    }

    pub fn validate(&self) -> Result<()> {
        if self.base_url.is_empty() {
            return Err(invalid(
                "PRINTER_HOST",
                "Moonraker base_url must be non-empty",
            ));
        }
        if !(self.base_url.starts_with("http://") || self.base_url.starts_with("https://")) {
            return Err(invalid(
                "PRINTER_HOST",
                "Moonraker base_url must start with http:// or https://",
            ));
        }
        if self.root.is_empty() || self.root.contains("..") || self.root.contains('/') {
            return Err(invalid(
                "PRINTER_ROOT",
                "Moonraker root must be a simple name (e.g. gcodes)",
            ));
        }
        Ok(())
    }
}

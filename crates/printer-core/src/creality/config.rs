use crate::moonraker::MoonrakerConfig;
use crate::{Result, invalid};
use std::time::Duration;

/// Creality LAN config. Wire protocol is Moonraker HTTP.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrealityConfig {
    pub inner: MoonrakerConfig,
}

impl CrealityConfig {
    pub fn new(base_url: impl Into<String>) -> Self {
        let mut inner = MoonrakerConfig::new(base_url);
        inner.printer_name = "creality".into();
        Self { inner }
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.inner = self.inner.with_api_key(key);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.inner.timeout = timeout;
        self
    }

    pub fn validate(&self) -> Result<()> {
        self.inner.validate().map_err(|e| {
            if e.code == "PRINTER_HOST" {
                invalid(
                    "PRINTER_HOST",
                    "Creality base_url must be a Moonraker HTTP URL (often :7125)",
                )
            } else {
                e
            }
        })
    }
}

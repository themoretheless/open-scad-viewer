use crate::{invalid, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BambuLanConfig {
    pub host: String,
    pub access_code: String,
    pub serial: String,
    pub mqtt_port: u16,
    pub ftps_port: u16,
}

impl BambuLanConfig {
    pub fn new(host: impl Into<String>, access_code: impl Into<String>, serial: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            access_code: access_code.into(),
            serial: serial.into(),
            mqtt_port: 8883,
            ftps_port: 990,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.host.trim().is_empty() {
            return Err(invalid("PRINTER_HOST", "Bambu LAN host must be non-empty"));
        }
        if self.host.contains('\0') || self.host.contains('/') {
            return Err(invalid("PRINTER_HOST", "Bambu LAN host is not a hostname/IP"));
        }
        if self.access_code.is_empty() || self.access_code.len() > 32 {
            return Err(invalid(
                "PRINTER_ACCESS_CODE",
                "Bambu LAN access code must be 1..32 characters",
            ));
        }
        if !self.access_code.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(invalid(
                "PRINTER_ACCESS_CODE",
                "Bambu LAN access code must be ASCII alphanumeric",
            ));
        }
        if self.serial.trim().is_empty() || self.serial.len() > 64 {
            return Err(invalid(
                "PRINTER_SERIAL",
                "Bambu serial must be 1..64 characters",
            ));
        }
        if !self
            .serial
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(invalid(
                "PRINTER_SERIAL",
                "Bambu serial must be ASCII alphanumeric/-/_",
            ));
        }
        if self.mqtt_port == 0 || self.ftps_port == 0 {
            return Err(invalid("PRINTER_PORT", "Bambu ports must be non-zero"));
        }
        Ok(())
    }

    pub fn mqtt_username(&self) -> &'static str {
        "bblp"
    }

    pub fn mqtt_password(&self) -> &str {
        &self.access_code
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BambuPrintOptions {
    pub use_ams: bool,
    pub bed_levelling: bool,
    pub flow_cali: bool,
    pub vibration_cali: bool,
    pub layer_inspect: bool,
    pub timelapse: bool,
    pub bed_type: String,
}

impl Default for BambuPrintOptions {
    fn default() -> Self {
        Self {
            use_ams: false,
            bed_levelling: true,
            flow_cali: false,
            vibration_cali: false,
            layer_inspect: false,
            timelapse: false,
            bed_type: "auto".into(),
        }
    }
}

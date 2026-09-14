use crate::{invalid, Result, MAX_ARTIFACT_BYTES, MAX_REMOTE_NAME_BYTES};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrinterId {
    pub vendor: String,
    pub serial: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactKind {
    /// Plain UTF-8 / binary G-code file.
    Gcode,
    /// OPC package that already contains plate G-code (e.g. `.gcode.3mf`).
    Gcode3mf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrintArtifact {
    pub kind: ArtifactKind,
    pub bytes: Vec<u8>,
    /// Basename only; backends may reject paths with separators.
    pub file_name: String,
}

impl PrintArtifact {
    pub fn validate(&self) -> Result<()> {
        if self.bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(invalid(
                "PRINTER_ARTIFACT_LIMIT",
                "Print artifact exceeds 64 MiB",
            ));
        }
        admit_remote_name(&self.file_name)?;
        match self.kind {
            ArtifactKind::Gcode if !self.file_name.ends_with(".gcode") => {
                Err(invalid(
                    "PRINTER_ARTIFACT_NAME",
                    "G-code artifact file_name must end with .gcode",
                ))
            }
            ArtifactKind::Gcode3mf
                if !(self.file_name.ends_with(".gcode.3mf") || self.file_name.ends_with(".3mf")) =>
            {
                Err(invalid(
                    "PRINTER_ARTIFACT_NAME",
                    "3MF print artifact file_name must end with .gcode.3mf or .3mf",
                ))
            }
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrintJob {
    pub printer: PrinterId,
    pub artifact: PrintArtifact,
    pub plate_gcode_path: String,
}

impl PrintJob {
    pub fn validate(&self) -> Result<()> {
        self.artifact.validate()?;
        if self.printer.serial.trim().is_empty() {
            return Err(invalid(
                "PRINTER_SERIAL",
                "Printer serial must be non-empty",
            ));
        }
        if self.plate_gcode_path.is_empty()
            || self.plate_gcode_path.starts_with('/')
            || self.plate_gcode_path.contains('\\')
            || self
                .plate_gcode_path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(invalid(
                "PRINTER_PLATE_PATH",
                "plate_gcode_path must be a relative OPC path without ..",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobStatus {
    pub gcode_state: String,
    pub percent: Option<u8>,
    pub layer: Option<u32>,
    pub raw: String,
}

pub fn admit_remote_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > MAX_REMOTE_NAME_BYTES {
        return Err(invalid(
            "PRINTER_ARTIFACT_NAME",
            "Remote file name is empty or longer than 255 bytes",
        ));
    }
    if name.contains('/') || name.contains('\\') || name.contains('\0') || name == "." || name == ".."
    {
        return Err(invalid(
            "PRINTER_ARTIFACT_NAME",
            "Remote file name must be a basename without separators",
        ));
    }
    Ok(())
}

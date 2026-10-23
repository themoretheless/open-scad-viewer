use printer_core::{ArtifactKind, PrintArtifact, Result};
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrintArtifactBytes {
    pub kind: ArtifactKind,
    pub bytes: Vec<u8>,
    pub file_name: String,
}

pub fn infer_artifact_kind(path: &Path) -> Result<ArtifactKind> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if name.ends_with(".gcode.3mf") || name.ends_with(".3mf") {
        Ok(ArtifactKind::Gcode3mf)
    } else if name.ends_with(".gcode") || name.ends_with(".gco") {
        Ok(ArtifactKind::Gcode)
    } else {
        Err(printer_core::Error::new(
            "PRINTER_ARTIFACT_NAME",
            "Artifact path must end with .gcode, .gco, .gcode.3mf, or .3mf",
        ))
    }
}

pub fn load_artifact(path: &Path) -> Result<PrintArtifactBytes> {
    let kind = infer_artifact_kind(path)?;
    let bytes = fs::read(path).map_err(|e| {
        printer_core::Error::new(
            "PRINTER_ARTIFACT_IO",
            format!("Failed to read artifact: {e}"),
        )
    })?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| {
            printer_core::Error::new("PRINTER_ARTIFACT_NAME", "Artifact path has no file name")
        })?
        .to_owned();
    // Normalize .gco → .gcode for backend name checks.
    let file_name = if file_name.to_ascii_lowercase().ends_with(".gco") {
        format!("{}.gcode", &file_name[..file_name.len() - 4])
    } else {
        file_name
    };
    let artifact = PrintArtifact {
        kind,
        bytes: bytes.clone(),
        file_name: file_name.clone(),
    };
    artifact.validate()?;
    Ok(PrintArtifactBytes {
        kind,
        bytes,
        file_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn infers_kinds_from_extension() {
        assert_eq!(
            infer_artifact_kind(Path::new("a.gcode.3mf")).unwrap(),
            ArtifactKind::Gcode3mf
        );
        assert_eq!(
            infer_artifact_kind(Path::new("a.gcode")).unwrap(),
            ArtifactKind::Gcode
        );
    }

    #[test]
    fn loads_gcode_file() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("printer-cli-{stamp}.gcode"));
        {
            let mut f = fs::File::create(&path).unwrap();
            writeln!(f, "; test").unwrap();
        }
        let art = load_artifact(&path).unwrap();
        assert_eq!(art.kind, ArtifactKind::Gcode);
        assert_eq!(art.file_name, path.file_name().unwrap().to_str().unwrap());
        let _ = fs::remove_file(&path);
    }
}

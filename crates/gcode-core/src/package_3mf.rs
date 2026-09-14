//! OPC ZIP that carries preview G-code as `Metadata/plate_1.gcode`.
//! Stored compression only. Not a printer job and not Bambu LAN I/O.

use crate::{invalid, emit, parse, MachineProfile, PlannedLayer, Result, MAX_OUTPUT_BYTES};
use std::io::{self, Write};

pub const GCODE_PATH: &str = "Metadata/plate_1.gcode";
const MAX_PACKAGE_BYTES: usize = MAX_OUTPUT_BYTES + 64 * 1024;
const TYPES: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"model\" ContentType=\"application/vnd.ms-package.3dmanufacturing-3dmodel+xml\"/><Default Extension=\"gcode\" ContentType=\"text/x.gcode\"/></Types>";
const RELS: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Target=\"/3D/3dmodel.model\" Id=\"rel0\" Type=\"http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel\"/><Relationship Target=\"/Metadata/plate_1.gcode\" Id=\"rel1\" Type=\"http://schemas.bambulab.com/package/2021/gcode\"/></Relationships>";
const MODEL: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><model unit=\"millimeter\" xml:lang=\"en-US\" xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\"><resources></resources><build></build></model>";
const PATHS: [&str; 4] = [
    "[Content_Types].xml",
    "_rels/.rels",
    "3D/3dmodel.model",
    GCODE_PATH,
];

struct Bounded {
    bytes: Vec<u8>,
    limit: usize,
}

impl Write for Bounded {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if data.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other("3MF package exceeds the byte budget"));
        }
        self.bytes
            .try_reserve(data.len())
            .map_err(io::Error::other)?;
        self.bytes.extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn u16_at(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn u32_at(bytes: &mut [u8], offset: usize, value: usize) {
    bytes[offset..offset + 4].copy_from_slice(&(value as u32).to_le_bytes());
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb88320 } else { 0 };
        }
    }
    !crc
}

fn zip_error(message: &str) -> crate::Error {
    invalid("GCODE_3MF", message)
}

/// Wrap UTF-8 G-code in a stored OPC 3MF. Does not send the file to a printer.
pub fn package_gcode_3mf(gcode: &str) -> Result<Vec<u8>> {
    if gcode.len() > MAX_OUTPUT_BYTES {
        return Err(invalid(
            "GCODE_OUTPUT_LIMIT",
            "G-code preview exceeds 4 MiB",
        ));
    }
    if gcode.as_bytes().contains(&0) {
        return Err(zip_error("G-code for 3MF must be UTF-8 text without NUL"));
    }
    package_members(&[
        (PATHS[0], TYPES),
        (PATHS[1], RELS),
        (PATHS[2], MODEL),
        (PATHS[3], gcode.as_bytes()),
    ])
}

fn package_members(files: &[(&str, &[u8])]) -> Result<Vec<u8>> {
    let overhead: usize = 22
        + files
            .iter()
            .map(|(name, _)| 76 + 2 * name.len())
            .sum::<usize>();
    let mut remaining = MAX_PACKAGE_BYTES.saturating_sub(overhead);
    let mut out = Bounded {
        bytes: Vec::new(),
        limit: MAX_PACKAGE_BYTES,
    };
    let mut directory = Vec::new();
    for (name, data) in files {
        admit_path(name)?;
        if data.len() > remaining {
            return Err(zip_error("3MF package exceeds the byte budget"));
        }
        remaining -= data.len();
        let name_bytes = name.as_bytes();
        let offset = out.bytes.len();
        let crc = crc32(data);
        let mut local = vec![0; 30 + name_bytes.len()];
        u32_at(&mut local, 0, 0x04034b50);
        u16_at(&mut local, 4, 20);
        u16_at(&mut local, 6, 0x800);
        u16_at(&mut local, 12, 33);
        u32_at(&mut local, 14, crc as usize);
        u32_at(&mut local, 18, data.len());
        u32_at(&mut local, 22, data.len());
        u16_at(&mut local, 26, name_bytes.len() as u16);
        local[30..].copy_from_slice(name_bytes);
        out.write_all(&local)
            .and_then(|_| out.write_all(data))
            .map_err(|e| zip_error(&e.to_string()))?;
        let mut central = vec![0; 46 + name_bytes.len()];
        u32_at(&mut central, 0, 0x02014b50);
        u16_at(&mut central, 4, 20);
        u16_at(&mut central, 6, 20);
        u16_at(&mut central, 8, 0x800);
        u16_at(&mut central, 14, 33);
        u32_at(&mut central, 16, crc as usize);
        u32_at(&mut central, 20, data.len());
        u32_at(&mut central, 24, data.len());
        u16_at(&mut central, 28, name_bytes.len() as u16);
        u32_at(&mut central, 42, offset);
        central[46..].copy_from_slice(name_bytes);
        directory.extend_from_slice(&central);
    }
    let mut end = [0; 22];
    u32_at(&mut end, 0, 0x06054b50);
    u16_at(&mut end, 8, files.len() as u16);
    u16_at(&mut end, 10, files.len() as u16);
    u32_at(&mut end, 12, directory.len());
    u32_at(&mut end, 16, out.bytes.len());
    out.write_all(&directory)
        .and_then(|_| out.write_all(&end))
        .map_err(|e| zip_error(&e.to_string()))?;
    Ok(out.bytes)
}

pub fn emit_3mf(layers: &[PlannedLayer], machine: &MachineProfile) -> Result<Vec<u8>> {
    package_gcode_3mf(&emit(layers, machine)?)
}

pub fn parse_3mf(bytes: &[u8]) -> Result<crate::GcodePreview> {
    parse(&extract_gcode_3mf(bytes)?)
}

/// Read `Metadata/plate_1.gcode` from a stored OPC 3MF. Other members are ignored.
pub fn extract_gcode_3mf(bytes: &[u8]) -> Result<String> {
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err(zip_error("3MF package exceeds the byte budget"));
    }
    let mut offset = 0usize;
    let mut found: Option<String> = None;
    while offset + 4 <= bytes.len() {
        let sig = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        if sig == 0x02014b50 || sig == 0x06054b50 {
            break;
        }
        if sig != 0x04034b50 {
            return Err(zip_error("3MF is not a stored OPC ZIP"));
        }
        if offset + 30 > bytes.len() {
            return Err(zip_error("3MF local header is truncated"));
        }
        let method = u16::from_le_bytes(bytes[offset + 8..offset + 10].try_into().unwrap());
        let crc = u32::from_le_bytes(bytes[offset + 14..offset + 18].try_into().unwrap());
        let compressed = u32::from_le_bytes(bytes[offset + 18..offset + 22].try_into().unwrap())
            as usize;
        let uncompressed = u32::from_le_bytes(bytes[offset + 22..offset + 26].try_into().unwrap())
            as usize;
        let name_len = u16::from_le_bytes(bytes[offset + 26..offset + 28].try_into().unwrap())
            as usize;
        let extra_len = u16::from_le_bytes(bytes[offset + 28..offset + 30].try_into().unwrap())
            as usize;
        let name_at = offset + 30;
        let data_at = name_at
            .checked_add(name_len)
            .and_then(|n| n.checked_add(extra_len))
            .ok_or_else(|| zip_error("3MF local header is truncated"))?;
        let data_end = data_at
            .checked_add(compressed)
            .ok_or_else(|| zip_error("3MF local header is truncated"))?;
        if data_end > bytes.len() {
            return Err(zip_error("3MF member payload is truncated"));
        }
        let name = std::str::from_utf8(&bytes[name_at..name_at + name_len])
            .map_err(|_| zip_error("3MF member path is not UTF-8"))?;
        admit_path(name)?;
        if name == GCODE_PATH {
            if found.is_some() {
                return Err(zip_error("3MF contains duplicate G-code members"));
            }
            if method != 0 || compressed != uncompressed {
                return Err(invalid(
                    "GCODE_3MF_COMPRESSION",
                    "G-code 3MF members must be stored, not deflated",
                ));
            }
            if uncompressed > MAX_OUTPUT_BYTES {
                return Err(invalid(
                    "GCODE_OUTPUT_LIMIT",
                    "G-code preview exceeds 4 MiB",
                ));
            }
            let payload = &bytes[data_at..data_end];
            if crc32(payload) != crc {
                return Err(zip_error("3MF G-code CRC does not match"));
            }
            found = Some(
                std::str::from_utf8(payload)
                    .map_err(|_| zip_error("3MF G-code is not UTF-8"))?
                    .to_owned(),
            );
        }
        offset = data_end;
    }
    found.ok_or_else(|| zip_error("3MF is missing Metadata/plate_1.gcode"))
}

fn admit_path(name: &str) -> Result<()> {
    if name.is_empty()
        || name.starts_with('/')
        || name.starts_with('\\')
        || name.contains('\\')
        || name.split('/').any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(zip_error("3MF member path is not an allowed OPC name"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MachineProfile, PlannedLayer, PlannedPath};

    fn square() -> PlannedLayer {
        PlannedLayer {
            z_mm: 0.2,
            paths: vec![PlannedPath {
                points: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
                closed: true,
            }],
        }
    }

    #[test]
    fn packages_and_extracts_preview_gcode() {
        let gcode = emit(&[square()], &MachineProfile::default()).unwrap();
        let packaged = package_gcode_3mf(&gcode).unwrap();
        assert!(packaged.starts_with(b"PK"));
        assert_eq!(extract_gcode_3mf(&packaged).unwrap(), gcode);
        let preview = parse_3mf(&packaged).unwrap();
        assert_eq!(preview.layers, 1);
        assert!(preview.extrusion_mm > 0.0);
    }

    #[test]
    fn rejects_missing_and_unsafe_members() {
        assert_eq!(
            extract_gcode_3mf(b"not a zip").unwrap_err().code,
            "GCODE_3MF"
        );
        let model_only = package_members(&[
            (PATHS[0], TYPES),
            (PATHS[1], RELS),
            (PATHS[2], MODEL),
        ])
        .unwrap();
        assert_eq!(
            extract_gcode_3mf(&model_only).unwrap_err().message,
            "3MF is missing Metadata/plate_1.gcode"
        );
        assert!(admit_path("../plate.gcode").is_err());
        assert!(admit_path("/Metadata/plate_1.gcode").is_err());
        assert!(admit_path("Metadata/plate_1.gcode").is_ok());
    }
}

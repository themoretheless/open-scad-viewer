//! Deterministic OPC ZIP with native CRC32 and raw DEFLATE. Fixed trusted paths.
use crate::mesh_export::MAX_BYTES;
use crate::{Mesh, Result, check, error};
use crc32fast::hash as crc32;
use std::io::{self, Write};
const TYPES: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"model\" ContentType=\"application/vnd.ms-package.3dmanufacturing-3dmodel+xml\"/></Types>";
const RELS: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Target=\"/3D/3dmodel.model\" Id=\"rel0\" Type=\"http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel\"/></Relationships>";
const PATHS: [&[u8]; 3] = [b"[Content_Types].xml", b"_rels/.rels", b"3D/3dmodel.model"];
struct Bounded {
    bytes: Vec<u8>,
    limit: usize,
}
impl Write for Bounded {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if data.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other("Export exceeds 4 MiB."));
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
pub fn export(mesh: &Mesh, parts: &[Mesh], compressed: bool) -> Result<Vec<u8>> {
    let model = crate::model_3mf::export(mesh, parts)?;
    package(&model, compressed)
}
fn package(model: &[u8], compressed: bool) -> Result<Vec<u8>> {
    let files = [TYPES, RELS, model];
    let overhead: usize = 22 + PATHS.iter().map(|name| 76 + 2 * name.len()).sum::<usize>();
    let mut remaining = MAX_BYTES - overhead;
    let mut out = Bounded {
        bytes: Vec::new(),
        limit: MAX_BYTES,
    };
    let mut directory = Vec::new();
    for (name, data) in PATHS.into_iter().zip(files) {
        let payload = if compressed {
            let sink = Bounded {
                bytes: Vec::new(),
                limit: remaining,
            };
            let mut encoder =
                flate2::write::DeflateEncoder::new(sink, flate2::Compression::default());
            encoder.write_all(data).map_err(|e| error(e.to_string()))?;
            encoder.finish().map_err(|e| error(e.to_string()))?.bytes
        } else {
            check(data.len() <= remaining, "Export exceeds 4 MiB.")?;
            data.to_vec()
        };
        remaining -= payload.len();
        let offset = out.bytes.len();
        let crc = crc32(data);
        let method = if compressed { 8 } else { 0 };
        let mut local = vec![0; 30 + name.len()];
        u32_at(&mut local, 0, 0x04034b50);
        u16_at(&mut local, 4, 20);
        u16_at(&mut local, 6, 0x800);
        u16_at(&mut local, 8, method);
        u16_at(&mut local, 12, 33);
        u32_at(&mut local, 14, crc as usize);
        u32_at(&mut local, 18, payload.len());
        u32_at(&mut local, 22, data.len());
        u16_at(&mut local, 26, name.len() as u16);
        local[30..].copy_from_slice(name);
        out.write_all(&local)
            .and_then(|_| out.write_all(&payload))
            .map_err(|e| error(e.to_string()))?;
        let mut central = vec![0; 46 + name.len()];
        u32_at(&mut central, 0, 0x02014b50);
        u16_at(&mut central, 4, 20);
        u16_at(&mut central, 6, 20);
        u16_at(&mut central, 8, 0x800);
        u16_at(&mut central, 10, method);
        u16_at(&mut central, 14, 33);
        u32_at(&mut central, 16, crc as usize);
        u32_at(&mut central, 20, payload.len());
        u32_at(&mut central, 24, data.len());
        u16_at(&mut central, 28, name.len() as u16);
        u32_at(&mut central, 42, offset);
        central[46..].copy_from_slice(name);
        directory.extend_from_slice(&central);
    }
    let mut end = [0; 22];
    u32_at(&mut end, 0, 0x06054b50);
    u16_at(&mut end, 8, 3);
    u16_at(&mut end, 10, 3);
    u32_at(&mut end, 12, directory.len());
    u32_at(&mut end, 16, out.bytes.len());
    out.write_all(&directory)
        .and_then(|_| out.write_all(&end))
        .map_err(|e| error(e.to_string()))?;
    Ok(out.bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crc_standard_check_and_exact_artifact_budget() {
        assert_eq!(crc32(b"123456789"), 0xcbf43926);
        let overhead =
            22 + PATHS.iter().map(|n| 76 + 2 * n.len()).sum::<usize>() + TYPES.len() + RELS.len();
        let mut model = vec![b'x'; MAX_BYTES - overhead];
        assert_eq!(package(&model, false).unwrap().len(), MAX_BYTES);
        model.push(b'x');
        assert!(package(&model, false).is_err());
        assert!(package(&model, true).unwrap().len() < MAX_BYTES);
    }
}

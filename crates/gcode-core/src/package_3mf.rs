//! OPC ZIP that carries G-code as `Metadata/plate_1.gcode`.
//! Stored compression only. Not Bambu LAN I/O.

use crate::{
    JobProfile, MAX_OUTPUT_BYTES, MachineProfile, PlannedLayer, Result, emit, emit_job, invalid,
    parse, parse_job,
};
use crc32fast::hash as crc32;
use std::fmt::Write as _;
use std::io::{self, Write};

pub const GCODE_PATH: &str = "Metadata/plate_1.gcode";
pub const PLATE_JSON_PATH: &str = "Metadata/plate_1.json";
const MODEL_PATH: &str = "3D/3dmodel.model";
const MAX_PACKAGE_BYTES: usize = MAX_OUTPUT_BYTES + 512 * 1024;
const MAX_MESH_TRIANGLES: usize = 100_000;
const EMPTY_MODEL: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><model unit=\"millimeter\" xml:lang=\"en-US\" xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\"><resources></resources><build></build></model>";
const TYPES_PREVIEW: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"model\" ContentType=\"application/vnd.ms-package.3dmanufacturing-3dmodel+xml\"/><Default Extension=\"gcode\" ContentType=\"text/x.gcode\"/></Types>";
const TYPES_JOB: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"model\" ContentType=\"application/vnd.ms-package.3dmanufacturing-3dmodel+xml\"/><Default Extension=\"gcode\" ContentType=\"text/x.gcode\"/><Default Extension=\"json\" ContentType=\"application/json\"/></Types>";
const RELS_PREVIEW: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Target=\"/3D/3dmodel.model\" Id=\"rel0\" Type=\"http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel\"/><Relationship Target=\"/Metadata/plate_1.gcode\" Id=\"rel1\" Type=\"http://schemas.bambulab.com/package/2021/gcode\"/></Relationships>";
const RELS_JOB: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Target=\"/3D/3dmodel.model\" Id=\"rel0\" Type=\"http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel\"/><Relationship Target=\"/Metadata/plate_1.gcode\" Id=\"rel1\" Type=\"http://schemas.bambulab.com/package/2021/gcode\"/><Relationship Target=\"/Metadata/plate_1.json\" Id=\"rel2\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata\"/></Relationships>";

/// Optional triangle body embedded as `3D/3dmodel.model` inside a job package.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshBody {
    pub positions: Vec<f64>,
    pub indices: Vec<usize>,
}

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

/// RFC 1321 MD5 for plate metadata. Not a security boundary.
fn md5_hex(data: &[u8]) -> String {
    fn f(x: u32, y: u32, z: u32) -> u32 {
        (x & y) | (!x & z)
    }
    fn g(x: u32, y: u32, z: u32) -> u32 {
        (x & z) | (y & !z)
    }
    fn h(x: u32, y: u32, z: u32) -> u32 {
        x ^ y ^ z
    }
    fn ii(x: u32, y: u32, z: u32) -> u32 {
        y ^ (x | !z)
    }
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_le_bytes());
    let mut state = [0x67452301u32, 0xefcdab89, 0x98badcfe, 0x10325476];
    for chunk in msg.chunks_exact(64) {
        let mut x = [0u32; 16];
        for (i, word) in x.iter_mut().enumerate() {
            *word = u32::from_le_bytes(chunk[i * 4..i * 4 + 4].try_into().unwrap());
        }
        let (mut a, mut b, mut c, mut d) = (state[0], state[1], state[2], state[3]);
        const S: [u32; 64] = [
            7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20,
            5, 9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
            6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
        ];
        const K: [u32; 64] = [
            0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613,
            0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193,
            0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d,
            0x02441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
            0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122,
            0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
            0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244,
            0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
            0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb,
            0xeb86d391,
        ];
        for i in 0..64 {
            let (fun, g_idx) = match i {
                0..=15 => (f as fn(u32, u32, u32) -> u32, i),
                16..=31 => (g as fn(u32, u32, u32) -> u32, (5 * i + 1) % 16),
                32..=47 => (h as fn(u32, u32, u32) -> u32, (3 * i + 5) % 16),
                _ => (ii as fn(u32, u32, u32) -> u32, (7 * i) % 16),
            };
            let temp = a
                .wrapping_add(fun(b, c, d))
                .wrapping_add(x[g_idx])
                .wrapping_add(K[i])
                .rotate_left(S[i])
                .wrapping_add(b);
            a = d;
            d = c;
            c = b;
            b = temp;
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
    }
    let mut out = String::with_capacity(32);
    for word in state {
        for byte in word.to_le_bytes() {
            write!(out, "{byte:02x}").unwrap();
        }
    }
    out
}

fn zip_error(message: &str) -> crate::Error {
    invalid("GCODE_3MF", message)
}

fn format_coord(value: f64) -> Result<String> {
    if !value.is_finite() || value.abs() > 1_000_000.0 {
        return Err(zip_error(
            "Mesh coordinates must be finite within +/-1000000 mm",
        ));
    }
    Ok(format!("{value}"))
}

fn model_xml_from_mesh(mesh: &MeshBody) -> Result<Vec<u8>> {
    if mesh.positions.len() % 3 != 0 {
        return Err(zip_error("Mesh positions must be XYZ triples"));
    }
    if mesh.indices.len() % 3 != 0 {
        return Err(zip_error("Mesh indices must be triangle triples"));
    }
    let vertices = mesh.positions.len() / 3;
    let triangles = mesh.indices.len() / 3;
    if triangles > MAX_MESH_TRIANGLES || vertices > 300_000 {
        return Err(zip_error(
            "Mesh exceeds 100000 triangles or 300000 vertices",
        ));
    }
    if triangles == 0 {
        return Err(zip_error("Mesh body must contain at least one triangle"));
    }
    for &index in &mesh.indices {
        if index >= vertices {
            return Err(zip_error("Mesh index is out of range"));
        }
    }
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?><model unit=\"millimeter\" xml:lang=\"en-US\" xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\"><resources><object id=\"1\" name=\"Part 1\" type=\"model\"><mesh><vertices>");
    for p in mesh.positions.chunks_exact(3) {
        write!(
            out,
            "<vertex x=\"{}\" y=\"{}\" z=\"{}\"/>",
            format_coord(p[0])?,
            format_coord(p[1])?,
            format_coord(p[2])?
        )
        .map_err(|_| zip_error("Mesh model allocation failed"))?;
        if out.len() > MAX_OUTPUT_BYTES {
            return Err(zip_error("Mesh model exceeds the byte budget"));
        }
    }
    out.push_str("</vertices><triangles>");
    for t in mesh.indices.chunks_exact(3) {
        write!(
            out,
            "<triangle v1=\"{}\" v2=\"{}\" v3=\"{}\"/>",
            t[0], t[1], t[2]
        )
        .map_err(|_| zip_error("Mesh model allocation failed"))?;
        if out.len() > MAX_OUTPUT_BYTES {
            return Err(zip_error("Mesh model exceeds the byte budget"));
        }
    }
    out.push_str(
        "</triangles></mesh></object></resources><build><item objectid=\"1\"/></build></model>",
    );
    Ok(out.into_bytes())
}

fn plate_json(job: &JobProfile, gcode: &str) -> String {
    format!(
        "{{\"plate\":1,\"nozzle_temp_c\":{},\"bed_temp_c\":{},\"filament_diameter_mm\":{},\"filament\":\"placeholder\",\"gcode_md5\":\"{}\"}}",
        job.nozzle_temp_c,
        job.bed_temp_c,
        job.machine.filament_diameter_mm,
        md5_hex(gcode.as_bytes())
    )
}

/// Wrap UTF-8 preview G-code in a stored OPC 3MF with an empty model body.
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
        ("[Content_Types].xml", TYPES_PREVIEW),
        ("_rels/.rels", RELS_PREVIEW),
        (MODEL_PATH, EMPTY_MODEL),
        (GCODE_PATH, gcode.as_bytes()),
    ])
}

/// Package job G-code with plate metadata and an optional mesh model body.
pub fn package_job_3mf(gcode: &str, job: &JobProfile, mesh: Option<&MeshBody>) -> Result<Vec<u8>> {
    if gcode.len() > MAX_OUTPUT_BYTES {
        return Err(invalid("GCODE_OUTPUT_LIMIT", "G-code job exceeds 4 MiB"));
    }
    if gcode.as_bytes().contains(&0) {
        return Err(zip_error("G-code for 3MF must be UTF-8 text without NUL"));
    }
    let model;
    let model_bytes = if let Some(mesh) = mesh {
        model = model_xml_from_mesh(mesh)?;
        model.as_slice()
    } else {
        EMPTY_MODEL
    };
    let json = plate_json(job, gcode);
    package_members(&[
        ("[Content_Types].xml", TYPES_JOB),
        ("_rels/.rels", RELS_JOB),
        (MODEL_PATH, model_bytes),
        (GCODE_PATH, gcode.as_bytes()),
        (PLATE_JSON_PATH, json.as_bytes()),
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

pub fn emit_gcode_3mf_job(
    layers: &[PlannedLayer],
    job: &JobProfile,
    mesh: Option<&MeshBody>,
) -> Result<Vec<u8>> {
    package_job_3mf(&emit_job(layers, job)?, job, mesh)
}

pub fn parse_3mf(bytes: &[u8]) -> Result<crate::GcodePreview> {
    let gcode = extract_gcode_3mf(bytes)?;
    if gcode.lines().next() == Some(format!("; {}", crate::JOB_DIALECT).as_str()) {
        parse_job(&gcode)
    } else {
        parse(&gcode)
    }
}

/// Read `Metadata/plate_1.gcode` from a stored OPC 3MF. Other members are ignored.
pub fn extract_gcode_3mf(bytes: &[u8]) -> Result<String> {
    extract_member_3mf(bytes, GCODE_PATH)
}

/// Read a stored UTF-8 member by exact path.
pub fn extract_member_3mf(bytes: &[u8], path: &str) -> Result<String> {
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err(zip_error("3MF package exceeds the byte budget"));
    }
    admit_path(path)?;
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
        let compressed =
            u32::from_le_bytes(bytes[offset + 18..offset + 22].try_into().unwrap()) as usize;
        let uncompressed =
            u32::from_le_bytes(bytes[offset + 22..offset + 26].try_into().unwrap()) as usize;
        let name_len =
            u16::from_le_bytes(bytes[offset + 26..offset + 28].try_into().unwrap()) as usize;
        let extra_len =
            u16::from_le_bytes(bytes[offset + 28..offset + 30].try_into().unwrap()) as usize;
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
        if name == path {
            if found.is_some() {
                return Err(zip_error(
                    "3MF contains duplicate members for the requested path",
                ));
            }
            if method != 0 || compressed != uncompressed {
                return Err(invalid(
                    "GCODE_3MF_COMPRESSION",
                    "G-code 3MF members must be stored, not deflated",
                ));
            }
            if uncompressed > MAX_OUTPUT_BYTES {
                return Err(invalid("GCODE_OUTPUT_LIMIT", "3MF member exceeds 4 MiB"));
            }
            let payload = &bytes[data_at..data_end];
            if crc32(payload) != crc {
                return Err(zip_error("3MF member CRC does not match"));
            }
            found = Some(
                std::str::from_utf8(payload)
                    .map_err(|_| zip_error("3MF member is not UTF-8"))?
                    .to_owned(),
            );
        }
        offset = data_end;
    }
    found.ok_or_else(|| zip_error(&format!("3MF is missing {path}")))
}

fn admit_path(name: &str) -> Result<()> {
    if name.is_empty()
        || name.starts_with('/')
        || name.starts_with('\\')
        || name.contains('\\')
        || name
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
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

    fn cube_mesh() -> MeshBody {
        MeshBody {
            positions: vec![
                0., 0., 0., 1., 0., 0., 1., 1., 0., 0., 1., 0., 0., 0., 1., 1., 0., 1., 1., 1., 1.,
                0., 1., 1.,
            ],
            indices: vec![
                0, 1, 2, 0, 2, 3, 4, 6, 5, 4, 7, 6, 0, 4, 5, 0, 5, 1, 1, 5, 6, 1, 6, 2, 2, 6, 7, 2,
                7, 3, 3, 7, 4, 3, 4, 0,
            ],
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
    fn job_package_embeds_mesh_and_plate_metadata() {
        let job = JobProfile::default();
        let packaged = emit_gcode_3mf_job(&[square()], &job, Some(&cube_mesh())).unwrap();
        let gcode = extract_gcode_3mf(&packaged).unwrap();
        assert!(gcode.contains("M109"));
        let model = extract_member_3mf(&packaged, MODEL_PATH).unwrap();
        assert!(model.contains("<triangle"));
        assert!(!model.contains("<resources></resources>"));
        let json = extract_member_3mf(&packaged, PLATE_JSON_PATH).unwrap();
        assert!(json.contains("\"plate\":1"));
        assert!(json.contains(&md5_hex(gcode.as_bytes())));
        let preview = parse_3mf(&packaged).unwrap();
        assert_eq!(preview.layers, 1);
    }

    #[test]
    fn md5_matches_rfc_vector() {
        assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn rejects_missing_and_unsafe_members() {
        assert_eq!(
            extract_gcode_3mf(b"not a zip").unwrap_err().code,
            "GCODE_3MF"
        );
        let model_only = package_members(&[
            ("[Content_Types].xml", TYPES_PREVIEW),
            ("_rels/.rels", RELS_PREVIEW),
            (MODEL_PATH, EMPTY_MODEL),
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

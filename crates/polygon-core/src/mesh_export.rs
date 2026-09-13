//! Validated millimeter mesh artifacts. Serialization and numeric admission are native.
use crate::solid::export_file::Number;
use crate::{Mesh, Result, check, cross, error, norm, sub};
use std::fmt::{self, Write};

pub const MAX_BYTES: usize = 4 * 1024 * 1024;
struct Output(Vec<u8>);
impl Write for Output {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if text.len() > MAX_BYTES.saturating_sub(self.0.len()) {
            return Err(fmt::Error);
        }
        self.0.try_reserve(text.len()).map_err(|_| fmt::Error)?;
        self.0.extend_from_slice(text.as_bytes());
        Ok(())
    }
}

pub fn export(mesh: &Mesh, format: &str) -> Result<Vec<u8>> {
    check(
        matches!(format, "stl" | "stl_binary" | "obj" | "ply" | "off" | "amf"),
        "Unsupported mesh export format.",
    )?;
    check(
        mesh.indices.len() / 3 <= 100_000,
        "Mesh export exceeds 100000 triangles.",
    )?;
    let report = mesh.inspect()?;
    check(
        report.degenerate_triangles == 0
            && report.non_manifold_edges == 0
            && report.orientation_conflicts == 0,
        "Mesh has invalid or inconsistent topology.",
    )?;
    if matches!(format, "stl" | "stl_binary" | "amf") {
        check(
            report.closed && report.signed_volume_mm3 > 0.,
            "Printing export requires a closed oriented mesh with positive volume.",
        )?;
    }
    if format == "stl" {
        let text = mesh.export_stl()?;
        check(text.len() <= MAX_BYTES, "Export exceeds 4 MiB.")?;
        return Ok(text.into_bytes());
    }
    if format == "stl_binary" {
        return binary_stl(mesh);
    }
    let mut out = Output(Vec::new());
    let written = (|| -> fmt::Result {
        match format {
            "obj" => out.write_str("# ModelGraph; units: millimeter\n")?,
            "ply" => write!(out, "ply\nformat ascii 1.0\ncomment units millimeter\nelement vertex {}\nproperty double x\nproperty double y\nproperty double z\nelement face {}\nproperty list uchar int vertex_indices\nend_header\n", mesh.positions.len()/3, mesh.indices.len()/3)?,
            "off" => writeln!(out, "OFF\n{} {} 0", mesh.positions.len()/3, mesh.indices.len()/3)?,
            "amf" => out.write_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?><amf unit=\"millimeter\" version=\"1.1\"><object id=\"0\"><mesh><vertices>")?,
            _ => unreachable!(),
        }
        for p in mesh.positions.chunks_exact(3) {
            let [x, y, z] = [Number(p[0]), Number(p[1]), Number(p[2])];
            match format {
                "obj" => writeln!(out, "v {x} {y} {z}")?,
                "amf" => write!(
                    out,
                    "<vertex><coordinates><x>{x}</x><y>{y}</y><z>{z}</z></coordinates></vertex>"
                )?,
                _ => writeln!(out, "{x} {y} {z}")?,
            }
        }
        if format == "amf" {
            out.write_str("</vertices><volume>")?;
        }
        for t in mesh.indices.chunks_exact(3) {
            let [a, b, c] = [t[0], t[1], t[2]];
            match format {
                "obj" => writeln!(out, "f {} {} {}", a + 1, b + 1, c + 1)?,
                "amf" => write!(
                    out,
                    "<triangle><v1>{a}</v1><v2>{b}</v2><v3>{c}</v3></triangle>"
                )?,
                _ => writeln!(out, "3 {a} {b} {c}")?,
            }
        }
        if format == "amf" {
            out.write_str("</volume></mesh></object></amf>")?;
        }
        Ok(())
    })();
    written.map_err(|_| error("Export exceeds 4 MiB or allocation failed."))?;
    Ok(out.0)
}

fn binary_stl(mesh: &Mesh) -> Result<Vec<u8>> {
    let size = 84 + mesh.indices.len() / 3 * 50;
    check(size <= MAX_BYTES, "Export exceeds 4 MiB.")?;
    let rounded = Mesh {
        positions: mesh.positions.iter().map(|&v| (v as f32) as f64).collect(),
        indices: mesh.indices.clone(),
        uv: None,
    };
    check(
        rounded.positions.iter().all(|v| v.is_finite()),
        "STL float32 range exceeded.",
    )?;
    check(
        rounded.inspect()?.degenerate_triangles == 0,
        "Binary STL precision would collapse triangles; use ASCII STL or 3MF.",
    )?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|_| error("Export allocation failed."))?;
    bytes.resize(84, 0);
    let header = b"ModelGraph mesh; coordinates in millimeters";
    bytes[..header.len()].copy_from_slice(header);
    bytes[80..84].copy_from_slice(&((mesh.indices.len() / 3) as u32).to_le_bytes());
    for t in mesh.indices.chunks_exact(3) {
        let [a, b, c] = [mesh.point(t[0])?, mesh.point(t[1])?, mesh.point(t[2])?];
        let n = cross(sub(b, a), sub(c, a));
        let length = norm(n);
        for value in n.map(|v| v / length).into_iter().chain(a).chain(b).chain(c) {
            let f = value as f32;
            check(f.is_finite(), "STL float32 range exceeded.")?;
            bytes.extend_from_slice(&f.to_le_bytes());
        }
        bytes.extend_from_slice(&[0, 0]);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn square() -> Mesh {
        Mesh {
            positions: vec![0., 0., 0., 2., 0., 0., 2., 3., 0., 0., 3., 0.],
            indices: vec![0, 1, 2, 0, 2, 3],
            uv: None,
        }
    }
    #[test]
    fn open_mesh_admission_and_index_conventions() {
        let mesh = square();
        for format in ["stl", "stl_binary", "amf"] {
            assert!(
                export(&mesh, format)
                    .unwrap_err()
                    .message
                    .contains("closed")
            );
        }
        let obj = String::from_utf8(export(&mesh, "obj").unwrap()).unwrap();
        assert!(obj.ends_with("f 1 2 3\nf 1 3 4\n"));
        for format in ["off", "ply"] {
            let text = String::from_utf8(export(&mesh, format).unwrap()).unwrap();
            assert!(text.ends_with("3 0 1 2\n3 0 2 3\n"));
        }
        let mut malformed = mesh.clone();
        malformed.indices[0] = 99;
        for format in ["stl", "stl_binary", "obj", "ply", "off", "amf"] {
            assert!(export(&malformed, format).is_err());
        }
        assert!(export(&mesh, "unknown").is_err());
    }
    #[test]
    fn binary_stl_records_and_float32_collapse() {
        let mesh = square().thicken([0., 0., 4.]).unwrap().mesh;
        let data = export(&mesh, "stl_binary").unwrap();
        assert_eq!(data.len(), 84 + 12 * 50);
        assert_eq!(&data[80..84], &12u32.to_le_bytes());
        // First face points downward, with source indexing preserved.
        assert_eq!(f32::from_le_bytes(data[92..96].try_into().unwrap()), -1.);
        assert_eq!(&data[132..134], &[0, 0]);
        let mut shifted = mesh.clone();
        for p in shifted.positions.chunks_exact_mut(3) {
            p[0] += 100_000_000.;
        }
        assert!(
            export(&shifted, "stl_binary")
                .unwrap_err()
                .message
                .contains("collapse")
        );
        assert!(export(&shifted, "stl").is_ok());
    }
    #[test]
    fn bounded_text_never_appends_past_limit() {
        let mut out = Output(vec![0; MAX_BYTES - 1]);
        assert!(out.write_str("xx").is_err());
        assert_eq!(out.0.len(), MAX_BYTES - 1);
        out.write_str("x").unwrap();
        assert!(out.write_str("x").is_err());
        assert_eq!(out.0.len(), MAX_BYTES);
    }
}

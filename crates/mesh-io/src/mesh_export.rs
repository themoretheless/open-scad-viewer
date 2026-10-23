//! Validated millimeter mesh artifacts. Serialization and numeric admission are native.
use crate::export_file::Number;
use crate::{MeshView, Result, check, cross, error, norm, sub};
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

pub fn export(mesh: &MeshView<'_>, format: &str) -> Result<Vec<u8>> {
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
        let text = crate::export_stl(mesh)?;
        check(text.len() <= MAX_BYTES, "Export exceeds 4 MiB.")?;
        return Ok(text.into_bytes());
    }
    if format == "stl_binary" {
        return binary_stl(mesh);
    }
    let v_count = mesh.positions.len() / 3;
    let f_count = mesh.indices.len() / 3;
    let est_cap = match format {
        "obj" => 36 + v_count * 36 + f_count * 28,
        "ply" => 180 + v_count * 36 + f_count * 28,
        "off" => 32 + v_count * 36 + f_count * 28,
        "amf" => 160 + v_count * 96 + f_count * 64,
        _ => 256,
    }
    .min(MAX_BYTES);
    let mut out = Output(Vec::with_capacity(est_cap));
    let written = (|| -> fmt::Result {
        match format {
            "obj" => out.write_str("# RushGraph; units: millimeter\n")?,
            "ply" => write!(out, "ply\nformat ascii 1.0\ncomment units millimeter\nelement vertex {}\nproperty double x\nproperty double y\nproperty double z\nelement face {}\nproperty list uchar int vertex_indices\nend_header\n", mesh.positions.len()/3, mesh.indices.len()/3)?,
            "off" => writeln!(out, "OFF\n{} {} 0", mesh.positions.len()/3, mesh.indices.len()/3)?,
            "amf" => out.write_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?><amf unit=\"millimeter\" version=\"1.1\"><object id=\"0\"><mesh><vertices>")?,
            _ => unreachable!(),
        }
        for p in mesh.positions.as_chunks::<3>().0 {
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
        for t in mesh.indices.as_chunks::<3>().0 {
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

/// Print-mesh trio: STL, OBJ, and mesh 3MF. Other mesh codecs stay on [`export`].
pub fn export_print_mesh(mesh: &MeshView<'_>, format: &str) -> Result<Vec<u8>> {
    match format {
        "stl" | "stl_binary" | "obj" => export(mesh, format),
        "3mf" => {
            let report = mesh.inspect()?;
            check(
                report.closed
                    && report.signed_volume_mm3 > 0.
                    && report.degenerate_triangles == 0
                    && report.non_manifold_edges == 0
                    && report.orientation_conflicts == 0,
                "Printing export requires a closed oriented mesh with positive volume.",
            )?;
            crate::package_3mf::export(mesh, &[], false)
        }
        _ => Err(error(
            "Unsupported print mesh format; use stl, stl_binary, obj, or 3mf.",
        )),
    }
}

fn binary_stl(mesh: &MeshView<'_>) -> Result<Vec<u8>> {
    let size = 84 + mesh.indices.len() / 3 * 50;
    check(size <= MAX_BYTES, "Export exceeds 4 MiB.")?;
    let positions: Vec<f64> = mesh.positions.iter().map(|&v| (v as f32) as f64).collect();
    let rounded = MeshView {
        positions: &positions,
        indices: mesh.indices,
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
    let header = b"RushGraph mesh; coordinates in millimeters";
    bytes[..header.len()].copy_from_slice(header);
    bytes[80..84].copy_from_slice(&((mesh.indices.len() / 3) as u32).to_le_bytes());
    for t in mesh.indices.as_chunks::<3>().0 {
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

/// Exports a validated `MeshView` to either ASCII PLY (`binary_little_endian == false`)
/// or compact binary little-endian PLY (`binary_little_endian == true`).
pub fn export_ply(mesh: &MeshView<'_>, binary_little_endian: bool) -> Result<Vec<u8>> {
    if !binary_little_endian {
        return export(mesh, "ply");
    }
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
    let v_count = mesh.positions.len() / 3;
    let f_count = mesh.indices.len() / 3;
    let header = format!(
        "ply\nformat binary_little_endian 1.0\ncomment units millimeter\nelement vertex {v_count}\nproperty double x\nproperty double y\nproperty double z\nelement face {f_count}\nproperty list uchar int vertex_indices\nend_header\n"
    );
    let body_size = v_count * 24 + f_count * 13;
    check(
        header.len() + body_size <= MAX_BYTES,
        "Export exceeds 4 MiB.",
    )?;
    let mut out = Vec::with_capacity(header.len() + body_size);
    out.extend_from_slice(header.as_bytes());
    for &coord in mesh.positions {
        out.extend_from_slice(&coord.to_le_bytes());
    }
    for tri in mesh.indices.as_chunks::<3>().0 {
        out.push(3u8);
        for &idx in tri {
            out.extend_from_slice(&(idx as i32).to_le_bytes());
        }
    }
    Ok(out)
}

/// Exports a validated `MeshView` to a Wavefront `.obj` string along with a companion `.mtl`
/// material library string defining `material_name` with RGB diffuse color `diffuse_rgb` ($\in [0, 1]^3$).
pub fn export_obj_with_mtl(
    mesh: &MeshView<'_>,
    material_name: &str,
    diffuse_rgb: [f64; 3],
) -> Result<(String, String)> {
    let mat = material_name.trim();
    check(
        !mat.is_empty()
            && mat
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')),
        "Invalid OBJ material name.",
    )?;
    check(
        diffuse_rgb
            .iter()
            .all(|&c| c.is_finite() && (0.0..=1.0).contains(&c)),
        "Diffuse RGB components must be finite and within 0.0..=1.0.",
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

    let v_count = mesh.positions.len() / 3;
    let f_count = mesh.indices.len() / 3;
    let est_cap = (64 + mat.len() * 2 + v_count * 36 + f_count * 28).min(MAX_BYTES);
    let mut obj = Output(Vec::with_capacity(est_cap));
    let written = (|| -> fmt::Result {
        writeln!(
            obj,
            "# RushGraph; units: millimeter\nmtllib {mat}.mtl\nusemtl {mat}"
        )?;
        for p in mesh.positions.as_chunks::<3>().0 {
            let [x, y, z] = [Number(p[0]), Number(p[1]), Number(p[2])];
            writeln!(obj, "v {x} {y} {z}")?;
        }
        for t in mesh.indices.as_chunks::<3>().0 {
            writeln!(obj, "f {} {} {}", t[0] + 1, t[1] + 1, t[2] + 1)?;
        }
        Ok(())
    })();
    written.map_err(|_| error("Export exceeds 4 MiB or allocation failed."))?;
    let obj_source = String::from_utf8(obj.0).map_err(|_| error("UTF-8 encoding failed."))?;
    let mut mtl_source = String::with_capacity(140 + mat.len());
    let _ = write!(
        mtl_source,
        "# RushGraph material library\nnewmtl {mat}\nKa 0.1000 0.1000 0.1000\nKd {:.4} {:.4} {:.4}\nKs 0.2000 0.2000 0.2000\nNs 32.0\nd 1.0\nillum 2\n",
        diffuse_rgb[0], diffuse_rgb[1], diffuse_rgb[2]
    );
    Ok((obj_source, mtl_source))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_text_never_appends_past_limit() {
        let mut out = Output(vec![0; MAX_BYTES - 1]);
        assert!(out.write_str("xx").is_err());
        assert_eq!(out.0.len(), MAX_BYTES - 1);
        out.write_str("x").unwrap();
        assert!(out.write_str("x").is_err());
        assert_eq!(out.0.len(), MAX_BYTES);
    }

    #[test]
    fn export_and_import_ply_binary_and_ascii_roundtrip() {
        let positions = [
            0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 10.0,
        ];
        let indices = [0, 2, 1, 0, 1, 3, 1, 2, 3, 2, 0, 3];
        let view = MeshView::new(&positions, &indices);

        let bin_bytes = export_ply(&view, true).unwrap();
        let imported_bin = crate::import_ply(&bin_bytes).unwrap();
        assert_eq!(imported_bin.positions, positions);
        assert_eq!(imported_bin.indices, indices);

        let ascii_bytes = export_ply(&view, false).unwrap();
        let imported_ascii = crate::import_ply(&ascii_bytes).unwrap();
        assert_eq!(imported_ascii.positions, positions);
        assert_eq!(imported_ascii.indices, indices);

        let (obj_src, mtl_src) =
            export_obj_with_mtl(&view, " anodized_alu ", [0.25, 0.55, 0.85]).unwrap();
        assert!(obj_src.contains("mtllib anodized_alu.mtl"));
        assert!(obj_src.contains("usemtl anodized_alu"));
        assert!(obj_src.contains("f 1 3 2"));
        assert!(mtl_src.contains("newmtl anodized_alu"));
        assert!(mtl_src.contains("Kd 0.2500 0.5500 0.8500"));
    }
}

//! 3MF model document admission and serialization, before OPC packaging.
use crate::export_file::Number;
use crate::{MeshView, Result, check, error};
use std::fmt::{self, Write};
// The final compressed artifact has a separate 4 MiB limit. Bound the expanded
// document without rejecting valid compressible models at that smaller limit.
pub const MAX_MODEL_BYTES: usize = 64 * 1024 * 1024;
struct Document(Vec<u8>);
impl Write for Document {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if text.len() > MAX_MODEL_BYTES.saturating_sub(self.0.len()) {
            return Err(fmt::Error);
        }
        self.0.try_reserve(text.len()).map_err(|_| fmt::Error)?;
        self.0.extend_from_slice(text.as_bytes());
        Ok(())
    }
}
pub fn export(mesh: &MeshView<'_>, parts: &[MeshView<'_>]) -> Result<Vec<u8>> {
    mesh.validate()?;
    let parts = if parts.is_empty() {
        std::slice::from_ref(mesh)
    } else {
        parts
    };
    let mut triangles = 0usize;
    let mut vertices = 0usize;
    for part in parts {
        // Count the actual exported parts, not only the aggregate preview mesh.
        triangles = triangles
            .checked_add(part.indices.len() / 3)
            .ok_or_else(|| error("3MF resource budget exceeded."))?;
        vertices = vertices
            .checked_add(part.positions.len() / 3)
            .ok_or_else(|| error("3MF resource budget exceeded."))?;
        check(
            triangles <= 100_000 && vertices <= 300_000,
            "3MF resource budget exceeded.",
        )?;
        let r = part.inspect()?;
        check(
            r.closed
                && r.signed_volume_mm3 > 0.
                && r.degenerate_triangles == 0
                && r.non_manifold_edges == 0
                && r.orientation_conflicts == 0,
            "Each 3MF object must be a closed oriented mesh with positive volume.",
        )?;
    }
    let mut out = Document(Vec::new());
    let result = (|| -> fmt::Result {
        out.write_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?><model unit=\"millimeter\" xml:lang=\"en-US\" xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\"><resources>")?;
        for (index, part) in parts.iter().enumerate() {
            let id = index + 1;
            write!(
                out,
                "<object id=\"{id}\" name=\"Part {id}\" type=\"model\"><mesh><vertices>"
            )?;
            for p in part.positions.as_chunks::<3>().0 {
                write!(
                    out,
                    "<vertex x=\"{}\" y=\"{}\" z=\"{}\"/>",
                    Number(p[0]),
                    Number(p[1]),
                    Number(p[2])
                )?;
            }
            out.write_str("</vertices><triangles>")?;
            for t in part.indices.as_chunks::<3>().0 {
                write!(
                    out,
                    "<triangle v1=\"{}\" v2=\"{}\" v3=\"{}\"/>",
                    t[0], t[1], t[2]
                )?;
            }
            out.write_str("</triangles></mesh></object>")?;
        }
        out.write_str("</resources><build>")?;
        for index in 0..parts.len() {
            write!(out, "<item objectid=\"{}\"/>", index + 1)?;
        }
        out.write_str("</build></model>")?;
        Ok(())
    })();
    result.map_err(|_| error("3MF expanded model exceeds 64 MiB or allocation failed."))?;
    Ok(out.0)
}

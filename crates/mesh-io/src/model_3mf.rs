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

/// A material entry in a 3MF `<basematerials>` palette.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreeMfMaterial {
    pub name: String,
    /// Hex color string `#RRGGBB` or `#RRGGBBAA`.
    pub display_color: String,
}

/// A single strut in a 3MF Beam Lattice extension (`<beam v1=".." v2=".." r1=".." r2=".."/>`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThreeMfBeam {
    pub v1: usize,
    pub v2: usize,
    pub r1_mm: f64,
    pub r2_mm: f64,
}

/// Exports a 3MF Beam Lattice XML document with optional `<basematerials>` and `<metadata>`.
pub fn export_beam_lattice_3mf(
    nodes_mm: &[[f64; 3]],
    beams: &[ThreeMfBeam],
    min_length_mm: f64,
    default_radius_mm: f64,
    materials: &[ThreeMfMaterial],
    metadata: &[(&str, &str)],
) -> Result<Vec<u8>> {
    check(
        nodes_mm.len() >= 2 && nodes_mm.len() <= 300_000,
        "3MF beam lattice requires 2..=300000 nodes.",
    )?;
    check(
        !beams.is_empty() && beams.len() <= 300_000,
        "3MF beam lattice requires 1..=300000 beams.",
    )?;
    check(
        min_length_mm.is_finite()
            && min_length_mm > 0.0
            && default_radius_mm.is_finite()
            && default_radius_mm > 0.0,
        "3MF beam lattice min_length_mm and default_radius_mm must be positive and finite.",
    )?;
    for p in nodes_mm {
        check(
            p.iter().all(|v| v.is_finite()),
            "3MF beam lattice node coordinates must be finite.",
        )?;
    }
    for b in beams {
        check(
            b.v1 < nodes_mm.len()
                && b.v2 < nodes_mm.len()
                && b.v1 != b.v2
                && b.r1_mm.is_finite()
                && b.r1_mm > 0.0
                && b.r2_mm.is_finite()
                && b.r2_mm > 0.0,
            "3MF beam endpoints must be distinct valid node indices with positive finite radii.",
        )?;
    }
    let mut out = Document(Vec::new());
    let res = (|| -> fmt::Result {
        out.write_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?><model unit=\"millimeter\" xml:lang=\"en-US\" xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\" xmlns:b=\"http://schemas.microsoft.com/3dmanufacturing/beamlattice/2017/02\" requiredextensions=\"b\">")?;
        for &(k, v) in metadata {
            write!(out, "<metadata name=\"{k}\">{v}</metadata>")?;
        }
        out.write_str("<resources>")?;
        if !materials.is_empty() {
            out.write_str("<basematerials id=\"1\">")?;
            for m in materials {
                write!(
                    out,
                    "<base name=\"{}\" displaycolor=\"{}\"/>",
                    m.name, m.display_color
                )?;
            }
            out.write_str("</basematerials>")?;
        }
        let pid_attr = if materials.is_empty() {
            ""
        } else {
            " pid=\"1\" pindex=\"0\""
        };
        write!(
            out,
            "<object id=\"2\" name=\"Lattice\" type=\"model\"{pid_attr}><mesh><vertices>"
        )?;
        for p in nodes_mm {
            write!(
                out,
                "<vertex x=\"{}\" y=\"{}\" z=\"{}\"/>",
                Number(p[0]),
                Number(p[1]),
                Number(p[2])
            )?;
        }
        write!(
            out,
            "</vertices><b:beamlattice minlength=\"{}\" radius=\"{}\" cap=\"sphere\"><b:beams>",
            Number(min_length_mm),
            Number(default_radius_mm)
        )?;
        for b in beams {
            write!(
                out,
                "<b:beam v1=\"{}\" v2=\"{}\" r1=\"{}\" r2=\"{}\"/>",
                b.v1,
                b.v2,
                Number(b.r1_mm),
                Number(b.r2_mm)
            )?;
        }
        out.write_str(
            "</b:beams></b:beamlattice></mesh></object></resources><build><item objectid=\"2\"/></build></model>",
        )?;
        Ok(())
    })();
    res.map_err(|_| error("3MF beam lattice document exceeded budget."))?;
    Ok(out.0)
}

/// Parses vertices and `<b:beam>` / `<beam>` elements from a 3MF model XML byte slice.
pub fn import_beam_lattice_3mf(xml_bytes: &[u8]) -> Result<(Vec<[f64; 3]>, Vec<ThreeMfBeam>)> {
    let xml = std::str::from_utf8(xml_bytes)
        .map_err(|_| error("3MF XML must be valid UTF-8."))?;
    check(xml.len() <= MAX_MODEL_BYTES, "3MF XML exceeds size limit.")?;
    let attr = |tag: &str, key: &str| -> Option<f64> {
        let pat = format!("{key}=\"");
        let start = tag.find(&pat)? + pat.len();
        let end = tag[start..].find('"')? + start;
        tag[start..end].parse::<f64>().ok().filter(|v| v.is_finite())
    };
    let mut nodes = Vec::new();
    let mut beams = Vec::new();
    let mut rest = xml;
    while let Some(open) = rest.find('<') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('>') else {
            break;
        };
        let tag = &after[..close];
        if tag.starts_with("vertex ") {
            if let (Some(x), Some(y), Some(z)) = (attr(tag, "x"), attr(tag, "y"), attr(tag, "z")) {
                nodes.push([x, y, z]);
            }
        } else if tag.starts_with("b:beam ") || tag.starts_with("beam ") {
            let v1 = attr(tag, "v1").map(|v| v as usize);
            let v2 = attr(tag, "v2").map(|v| v as usize);
            let r1 = attr(tag, "r1").unwrap_or(1.0);
            let r2 = attr(tag, "r2").unwrap_or(r1);
            if let (Some(v1), Some(v2)) = (v1, v2) {
                check(
                    v1 < nodes.len() && v2 < nodes.len() && v1 != v2 && r1 > 0.0 && r2 > 0.0,
                    "Invalid 3MF beam vertex index or radius.",
                )?;
                beams.push(ThreeMfBeam {
                    v1,
                    v2,
                    r1_mm: r1,
                    r2_mm: r2,
                });
            }
        }
        rest = &after[close + 1..];
    }
    check(
        !nodes.is_empty() && !beams.is_empty(),
        "No valid 3MF beam lattice found in XML.",
    )?;
    Ok((nodes, beams))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beam_lattice_3mf_roundtrip_with_materials_and_metadata() {
        let nodes = vec![[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [5.0, 8.66, 10.0]];
        let beams = vec![
            ThreeMfBeam {
                v1: 0,
                v2: 1,
                r1_mm: 0.8,
                r2_mm: 0.8,
            },
            ThreeMfBeam {
                v1: 1,
                v2: 2,
                r1_mm: 0.8,
                r2_mm: 0.6,
            },
        ];
        let mats = vec![ThreeMfMaterial {
            name: "Ti6Al4V".into(),
            display_color: "#8899AAFF".into(),
        }];
        let xml = export_beam_lattice_3mf(
            &nodes,
            &beams,
            0.1,
            0.8,
            &mats,
            &[("Title", "Aerospace Bracket Lattice")],
        )
        .unwrap();
        let text = std::str::from_utf8(&xml).unwrap();
        assert!(text.contains("<basematerials id=\"1\">"));
        assert!(text.contains("Aerospace Bracket Lattice"));
        let (parsed_nodes, parsed_beams) = import_beam_lattice_3mf(&xml).unwrap();
        assert_eq!(parsed_nodes, nodes);
        assert_eq!(parsed_beams, beams);
    }
}


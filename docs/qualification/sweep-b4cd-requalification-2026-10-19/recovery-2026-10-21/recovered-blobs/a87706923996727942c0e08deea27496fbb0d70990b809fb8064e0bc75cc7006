//! DXF 2D/3D ASCII export and import (`LINE`, `LWPOLYLINE`, `3DFACE`) for laser cutting, CNC, and CAD interchange.
use crate::{MeshView, Result, check, error};
use std::fmt::Write;

/// Exports 2D contour rings as an ASCII DXF (`LWPOLYLINE` entities on layer `0`).
pub fn export_dxf_polylines(rings: &[Vec<[f64; 2]>], closed: bool) -> Result<String> {
    check(!rings.is_empty(), "DXF export requires at least one polyline ring.")?;
    let mut out = String::from("0\nSECTION\n2\nENTITIES\n");
    for ring in rings {
        check(
            ring.len() >= 2 && ring.iter().flatten().all(|v| v.is_finite()),
            "DXF polyline ring requires at least 2 finite points.",
        )?;
        writeln!(
            out,
            "0\nLWPOLYLINE\n8\n0\n90\n{}\n70\n{}",
            ring.len(),
            if closed { 1 } else { 0 }
        )
        .unwrap();
        for &[x, y] in ring {
            writeln!(out, "10\n{x}\n20\n{y}").unwrap();
        }
        check(out.len() <= 4 * 1024 * 1024, "DXF export exceeds 4 MiB.")?;
    }
    out.push_str("0\nENDSEC\n0\nEOF\n");
    Ok(out)
}

/// Exports a 3D triangle mesh as an ASCII DXF (`3DFACE` entities on layer `0`).
pub fn export_dxf_mesh(mesh: &MeshView<'_>) -> Result<String> {
    let report = mesh.inspect()?;
    check(
        report.triangle_count > 0,
        "DXF mesh export requires at least one triangle.",
    )?;
    let mut out = String::from("0\nSECTION\n2\nENTITIES\n");
    for t in mesh.indices.as_chunks::<3>().0 {
        let a = mesh.point(t[0])?;
        let b = mesh.point(t[1])?;
        let c = mesh.point(t[2])?;
        writeln!(
            out,
            "0\n3DFACE\n8\n0\n10\n{}\n20\n{}\n30\n{}\n11\n{}\n21\n{}\n31\n{}\n12\n{}\n22\n{}\n32\n{}\n13\n{}\n23\n{}\n33\n{}",
            a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2], c[0], c[1], c[2]
        )
        .unwrap();
        check(out.len() <= 4 * 1024 * 1024, "DXF export exceeds 4 MiB.")?;
    }
    out.push_str("0\nENDSEC\n0\nEOF\n");
    Ok(out)
}

/// Imports 2D polylines (`LWPOLYLINE` and `LINE` entities) from an ASCII DXF string.
pub fn import_dxf_polylines(dxf: &str) -> Result<Vec<Vec<[f64; 2]>>> {
    check(dxf.len() <= 4 * 1024 * 1024, "DXF input exceeds 4 MiB.")?;
    let mut lines = dxf.lines().map(str::trim);
    let mut pairs = Vec::new();
    while let Some(code_str) = lines.next() {
        if code_str.is_empty() {
            continue;
        }
        let val_str = lines
            .next()
            .ok_or_else(|| error("Unexpected EOF in DXF group pair."))?;
        let code: i32 = code_str
            .parse()
            .map_err(|_| error("Invalid DXF group code."))?;
        pairs.push((code, val_str));
    }
    let mut rings = Vec::new();
    let mut idx = 0;
    while idx < pairs.len() {
        if pairs[idx].0 == 0 && pairs[idx].1 == "LWPOLYLINE" {
            idx += 1;
            let mut ring = Vec::new();
            let mut pending_x: Option<f64> = None;
            while idx < pairs.len() && pairs[idx].0 != 0 {
                let (code, val) = pairs[idx];
                if code == 10 {
                    let x: f64 = val.parse().map_err(|_| error("Invalid DXF X coordinate."))?;
                    check(x.is_finite(), "Non-finite DXF X coordinate.")?;
                    pending_x = Some(x);
                } else if code == 20
                    && let Some(x) = pending_x.take()
                {
                    let y: f64 = val.parse().map_err(|_| error("Invalid DXF Y coordinate."))?;
                    check(y.is_finite(), "Non-finite DXF Y coordinate.")?;
                    ring.push([x, y]);
                }
                idx += 1;
            }
            if ring.len() >= 2 {
                rings.push(ring);
            }
        } else if pairs[idx].0 == 0 && pairs[idx].1 == "LINE" {
            idx += 1;
            let (mut x0, mut y0, mut x1, mut y1) = (None, None, None, None);
            while idx < pairs.len() && pairs[idx].0 != 0 {
                let (code, val) = pairs[idx];
                match code {
                    10 => x0 = val.parse::<f64>().ok().filter(|v| v.is_finite()),
                    20 => y0 = val.parse::<f64>().ok().filter(|v| v.is_finite()),
                    11 => x1 = val.parse::<f64>().ok().filter(|v| v.is_finite()),
                    21 => y1 = val.parse::<f64>().ok().filter(|v| v.is_finite()),
                    _ => {}
                }
                idx += 1;
            }
            if let (Some(ax), Some(ay), Some(bx), Some(by)) = (x0, y0, x1, y1) {
                rings.push(vec![[ax, ay], [bx, by]]);
            }
        } else {
            idx += 1;
        }
    }
    check(!rings.is_empty(), "No valid LWPOLYLINE or LINE entities found in DXF.")?;
    Ok(rings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dxf_polyline_roundtrip() {
        let rings = vec![vec![[0.0, 0.0], [10.0, 0.0], [10.0, 5.0], [0.0, 5.0]]];
        let dxf = export_dxf_polylines(&rings, true).unwrap();
        let parsed = import_dxf_polylines(&dxf).unwrap();
        assert_eq!(parsed, rings);
    }
}

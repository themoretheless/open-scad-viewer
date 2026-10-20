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

/// 2D circle entity (`CIRCLE` in DXF).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DxfCircle2D {
    pub center: [f64; 2],
    pub radius: f64,
}

/// 2D circular arc entity (`ARC` in DXF, angles in degrees counter-clockwise).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DxfArc2D {
    pub center: [f64; 2],
    pub radius: f64,
    pub start_angle_deg: f64,
    pub end_angle_deg: f64,
}

/// Exports 2D polylines, circles, and circular arcs into a single ASCII DXF `ENTITIES` section.
pub fn export_dxf_drawing(
    polylines: &[Vec<[f64; 2]>],
    closed_polylines: bool,
    circles: &[DxfCircle2D],
    arcs: &[DxfArc2D],
    layer: &str,
) -> Result<String> {
    check(
        !polylines.is_empty() || !circles.is_empty() || !arcs.is_empty(),
        "DXF drawing export requires at least one entity.",
    )?;
    let layer_name = if layer.trim().is_empty() {
        "0"
    } else {
        layer.trim()
    };
    let mut out = String::from("0\nSECTION\n2\nENTITIES\n");
    for ring in polylines {
        check(
            ring.len() >= 2 && ring.iter().flatten().all(|v| v.is_finite()),
            "DXF polyline ring requires at least 2 finite points.",
        )?;
        writeln!(
            out,
            "0\nLWPOLYLINE\n8\n{layer_name}\n90\n{}\n70\n{}",
            ring.len(),
            if closed_polylines { 1 } else { 0 }
        )
        .unwrap();
        for &[x, y] in ring {
            writeln!(out, "10\n{x}\n20\n{y}").unwrap();
        }
        check(out.len() <= 4 * 1024 * 1024, "DXF export exceeds 4 MiB.")?;
    }
    for c in circles {
        check(
            c.center[0].is_finite() && c.center[1].is_finite() && c.radius.is_finite() && c.radius > 0.0,
            "DXF circle requires finite center and positive radius.",
        )?;
        writeln!(
            out,
            "0\nCIRCLE\n8\n{layer_name}\n10\n{}\n20\n{}\n40\n{}",
            c.center[0], c.center[1], c.radius
        )
        .unwrap();
        check(out.len() <= 4 * 1024 * 1024, "DXF export exceeds 4 MiB.")?;
    }
    for a in arcs {
        check(
            a.center[0].is_finite()
                && a.center[1].is_finite()
                && a.radius.is_finite()
                && a.radius > 0.0
                && a.start_angle_deg.is_finite()
                && a.end_angle_deg.is_finite(),
            "DXF arc requires finite center, positive radius, and finite angles.",
        )?;
        writeln!(
            out,
            "0\nARC\n8\n{layer_name}\n10\n{}\n20\n{}\n40\n{}\n50\n{}\n51\n{}",
            a.center[0], a.center[1], a.radius, a.start_angle_deg, a.end_angle_deg
        )
        .unwrap();
        check(out.len() <= 4 * 1024 * 1024, "DXF export exceeds 4 MiB.")?;
    }
    out.push_str("0\nENDSEC\n0\nEOF\n");
    Ok(out)
}

/// Imports 2D polylines (`LWPOLYLINE`, `POLYLINE`/`VERTEX`, `LINE`, `CIRCLE`, `ARC`, and `SPLINE` entities)
/// from an ASCII DXF string, tessellating curved entities into polyline chains.
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
        if pairs[idx].0 == 0 && (pairs[idx].1 == "LWPOLYLINE" || pairs[idx].1 == "SPLINE") {
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
        } else if pairs[idx].0 == 0 && pairs[idx].1 == "POLYLINE" {
            idx += 1;
            while idx < pairs.len() && pairs[idx].0 != 0 {
                idx += 1;
            }
            let mut ring = Vec::new();
            while idx < pairs.len() {
                if pairs[idx].0 == 0 && pairs[idx].1 == "VERTEX" {
                    idx += 1;
                    let (mut vx, mut vy) = (None, None);
                    while idx < pairs.len() && pairs[idx].0 != 0 {
                        match pairs[idx].0 {
                            10 => vx = pairs[idx].1.parse::<f64>().ok().filter(|v| v.is_finite()),
                            20 => vy = pairs[idx].1.parse::<f64>().ok().filter(|v| v.is_finite()),
                            _ => {}
                        }
                        idx += 1;
                    }
                    if let (Some(x), Some(y)) = (vx, vy) {
                        ring.push([x, y]);
                    }
                } else if pairs[idx].0 == 0 && pairs[idx].1 == "SEQEND" {
                    idx += 1;
                    while idx < pairs.len() && pairs[idx].0 != 0 {
                        idx += 1;
                    }
                    break;
                } else {
                    break;
                }
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
        } else if pairs[idx].0 == 0 && pairs[idx].1 == "CIRCLE" {
            idx += 1;
            let (mut cx, mut cy, mut r) = (None, None, None);
            while idx < pairs.len() && pairs[idx].0 != 0 {
                let (code, val) = pairs[idx];
                match code {
                    10 => cx = val.parse::<f64>().ok().filter(|v| v.is_finite()),
                    20 => cy = val.parse::<f64>().ok().filter(|v| v.is_finite()),
                    40 => r = val.parse::<f64>().ok().filter(|v| v.is_finite() && *v > 0.0),
                    _ => {}
                }
                idx += 1;
            }
            if let (Some(cx), Some(cy), Some(r)) = (cx, cy, r) {
                let steps = 32;
                let mut ring = Vec::with_capacity(steps);
                for i in 0..steps {
                    let a = std::f64::consts::TAU * (i as f64) / (steps as f64);
                    ring.push([cx + r * a.cos(), cy + r * a.sin()]);
                }
                rings.push(ring);
            }
        } else if pairs[idx].0 == 0 && pairs[idx].1 == "ARC" {
            idx += 1;
            let (mut cx, mut cy, mut r, mut a0, mut a1) = (None, None, None, None, None);
            while idx < pairs.len() && pairs[idx].0 != 0 {
                let (code, val) = pairs[idx];
                match code {
                    10 => cx = val.parse::<f64>().ok().filter(|v| v.is_finite()),
                    20 => cy = val.parse::<f64>().ok().filter(|v| v.is_finite()),
                    40 => r = val.parse::<f64>().ok().filter(|v| v.is_finite() && *v > 0.0),
                    50 => a0 = val.parse::<f64>().ok().filter(|v| v.is_finite()),
                    51 => a1 = val.parse::<f64>().ok().filter(|v| v.is_finite()),
                    _ => {}
                }
                idx += 1;
            }
            if let (Some(cx), Some(cy), Some(r), Some(deg0), Some(mut deg1)) = (cx, cy, r, a0, a1) {
                while deg1 <= deg0 {
                    deg1 += 360.0;
                }
                let steps = 16;
                let mut ring = Vec::with_capacity(steps + 1);
                for i in 0..=steps {
                    let deg = deg0 + (deg1 - deg0) * (i as f64) / (steps as f64);
                    let rad = deg.to_radians();
                    ring.push([cx + r * rad.cos(), cy + r * rad.sin()]);
                }
                rings.push(ring);
            }
        } else {
            idx += 1;
        }
    }
    check(
        !rings.is_empty(),
        "No valid 2D entities (LWPOLYLINE, POLYLINE, LINE, CIRCLE, ARC, SPLINE) found in DXF.",
    )?;
    Ok(rings)
}

/// Imports a 3D triangle mesh from `3DFACE` entities in an ASCII DXF string.
pub fn import_dxf_3dfaces(dxf: &str) -> Result<(Vec<f64>, Vec<usize>)> {
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
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    let mut idx = 0;
    while idx < pairs.len() {
        if pairs[idx].0 == 0 && pairs[idx].1 == "3DFACE" {
            idx += 1;
            let mut pts = [[0.0_f64; 3]; 4];
            let mut mask = [false; 12];
            while idx < pairs.len() && pairs[idx].0 != 0 {
                let (code, val) = pairs[idx];
                let slot = match code {
                    10 => Some((0, 0)),
                    20 => Some((0, 1)),
                    30 => Some((0, 2)),
                    11 => Some((1, 0)),
                    21 => Some((1, 1)),
                    31 => Some((1, 2)),
                    12 => Some((2, 0)),
                    22 => Some((2, 1)),
                    32 => Some((2, 2)),
                    13 => Some((3, 0)),
                    23 => Some((3, 1)),
                    33 => Some((3, 2)),
                    _ => None,
                };
                if let Some((vi, axis)) = slot {
                    let v: f64 = val.parse().map_err(|_| error("Invalid 3DFACE coordinate."))?;
                    check(v.is_finite(), "Non-finite 3DFACE coordinate.")?;
                    pts[vi][axis] = v;
                    mask[vi * 3 + axis] = true;
                }
                idx += 1;
            }
            if mask[..9].iter().all(|&b| b) {
                let base = positions.len() / 3;
                for v in &pts[..3] {
                    positions.extend_from_slice(v);
                }
                indices.extend_from_slice(&[base, base + 1, base + 2]);
                if mask[9..12].iter().all(|&b| b) && pts[3] != pts[2] {
                    positions.extend_from_slice(&pts[3]);
                    indices.extend_from_slice(&[base, base + 2, base + 3]);
                }
            }
        } else {
            idx += 1;
        }
    }
    check(!indices.is_empty(), "No valid 3DFACE entities found in DXF.")?;
    Ok((positions, indices))
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

    #[test]
    fn dxf_circles_arcs_and_3dfaces_roundtrip() {
        let dxf = export_dxf_drawing(
            &[vec![[0.0, 0.0], [5.0, 0.0]]],
            false,
            &[DxfCircle2D {
                center: [10.0, 10.0],
                radius: 3.0,
            }],
            &[DxfArc2D {
                center: [0.0, 0.0],
                radius: 5.0,
                start_angle_deg: 0.0,
                end_angle_deg: 90.0,
            }],
            "CUT",
        )
        .unwrap();
        let imported = import_dxf_polylines(&dxf).unwrap();
        assert_eq!(imported.len(), 3);
        assert_eq!(imported[1].len(), 32);
        assert_eq!(imported[2].len(), 17);

        let positions = [0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 10.0, 0.0];
        let indices = [0usize, 1, 2];
        let view = MeshView {
            positions: &positions,
            indices: &indices,
            uv: None,
        };
        let dxf_3d = export_dxf_mesh(&view).unwrap();
        let (pos_out, idx_out) = import_dxf_3dfaces(&dxf_3d).unwrap();
        assert_eq!(pos_out.len(), 9);
        assert_eq!(idx_out, vec![0, 1, 2]);
    }
}


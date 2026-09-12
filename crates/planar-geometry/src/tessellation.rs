//! Planar fill tessellation: flatten → ear-clip (with hole bridges).
//!
//! Target surface for curvex/egui fill meshes without lyon. Stroke fills by
//! expanding to an outline path first (`stroke`), then tessellating.
use crate::path::{BezierPath, FLATTEN_TOLERANCE};
use crate::rings::{self, Rings, area};
use crate::{Result, check};
use math_core::{cross2, sub2};

const MAX_VERTICES: usize = 65_536;
const MAX_TRIANGLES: usize = 131_072;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}

/// Triangle mesh in document mm (XY). Indices are triangles (i0,i1,i2).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FillMesh {
    pub positions: Vec<[f64; 2]>,
    pub indices: Vec<u32>,
}

impl FillMesh {
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty() || self.indices.is_empty()
    }
}

/// Tessellate closed polyline rings (outer + holes). Rings must be closed
/// without repeating the first point at the end.
pub fn tessellate_rings(rings: &Rings, rule: FillRule) -> Result<FillMesh> {
    check(!rings.is_empty(), "No rings to tessellate")?;
    let cleaned: Rings = rings
        .iter()
        .filter_map(|r| clean_ring(r))
        .collect();
    check(!cleaned.is_empty(), "No usable rings after cleanup")?;
    let total: usize = cleaned.iter().map(|r| r.len()).sum();
    check(total <= MAX_VERTICES, "Tessellation vertex budget exceeded")?;

    // EvenOdd with multiple components: tessellate each ring independently and
    // concatenate (overlapping areas remain overlapping triangles; callers that
    // need exact even-odd coverage should pre-boolean). NonZero nests holes.
    if matches!(rule, FillRule::EvenOdd) || cleaned.len() == 1 {
        let mut mesh = FillMesh::default();
        for ring in &cleaned {
            let piece = earclip_simple(ring)?;
            append_mesh(&mut mesh, piece)?;
        }
        return Ok(mesh);
    }

    // NonZero: partition into outer rings and holes by nesting depth.
    let groups = nest_groups(&cleaned)?;
    let mut mesh = FillMesh::default();
    for (outer, holes) in groups {
        let combined = if holes.is_empty() {
            outer
        } else {
            bridge_holes(outer, holes)?
        };
        let piece = earclip_simple(&combined)?;
        append_mesh(&mut mesh, piece)?;
    }
    Ok(mesh)
}

/// Flatten a closed Bézier path and tessellate. Open paths error.
pub fn tessellate_path(
    path: &BezierPath,
    tolerance: f64,
    rule: FillRule,
) -> Result<FillMesh> {
    check(path.closed, "Fill tessellation requires a closed path")?;
    check(tolerance > 0.0, "Invalid flatten tolerance")?;
    let ring = path_to_ring(path, tolerance)?;
    tessellate_rings(&vec![ring], rule)
}

pub fn tessellate_path_default(path: &BezierPath) -> Result<FillMesh> {
    tessellate_path(path, FLATTEN_TOLERANCE, FillRule::NonZero)
}

fn path_to_ring(path: &BezierPath, tolerance: f64) -> Result<Vec<[f64; 2]>> {
    let pts = path.flatten_tol(tolerance)?;
    check(pts.len() >= 3, "Closed path too short to fill")?;
    // Closed flatten may repeat start at end — drop duplicate.
    let mut ring = pts;
    if ring.len() >= 2 && dist(ring[0], *ring.last().unwrap()) < 1e-9 {
        ring.pop();
    }
    clean_ring(&ring).ok_or_else(|| crate::error("Degenerate fill ring"))
}

fn clean_ring(ring: &[[f64; 2]]) -> Option<Vec<[f64; 2]>> {
    if ring.len() < 3 {
        return None;
    }
    let mut out = Vec::with_capacity(ring.len());
    for &p in ring {
        if !p[0].is_finite() || !p[1].is_finite() {
            return None;
        }
        if out.last().is_none_or(|q| dist(*q, p) > 1e-12) {
            out.push(p);
        }
    }
    if out.len() >= 2 && dist(out[0], *out.last().unwrap()) < 1e-12 {
        out.pop();
    }
    if out.len() < 3 || area(&out).abs() < 1e-18 {
        return None;
    }
    Some(out)
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    let d = sub2(a, b);
    (d[0] * d[0] + d[1] * d[1]).sqrt()
}

fn append_mesh(dst: &mut FillMesh, src: FillMesh) -> Result<()> {
    let base = dst.positions.len() as u32;
    check(
        dst.triangle_count() + src.triangle_count() <= MAX_TRIANGLES,
        "Tessellation triangle budget exceeded",
    )?;
    dst.positions.extend(src.positions);
    dst.indices
        .extend(src.indices.into_iter().map(|i| i + base));
    Ok(())
}

/// Groups: each outer CCW ring with its CW holes (directly nested once).
fn nest_groups(rings: &Rings) -> Result<Vec<(Vec<[f64; 2]>, Vec<Vec<[f64; 2]>>)>> {
    let n = rings.len();
    let mut depth = vec![0usize; n];
    for i in 0..n {
        let probe = rings[i][0];
        for (j, other) in rings.iter().enumerate() {
            if i == j {
                continue;
            }
            if rings::contains_point(probe, other) {
                depth[i] += 1;
            }
        }
    }
    let mut groups = Vec::new();
    for i in 0..n {
        if depth[i] % 2 != 0 {
            continue; // hole relative to NonZero nesting
        }
        let mut outer = rings[i].clone();
        if area(&outer) < 0.0 {
            outer.reverse();
        }
        let mut holes = Vec::new();
        for j in 0..n {
            if depth[j] != depth[i] + 1 {
                continue;
            }
            // Hole must lie inside this outer.
            if !rings::contains_point(rings[j][0], &outer) {
                continue;
            }
            // And not inside a deeper sibling outer at same depth as i.
            let mut inside_sibling = false;
            for k in 0..n {
                if k == i || depth[k] != depth[i] {
                    continue;
                }
                if rings::contains_point(rings[j][0], &rings[k]) {
                    inside_sibling = true;
                    break;
                }
            }
            if inside_sibling {
                continue;
            }
            let mut hole = rings[j].clone();
            if area(&hole) > 0.0 {
                hole.reverse(); // holes CW
            }
            holes.push(hole);
        }
        groups.push((outer, holes));
    }
    check(!groups.is_empty(), "No outer rings for NonZero fill")?;
    Ok(groups)
}

/// Bridge each hole into the outer with a zero-area corridor, then ear-clip.
fn bridge_holes(mut outer: Vec<[f64; 2]>, mut holes: Vec<Vec<[f64; 2]>>) -> Result<Vec<[f64; 2]>> {
    // Process holes rightmost-first for stable bridges.
    holes.sort_by(|a, b| {
        let ax = a.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
        let bx = b.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
        bx.total_cmp(&ax)
    });
    for hole in holes {
        outer = bridge_one(outer, hole)?;
    }
    Ok(outer)
}

fn bridge_one(outer: Vec<[f64; 2]>, hole: Vec<[f64; 2]>) -> Result<Vec<[f64; 2]>> {
    check(hole.len() >= 3, "Degenerate hole")?;
    let (hi, &hp) = hole
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])))
        .unwrap();
    let mut best: Option<(usize, f64)> = None;
    for (oi, &op) in outer.iter().enumerate() {
        if op[0] + 1e-12 < hp[0] {
            continue;
        }
        if bridge_crosses(&outer, &hole, oi, hi) {
            continue;
        }
        let ang = (op[1] - hp[1]).atan2(op[0] - hp[0]);
        if best.is_none_or(|(_, a)| ang < a) {
            best = Some((oi, ang));
        }
    }
    let oi = if let Some((i, _)) = best {
        i
    } else {
        let mut nearest = None;
        for (i, &op) in outer.iter().enumerate() {
            if bridge_crosses(&outer, &hole, i, hi) {
                continue;
            }
            let d = dist(op, hp);
            if nearest.is_none_or(|(_, best_d)| d < best_d) {
                nearest = Some((i, d));
            }
        }
        nearest
            .map(|(i, _)| i)
            .ok_or_else(|| crate::error("No valid hole bridge"))?
    };
    let op = outer[oi];
    // Hairline corridor: nudge the return path so consecutive vertices are not
    // identical (exact duplicates collapse under cleanup and break earclip).
    let bridge = sub2(op, hp);
    let len = (bridge[0] * bridge[0] + bridge[1] * bridge[1]).sqrt().max(1e-12);
    let n = [-bridge[1] / len * 1e-9, bridge[0] / len * 1e-9];
    let hp_back = [hp[0] + n[0], hp[1] + n[1]];
    let op_back = [op[0] + n[0], op[1] + n[1]];

    let mut out = Vec::with_capacity(outer.len() + hole.len() + 4);
    out.extend_from_slice(&outer[..=oi]);
    for k in 0..hole.len() {
        out.push(hole[(hi + k) % hole.len()]);
    }
    out.push(hp_back);
    out.push(op_back);
    if oi + 1 < outer.len() {
        out.extend_from_slice(&outer[oi + 1..]);
    }
    check(out.len() >= 3, "Bridged ring degenerated")?;
    Ok(out)
}

fn bridge_crosses(outer: &[[f64; 2]], hole: &[[f64; 2]], oi: usize, hi: usize) -> bool {
    let a = hole[hi];
    let b = outer[oi];
    // Skip edges incident to bridge endpoints.
    for i in 0..outer.len() {
        let j = (i + 1) % outer.len();
        if i == oi || j == oi {
            continue;
        }
        if segments_cross(a, b, outer[i], outer[j]) {
            return true;
        }
    }
    for i in 0..hole.len() {
        let j = (i + 1) % hole.len();
        if i == hi || j == hi {
            continue;
        }
        if segments_cross(a, b, hole[i], hole[j]) {
            return true;
        }
    }
    false
}

fn segments_cross(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    let d1 = cross2(sub2(b, a), sub2(c, a));
    let d2 = cross2(sub2(b, a), sub2(d, a));
    let d3 = cross2(sub2(d, c), sub2(a, c));
    let d4 = cross2(sub2(d, c), sub2(b, c));
    ((d1 > 0.0) != (d2 > 0.0))
        && ((d3 > 0.0) != (d4 > 0.0))
        && d1.abs() > 1e-18
        && d2.abs() > 1e-18
        && d3.abs() > 1e-18
        && d4.abs() > 1e-18
}

fn earclip_simple(ring: &[[f64; 2]]) -> Result<FillMesh> {
    let mut poly: Vec<[f64; 2]> = ring.to_vec();
    if area(&poly) < 0.0 {
        poly.reverse();
    }
    let n0 = poly.len();
    check(n0 >= 3, "Polygon needs ≥3 vertices")?;
    check(n0 <= MAX_VERTICES, "Tessellation vertex budget exceeded")?;

    // Map working indices → original positions after optional reverse.
    let positions = poly.clone();
    let mut idx: Vec<usize> = (0..n0).collect();
    let mut indices = Vec::new();

    let mut guard = 0;
    while idx.len() > 3 {
        guard += 1;
        check(guard < n0 * n0 + 8, "Ear clipping failed to converge")?;
        let mut clipped = false;
        let m = idx.len();
        for i in 0..m {
            let i0 = idx[(i + m - 1) % m];
            let i1 = idx[i];
            let i2 = idx[(i + 1) % m];
            let a = positions[i0];
            let b = positions[i1];
            let c = positions[i2];
            if !is_convex(a, b, c) {
                continue;
            }
            if ear_contains_point(&positions, &idx, i0, i1, i2) {
                continue;
            }
            indices.extend([i0 as u32, i1 as u32, i2 as u32]);
            idx.remove(i);
            clipped = true;
            break;
        }
        check(clipped, "No ear found (self-intersecting or degenerate)")?;
        check(
            indices.len() / 3 <= MAX_TRIANGLES,
            "Tessellation triangle budget exceeded",
        )?;
    }
    indices.extend([idx[0] as u32, idx[1] as u32, idx[2] as u32]);
    Ok(FillMesh {
        positions,
        indices,
    })
}

fn is_convex(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> bool {
    cross2(sub2(b, a), sub2(c, b)) > 1e-15
}

fn point_in_tri(p: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> bool {
    let d1 = cross2(sub2(b, a), sub2(p, a));
    let d2 = cross2(sub2(c, b), sub2(p, b));
    let d3 = cross2(sub2(a, c), sub2(p, c));
    let has_neg = d1 < -1e-15 || d2 < -1e-15 || d3 < -1e-15;
    let has_pos = d1 > 1e-15 || d2 > 1e-15 || d3 > 1e-15;
    !(has_neg && has_pos)
}

fn ear_contains_point(
    positions: &[[f64; 2]],
    idx: &[usize],
    i0: usize,
    i1: usize,
    i2: usize,
) -> bool {
    let a = positions[i0];
    let b = positions[i1];
    let c = positions[i2];
    for &j in idx {
        if j == i0 || j == i1 || j == i2 {
            continue;
        }
        if point_in_tri(positions[j], a, b, c) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::PathSegment;

    fn signed_area_mesh(mesh: &FillMesh) -> f64 {
        let mut a = 0.0;
        for tri in mesh.indices.chunks_exact(3) {
            let p0 = mesh.positions[tri[0] as usize];
            let p1 = mesh.positions[tri[1] as usize];
            let p2 = mesh.positions[tri[2] as usize];
            a += cross2(sub2(p1, p0), sub2(p2, p0)) * 0.5;
        }
        a
    }

    #[test]
    fn tessellate_unit_square() {
        let path = BezierPath::from_rect([0., 0.], [2., 3.]).unwrap();
        let mesh = tessellate_path_default(&path).unwrap();
        assert_eq!(mesh.triangle_count(), 2);
        assert!((signed_area_mesh(&mesh) - 6.0).abs() < 1e-9);
    }

    #[test]
    fn tessellate_with_hole() {
        let outer = vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]];
        let hole = vec![[3., 3.], [3., 7.], [7., 7.], [7., 3.]]; // CW relative after nest
        let mesh = tessellate_rings(&vec![outer, hole], FillRule::NonZero).unwrap();
        assert!(mesh.triangle_count() >= 8);
        let a = signed_area_mesh(&mesh).abs();
        assert!((a - (100.0 - 16.0)).abs() < 1e-6, "area={a}");
    }

    #[test]
    fn tessellate_cubic_closed() {
        let path = BezierPath::closed(
            [0., 0.],
            vec![
                PathSegment::Cubic {
                    c1: [0., 2.],
                    c2: [2., 2.],
                    to: [2., 0.],
                },
                PathSegment::Line { to: [0., 0.] },
            ],
        )
        .unwrap();
        let mesh = tessellate_path(&path, 0.1, FillRule::NonZero).unwrap();
        assert!(mesh.triangle_count() >= 1);
        assert!(signed_area_mesh(&mesh) > 0.5);
    }

    #[test]
    fn open_path_rejected() {
        let path = BezierPath::from_polyline(&[[0., 0.], [1., 0.], [1., 1.]], false).unwrap();
        assert!(tessellate_path_default(&path).is_err());
    }
}

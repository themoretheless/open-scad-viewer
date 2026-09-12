//! Fill tessellation for compound paths with explicit winding rules.
//!
//! Resolve the planar arrangement, then sweep horizontal bands into disjoint
//! trapezoids. Holes need no artificial bridge and self intersections become
//! ordinary vertices before triangulation.
use crate::path::{BezierPath, FLATTEN_TOLERANCE};
use crate::rings::{self, Rings};
use crate::{Result, check};
use math_core::{cross2, sub2};
use std::collections::{BTreeSet, HashMap};

const MAX_VERTICES: usize = 65_536;
const MAX_TRIANGLES: usize = 131_072;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}

/// Triangle mesh in document mm (XY). Indices are counterclockwise triangles.
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

/// Tessellate compound closed contours. Rings may overlap, self-intersect or
/// have either winding; the fill rule applies to their combined winding.
/// Repeating the first point is accepted. Empty or fully cancelled fills return
/// an empty mesh, while malformed coordinates produce an error.
pub fn tessellate_rings(input: &Rings, rule: FillRule) -> Result<FillMesh> {
    check(
        input.iter().map(Vec::len).sum::<usize>() <= MAX_VERTICES,
        "Tessellation vertex budget exceeded",
    )?;
    rings::coordinate_metrics(input.iter().flatten())?;
    let cleaned: Rings = input
        .iter()
        .map(|ring| clean_ring(ring))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|ring| ring.len() >= 3)
        .collect();
    if cleaned.len() == 1 {
        if let Some(positions) = rings::convex_boundary(&cleaned[0]) {
            let indices = (1..positions.len() - 1)
                .flat_map(|i| [0, i as u32, i as u32 + 1])
                .collect();
            return Ok(FillMesh { positions, indices });
        }
    }
    let normalized = rings::normalize(&cleaned, rule)?;
    sweep_region(&normalized)
}

/// Flatten a closed Bézier path and tessellate. Open paths error.
pub fn tessellate_path(path: &BezierPath, tolerance: f64, rule: FillRule) -> Result<FillMesh> {
    check(path.closed, "Fill tessellation requires a closed path")?;
    check(
        tolerance.is_finite() && tolerance > 0.0,
        "Invalid flatten tolerance",
    )?;
    tessellate_rings(&vec![path.flatten_tol(tolerance)?], rule)
}

pub fn tessellate_path_default(path: &BezierPath) -> Result<FillMesh> {
    tessellate_path(path, FLATTEN_TOLERANCE, FillRule::NonZero)
}

fn clean_ring(ring: &[[f64; 2]]) -> Result<Vec<[f64; 2]>> {
    let mut out = Vec::with_capacity(ring.len());
    for &p in ring {
        if out.last() != Some(&p) {
            out.push(p);
        }
    }
    if out.len() >= 2 && out.first() == out.last() {
        out.pop();
    }
    // A self-intersecting contour can have zero signed area while enclosing
    // material (e.g. a bowtie), so area is never a cleanup criterion.
    Ok(out)
}

fn sweep_region(rings: &Rings) -> Result<FillMesh> {
    let mut levels: Vec<f64> = rings.iter().flatten().map(|p| p[1]).collect();
    levels.sort_by(f64::total_cmp);
    levels.dedup();
    let edges: Vec<_> = rings
        .iter()
        .flat_map(|r| (0..r.len()).map(move |i| (r[i], r[(i + 1) % r.len()])))
        .filter(|(a, b)| a[1] != b[1])
        .collect();
    let mut mesh = FillMesh::default();
    let mut vertex_ids = HashMap::new();
    let mut starts: Vec<_> = (0..edges.len()).collect();
    let mut ends = starts.clone();
    starts.sort_by(|&a, &b| {
        edges[a].0[1]
            .min(edges[a].1[1])
            .total_cmp(&edges[b].0[1].min(edges[b].1[1]))
    });
    ends.sort_by(|&a, &b| {
        edges[a].0[1]
            .max(edges[a].1[1])
            .total_cmp(&edges[b].0[1].max(edges[b].1[1]))
    });
    let (mut start_cursor, mut end_cursor) = (0, 0);
    let mut active_ids = BTreeSet::new();
    for band in levels.windows(2) {
        let (bottom, top) = (band[0], band[1]);
        while start_cursor < starts.len() {
            let edge = edges[starts[start_cursor]];
            if edge.0[1].min(edge.1[1]) > bottom {
                break;
            }
            active_ids.insert(starts[start_cursor]);
            start_cursor += 1;
        }
        while end_cursor < ends.len() {
            let edge = edges[ends[end_cursor]];
            if edge.0[1].max(edge.1[1]) > bottom {
                break;
            }
            active_ids.remove(&ends[end_cursor]);
            end_cursor += 1;
        }
        let middle = bottom + (top - bottom) * 0.5;
        let x_at = |a: [f64; 2], b: [f64; 2], y: f64| {
            if y == a[1] {
                return a[0];
            }
            if y == b[1] {
                return b[0];
            }
            a[0] + (y - a[1]) / (b[1] - a[1]) * (b[0] - a[0])
        };
        // Normalization split every crossing; active edges cannot exchange
        // order inside this open band. Outer and hole edges alternate here.
        let mut active: Vec<_> = active_ids
            .iter()
            .map(|&index| {
                let (a, b) = edges[index];
                (x_at(a, b, middle), a, b)
            })
            .collect();
        active.sort_by(|a, b| a.0.total_cmp(&b.0));
        check(active.len() % 2 == 0, "Unbalanced fill boundary")?;
        for pair in active.chunks_exact(2) {
            let (_, a, b) = pair[0];
            let (_, c, d) = pair[1];
            let lb = [x_at(a, b, bottom), bottom];
            let rb = [x_at(c, d, bottom), bottom];
            let rt = [x_at(c, d, top), top];
            let lt = [x_at(a, b, top), top];
            append_triangle(&mut mesh, &mut vertex_ids, lb, rb, rt)?;
            append_triangle(&mut mesh, &mut vertex_ids, lb, rt, lt)?;
        }
    }
    Ok(mesh)
}

/// Internal fast route for a region just produced by the normalization/stroke
/// kernel. Callers must not pass raw, overlapping or self-crossing contours.
pub(crate) fn tessellate_normalized_rings(rings: &Rings) -> Result<FillMesh> {
    rings::coordinate_metrics(rings.iter().flatten())?;
    sweep_region(rings)
}

fn append_triangle(
    mesh: &mut FillMesh,
    vertex_ids: &mut HashMap<(u64, u64), u32>,
    a: [f64; 2],
    b: [f64; 2],
    c: [f64; 2],
) -> Result<()> {
    let double_area = cross2(sub2(b, a), sub2(c, a));
    if double_area == 0. {
        return Ok(());
    }
    check(
        double_area.is_finite() && double_area > 0.,
        "Invalid fill triangle",
    )?;
    check(
        mesh.triangle_count() < MAX_TRIANGLES,
        "Tessellation triangle budget exceeded",
    )?;
    for p in [a, b, c] {
        let bits = |v: f64| if v == 0. { 0_u64 } else { v.to_bits() };
        let key = (bits(p[0]), bits(p[1]));
        let index = if let Some(&index) = vertex_ids.get(&key) {
            index
        } else {
            check(
                mesh.positions.len() < MAX_VERTICES,
                "Tessellation vertex budget exceeded",
            )?;
            let index = mesh.positions.len() as u32;
            mesh.positions.push(p);
            vertex_ids.insert(key, index);
            index
        };
        mesh.indices.push(index);
    }
    Ok(())
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

    fn rectangle(min: [f64; 2], max: [f64; 2]) -> Vec<[f64; 2]> {
        vec![min, [max[0], min[1]], max, [min[0], max[1]]]
    }

    /// Probe interiors independently of the sweep: every filled sample must be
    /// covered by exactly one triangle, and empty samples by none. Avoid edges
    /// and diagonals where adjacent triangles legitimately share a sample.
    fn assert_coverage(mesh: &FillMesh, input: &Rings, rule: FillRule) {
        for iy in 0..41 {
            for ix in 0..43 {
                let p = [-1. + ix as f64 * 0.317, -1. + iy as f64 * 0.293];
                let mut count = 0;
                let mut boundary = false;
                for tri in mesh.indices.chunks_exact(3) {
                    let points: Vec<_> = tri.iter().map(|&i| mesh.positions[i as usize]).collect();
                    let sides: Vec<_> = (0..3)
                        .map(|i| cross2(sub2(points[(i + 1) % 3], points[i]), sub2(p, points[i])))
                        .collect();
                    if sides.iter().all(|s| *s >= -1e-9) {
                        if sides.iter().any(|s| s.abs() <= 1e-9) {
                            boundary = true;
                        } else {
                            count += 1;
                        }
                    }
                }
                if !boundary {
                    assert_eq!(
                        count,
                        usize::from(rings::inside_with_rule(p, input, rule)),
                        "coverage at {p:?}"
                    );
                }
            }
        }
    }

    fn assert_fill(input: &Rings, rule: FillRule, expected_area: f64) {
        let mesh = tessellate_rings(input, rule).unwrap();
        assert!(
            (signed_area_mesh(&mesh) - expected_area).abs() < 1e-8,
            "area {}, expected {expected_area}",
            signed_area_mesh(&mesh)
        );
        assert_coverage(&mesh, input, rule);
    }

    #[test]
    fn same_winding_inner_ring_is_only_a_hole_under_evenodd() {
        let input = vec![
            rectangle([0., 0.], [10., 10.]),
            rectangle([2., 2.], [8., 8.]),
        ];
        assert_fill(&input, FillRule::NonZero, 100.);
        assert_fill(&input, FillRule::EvenOdd, 64.);
    }

    #[test]
    fn holes_and_nested_islands_have_exactly_once_coverage() {
        let input = vec![
            rectangle([0., 0.], [10., 10.]),
            rectangle([2., 2.], [8., 8.]).into_iter().rev().collect(),
            rectangle([4., 4.], [6., 6.]),
        ];
        for rule in [FillRule::NonZero, FillRule::EvenOdd] {
            assert_fill(&input, rule, 68.);
        }
    }

    #[test]
    fn overlapping_and_coincident_rings_apply_combined_winding() {
        let a = rectangle([0., 0.], [4., 4.]);
        let b = rectangle([2., 0.], [6., 4.]);
        assert_fill(&vec![a.clone(), b.clone()], FillRule::NonZero, 24.);
        assert_fill(&vec![a.clone(), b], FillRule::EvenOdd, 16.);
        assert_fill(&vec![a.clone(), a.clone()], FillRule::NonZero, 16.);
        assert_fill(&vec![a.clone(), a], FillRule::EvenOdd, 0.);
    }

    #[test]
    fn bowtie_keeps_both_lobes_despite_zero_signed_area() {
        let input = vec![vec![[0., 0.], [4., 4.], [0., 4.], [4., 0.]]];
        for rule in [FillRule::NonZero, FillRule::EvenOdd] {
            assert_fill(&input, rule, 8.);
        }
    }

    #[test]
    fn disjoint_and_point_touching_regions_do_not_overlap() {
        let input = vec![
            rectangle([0., 0.], [2., 2.]),
            rectangle([2., 2.], [4., 4.]),
            rectangle([6., 0.], [8., 2.]).into_iter().rev().collect(),
        ];
        for rule in [FillRule::NonZero, FillRule::EvenOdd] {
            assert_fill(&input, rule, 12.);
        }
    }

    #[test]
    fn empty_and_degenerate_fills_are_empty_but_invalid_coordinates_fail() {
        assert!(
            tessellate_rings(&vec![], FillRule::NonZero)
                .unwrap()
                .is_empty()
        );
        assert!(
            tessellate_rings(&vec![vec![[0., 0.], [1., 0.], [2., 0.]]], FillRule::NonZero)
                .unwrap()
                .is_empty()
        );
        assert!(
            tessellate_rings(
                &vec![rectangle([0., 0.], [f64::INFINITY, 1.])],
                FillRule::NonZero
            )
            .is_err()
        );
        assert!(
            tessellate_rings(
                &vec![rectangle([0., 0.], [1e200, 1e200])],
                FillRule::NonZero
            )
            .is_err()
        );
        let path = BezierPath::from_rect([0., 0.], [1., 1.]).unwrap();
        assert!(tessellate_path(&path, f64::NAN, FillRule::NonZero).is_err());
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

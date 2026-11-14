//! Watertight multi-patch tessellation with analytic normals, consistent
//! winding and a ladder of levels of detail.
//!
//! Each level is a [`shared_tessellation`] mesh: declared seams share one
//! strip of welded vertices, so there are no cracks or T-junctions between
//! patches, and every triangle carries a continuous conservative deviation
//! bound. On top of that this module
//! - picks, per requested tolerance (coarse to fine), the coarsest uniform
//!   grid whose bound meets it, so levels have strictly more triangles and
//!   strictly smaller bounds and plug straight into [`lod::select`];
//! - winds every triangle by its patch orientation (`S_u x S_v` for
//!   [`Orientation::Natural`], swapped for [`Orientation::Reversed`]);
//! - attaches analytic unit normals at every vertex, taken from the
//!   vertex's representative chart and flipped with its orientation, with
//!   the area-weighted mesh normal as an explicitly marked fallback at
//!   singular points;
//! - reports whether the mesh is closed and consistently oriented: every
//!   directed edge appears once and its reverse once.
//!
//! The per-triangle bounds take the smaller of a box enclosure and a
//! second-order Taylor bound from interval jets of the cell; inside one knot
//! span the latter shrinks with the square of the grid step.
//!
//! At a crease seam (a cube edge) a welded vertex gets the normal of one
//! side only; split the seam if per-face normals are needed there.
use crate::tessellation::lod::Level;
use crate::tessellation::shading::{NormalSource, Orientation, VertexNormal, analytic_normal};
use crate::tessellation::shared_tessellation::{self, Seam};
use crate::{Error, Result, check, surface::Surface};
use math_core::{cross, norm, sub};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct LodMesh {
    pub segments: usize,
    pub points: Vec<[f64; 3]>,
    pub normals: Vec<VertexNormal>,
    /// Wound by patch orientation.
    pub triangles: Vec<[usize; 3]>,
    /// Patch of each triangle.
    pub patches: Vec<usize>,
    pub level: Level,
    /// Every directed edge has exactly one reverse partner.
    pub closed: bool,
}

/// Build one level per tolerance (strictly decreasing). Grid sizes are
/// powers of two up to 128 per patch side; a tolerance no grid meets is a
/// typed refusal rather than a coarser level.
pub fn tessellate_lods(
    surfaces: &[Surface],
    seams: &[Seam],
    orientations: &[Orientation],
    tolerances: &[f64],
    max_cells: usize,
    max_agreement_cells: usize,
) -> Result<Vec<LodMesh>> {
    check(
        orientations.len() == surfaces.len(),
        "Watertight tessellation needs one orientation per patch",
    )?;
    check(
        (1..=3).contains(&tolerances.len())
            && tolerances.iter().all(|t| t.is_finite() && *t > 0.)
            && tolerances.windows(2).all(|w| w[1] < w[0]),
        "LOD tolerances must be 1..3 positive values, strictly decreasing",
    )?;
    let mut levels = Vec::with_capacity(tolerances.len());
    let mut segments = 1;
    for &tolerance in tolerances {
        let report = loop {
            check(
                surfaces.len() * segments * segments <= max_cells,
                "Watertight tessellation exceeds its cell budget",
            )?;
            let report = shared_tessellation::tessellate(
                surfaces,
                seams,
                segments,
                tolerance,
                max_cells,
                max_agreement_cells,
            )?;
            if report.mesh.is_none() {
                return Err(Error::new(
                    "NURBS_TESSELLATION_SEAM_REFUSED",
                    "A declared seam failed its edge agreement proof",
                ));
            }
            if report.within_tolerance {
                break report;
            }
            check(segments < 128, "No grid up to 128 segments meets the LOD tolerance")?;
            segments *= 2;
        };
        let mesh = report.mesh.expect("checked above");
        let triangles: Vec<[usize; 3]> = mesh
            .triangles
            .iter()
            .map(|t| {
                let mut v = t.vertices;
                if orientations[t.patch] == Orientation::Reversed {
                    v.swap(1, 2);
                }
                v
            })
            .collect();
        let normals = vertex_normals(surfaces, orientations, &mesh.points, &mesh.charts, &triangles)?;
        let closed = closed(&triangles);
        levels.push(LodMesh {
            segments,
            level: Level {
                triangle_count: triangles.len(),
                error_upper: report.error_upper,
            },
            patches: mesh.triangles.iter().map(|t| t.patch).collect(),
            points: mesh.points,
            normals,
            triangles,
            closed,
        });
        // The next, finer level must have strictly more triangles.
        segments *= 2;
    }
    Ok(levels)
}

fn vertex_normals(
    surfaces: &[Surface],
    orientations: &[Orientation],
    points: &[[f64; 3]],
    charts: &[(usize, [f64; 2])],
    triangles: &[[usize; 3]],
) -> Result<Vec<VertexNormal>> {
    let mut fallback = vec![[0.; 3]; points.len()];
    for t in triangles {
        let n = cross(sub(points[t[1]], points[t[0]]), sub(points[t[2]], points[t[0]]));
        for &v in t {
            for k in 0..3 {
                fallback[v][k] += n[k];
            }
        }
    }
    charts
        .iter()
        .zip(fallback)
        .map(|(&(patch, [u, v]), area)| {
            if let Some(mut n) = analytic_normal(&surfaces[patch], u, v)? {
                if orientations[patch] == Orientation::Reversed {
                    n = n.map(|x| -x);
                }
                return Ok(VertexNormal {
                    vector: n,
                    source: NormalSource::Analytic,
                });
            }
            let length = norm(area);
            check(
                length.is_finite() && length > 0.,
                "A collapsed vertex neighbourhood has no normal",
            )?;
            Ok(VertexNormal {
                vector: area.map(|x| x / length),
                source: NormalSource::MeshFallback,
            })
        })
        .collect()
}

/// Closed and consistently oriented: each directed edge occurs once and
/// its reverse occurs once. Degenerate triangles (a repeated vertex, as at
/// a collapsed pole row) are skipped.
fn closed(triangles: &[[usize; 3]]) -> bool {
    let mut directed: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    for t in triangles {
        if t[0] == t[1] || t[1] == t[2] || t[0] == t[2] {
            continue;
        }
        for k in 0..3 {
            *directed.entry((t[k], t[(k + 1) % 3])).or_default() += 1;
        }
    }
    !directed.is_empty()
        && directed
            .iter()
            .all(|(&(a, b), &count)| count == 1 && directed.get(&(b, a)) == Some(&1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tessellation::lod::{Request, select};
    use crate::tessellation::shared_tessellation::Side;

    fn bilinear(c: [[f64; 3]; 4]) -> Surface {
        // c: (u0,v0), (u1,v0), (u0,v1), (u1,v1)
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![vec![c[0].to_vec(), c[2].to_vec()], vec![c[1].to_vec(), c[3].to_vec()]],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }

    fn corner(s: &Surface, u: usize, v: usize) -> [f64; 3] {
        let p = &s.control_points[u * (s.control_points.len() - 1)][v * (s.control_points[0].len() - 1)];
        [p[0], p[1], p[2]]
    }

    fn ends(s: &Surface, side: Side) -> [[f64; 3]; 2] {
        match side {
            Side::U0 => [corner(s, 0, 0), corner(s, 0, 1)],
            Side::U1 => [corner(s, 1, 0), corner(s, 1, 1)],
            Side::V0 => [corner(s, 0, 0), corner(s, 1, 0)],
            Side::V1 => [corner(s, 0, 1), corner(s, 1, 1)],
        }
    }

    /// Declare a seam for every pair of patch sides with the same ends.
    fn seams(surfaces: &[Surface]) -> Vec<Seam> {
        let sides = [Side::U0, Side::U1, Side::V0, Side::V1];
        let mut out = Vec::new();
        for a in 0..surfaces.len() {
            for b in a + 1..surfaces.len() {
                for sa in sides {
                    for sb in sides {
                        let (ea, eb) = (ends(&surfaces[a], sa), ends(&surfaces[b], sb));
                        let reversed = if ea == eb {
                            false
                        } else if ea[0] == eb[1] && ea[1] == eb[0] {
                            true
                        } else {
                            continue;
                        };
                        out.push(Seam {
                            patches: [a, b],
                            sides: [sa, sb],
                            reversed,
                        });
                    }
                }
            }
        }
        out
    }

    /// A unit cube from six bilinear patches, each oriented outward.
    fn cube() -> (Vec<Surface>, Vec<Orientation>) {
        let p = |x: f64, y: f64, z: f64| [x, y, z];
        let faces = [
            [p(0., 0., 0.), p(1., 0., 0.), p(0., 1., 0.), p(1., 1., 0.)],
            [p(0., 0., 1.), p(1., 0., 1.), p(0., 1., 1.), p(1., 1., 1.)],
            [p(0., 0., 0.), p(1., 0., 0.), p(0., 0., 1.), p(1., 0., 1.)],
            [p(0., 1., 0.), p(1., 1., 0.), p(0., 1., 1.), p(1., 1., 1.)],
            [p(0., 0., 0.), p(0., 1., 0.), p(0., 0., 1.), p(0., 1., 1.)],
            [p(1., 0., 0.), p(1., 1., 0.), p(1., 0., 1.), p(1., 1., 1.)],
        ];
        let surfaces: Vec<Surface> = faces.iter().map(|&c| bilinear(c)).collect();
        let orientations = surfaces
            .iter()
            .map(|s| {
                let e = s.evaluate(0.5, 0.5).unwrap();
                let n = e.unit_normal().unwrap();
                let out = sub([e.point[0], e.point[1], e.point[2]], [0.5; 3]);
                if n[0] * out[0] + n[1] * out[1] + n[2] * out[2] > 0. {
                    Orientation::Natural
                } else {
                    Orientation::Reversed
                }
            })
            .collect();
        (surfaces, orientations)
    }

    #[test]
    fn cube_shell_is_closed_outward_and_lod_ready() {
        let (surfaces, orientations) = cube();
        let seams = seams(&surfaces);
        assert_eq!(seams.len(), 12);
        let levels =
            tessellate_lods(&surfaces, &seams, &orientations, &[0.6, 0.1], 100000, 1000000).unwrap();
        assert_eq!(levels.len(), 2);
        for level in &levels {
            assert!(level.closed);
            // Euler characteristic of a sphere-like closed shell.
            let edges = level.triangles.len() * 3 / 2;
            assert_eq!(level.points.len() + level.triangles.len() - edges, 2);
            for (t, &patch) in level.triangles.iter().zip(&level.patches) {
                let [a, b, c] = t.map(|i| level.points[i]);
                let n = cross(sub(b, a), sub(c, a));
                let centre: [f64; 3] = std::array::from_fn(|k| (a[k] + b[k] + c[k]) / 3. - 0.5);
                assert!(n[0] * centre[0] + n[1] * centre[1] + n[2] * centre[2] > 0., "patch {patch}");
            }
            assert!(level.normals.iter().all(|n| n.source == NormalSource::Analytic));
        }
        assert!(levels[1].level.triangle_count > levels[0].level.triangle_count);
        // Without declared seams the same patches are not closed.
        let open = tessellate_lods(&surfaces, &[], &orientations, &[0.6], 100000, 1000000).unwrap();
        assert!(!open[0].closed);
    }

    #[test]
    fn curved_patch_levels_refine_and_feed_the_selector() {
        // A biquadratic dome: finer tolerances need finer grids.
        let mut control_points = vec![vec![vec![0.; 3]; 3]; 3];
        for (i, row) in control_points.iter_mut().enumerate() {
            for (j, p) in row.iter_mut().enumerate() {
                let z = if i == 1 && j == 1 { 1. } else { 0. };
                *p = vec![i as f64 / 2., j as f64 / 2., z];
            }
        }
        let dome = Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points,
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let levels = tessellate_lods(
            &[dome],
            &[],
            &[Orientation::Natural],
            &[0.8, 0.25, 0.06],
            100000,
            1000000,
        )
        .unwrap();
        for w in levels.windows(2) {
            assert!(w[1].level.triangle_count > w[0].level.triangle_count);
            assert!(w[1].level.error_upper < w[0].level.error_upper);
        }
        for (level, tolerance) in levels.iter().zip([0.8, 0.25, 0.06]) {
            assert!(level.level.error_upper <= tolerance);
            assert!(!level.closed);
        }
        let ladder: Vec<Level> = levels.iter().map(|l| l.level).collect();
        let pick = select(
            &ladder,
            None,
            Request {
                pixels_per_unit: 10.,
                pixel_budget: 4.,
                triangle_budget: 1 << 20,
                hysteresis: 1.,
            },
        )
        .unwrap();
        assert!(levels[pick.level].level.error_upper * 10. <= 4.);
        assert!(tessellate_lods(&[], &[], &[], &[0.1], 10, 10).is_err());
        // Tolerances must decrease.
        let (cube, orientations) = cube();
        assert!(tessellate_lods(&cube, &[], &orientations, &[0.1, 0.2], 100000, 1000000).is_err());
    }
}

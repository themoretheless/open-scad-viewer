//! Re-express placed coordinate-plane charts in their authored world coordinates.
//! The 3D edge curves are authoritative. Copying their coordinates into an
//! identity plane avoids an independently rounded affine lift of old UV values.
use crate::{Model, Result};
use nurbs_core::surface::Surface;

fn identity(s: &Surface) -> Option<[usize; 2]> {
    if s.degree_u != 1
        || s.degree_v != 1
        || s.periodic_u
        || s.periodic_v
        || s.control_points.len() != 2
        || s.control_points.iter().any(|r| r.len() != 2)
        || !s.weights.iter().flatten().all(|w| *w == s.weights[0][0])
    {
        return None;
    }
    let domains = [[s.knots_u[1], s.knots_u[2]], [s.knots_v[1], s.knots_v[2]]];
    [[0, 1], [0, 2], [1, 0], [1, 2], [2, 0], [2, 1]]
        .into_iter()
        .find(|axes| {
            let fixed = 3 - axes[0] - axes[1];
            let height = s.control_points[0][0][fixed];
            (0..2).all(|u| {
                (0..2).all(|v| {
                    let p = &s.control_points[u][v];
                    p[axes[0]] == domains[0][u] && p[axes[1]] == domains[1][v] && p[fixed] == height
                })
            })
        })
}

fn authored_lifts_match(model: &Model, face: usize, axes: [usize; 2]) -> bool {
    let f = &model.faces[face];
    let fixed = 3 - axes[0] - axes[1];
    let height = f.surface.control_points[0][0][fixed];
    let bezier = |c: &nurbs_core::curve::Curve| {
        !c.periodic
            && c.control_points.len() == c.degree + 1
            && c.knots[..=c.degree].iter().all(|x| *x == c.knots[c.degree])
            && c.knots[c.control_points.len()..]
                .iter()
                .all(|x| *x == c.knots[c.control_points.len()])
    };
    std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .all(|wire| {
            model.loops[wire].coedges.iter().all(|use_| {
                let edge = &model.edges[use_.edge].curve;
                let uv = &use_.pcurve;
                edge.degree == uv.degree
                    && bezier(edge)
                    && bezier(uv)
                    && (0..edge.control_points.len()).all(|slot| {
                        let i = if use_.reversed {
                            edge.control_points.len() - 1 - slot
                        } else {
                            slot
                        };
                        edge.weights[i] == uv.weights[slot]
                            && edge.control_points[i][fixed] == height
                            && (0..2).all(|k| {
                                edge.control_points[i][axes[k]] == uv.control_points[slot][k]
                            })
                    })
            })
        })
}

pub(crate) fn reexpress(source: &Model, placed: &mut Model) -> Result<()> {
    for face in 0..placed.faces.len() {
        // Retain established UV domains on general planar operation charts.
        // This repair is limited to exact coordinate caps owning rational arcs.
        let f = &source.faces[face];
        let owns_rational_arc = std::iter::once(f.outer)
            .chain(f.holes.iter().copied())
            .flat_map(|wire| source.loops[wire].coedges.iter())
            .any(|c| {
                let edge = &source.edges[c.edge].curve;
                edge.degree > 1 && edge.weights.iter().any(|w| *w != edge.weights[0])
            });
        if !owns_rational_arc {
            continue;
        }
        let Some(source_axes) = identity(&source.faces[face].surface) else {
            continue;
        };
        if identity(&placed.faces[face].surface).is_some()
            || !authored_lifts_match(source, face, source_axes)
        {
            continue;
        }
        let s = &placed.faces[face].surface;
        if s.knots_u != vec![s.knots_u[1], s.knots_u[1], s.knots_u[2], s.knots_u[2]]
            || s.knots_v != vec![s.knots_v[1], s.knots_v[1], s.knots_v[2], s.knots_v[2]]
        {
            continue;
        }
        let p = &s.control_points;
        let uaxis = (0..3).find(|&k| {
            p[0][0][k] != p[1][0][k] && (0..3).all(|j| j == k || p[0][0][j] == p[1][0][j])
        });
        let vaxis = (0..3).find(|&k| {
            p[0][0][k] != p[0][1][k] && (0..3).all(|j| j == k || p[0][0][j] == p[0][1][j])
        });
        let (Some(a), Some(b)) = (uaxis, vaxis) else {
            continue;
        };
        if a == b {
            continue;
        }
        let fixed = 3 - a - b;
        let height = p[0][0][fixed];
        if !(0..2).all(|u| {
            (0..2).all(|v| {
                p[u][v][fixed] == height && p[u][v][a] == p[u][0][a] && p[u][v][b] == p[0][v][b]
            })
        }) {
            continue;
        }
        // Retain the physical chart normal and therefore material-left winding.
        let same_sign = (p[1][0][a] > p[0][0][a]) == (p[0][1][b] > p[0][0][b]);
        let axes = if same_sign { [a, b] } else { [b, a] };
        let domains = axes.map(|axis| {
            let lo = p
                .iter()
                .flatten()
                .map(|v| v[axis])
                .fold(f64::INFINITY, f64::min);
            let hi = p
                .iter()
                .flatten()
                .map(|v| v[axis])
                .fold(f64::NEG_INFINITY, f64::max);
            [lo, hi]
        });
        let wires = std::iter::once(placed.faces[face].outer)
            .chain(placed.faces[face].holes.iter().copied())
            .collect::<Vec<_>>();
        let mut updates = Vec::new();
        let mut admitted = true;
        for &wire in &wires {
            for (slot, coedge) in placed.loops[wire].coedges.iter().enumerate() {
                let curve = &placed.edges[coedge.edge].curve;
                // No projection or epsilon admission: the entire positive-weight
                // edge hull must lie exactly in this plane and chart rectangle.
                if curve.control_points.iter().any(|v| {
                    v[fixed] != height
                        || (0..2).any(|k| v[axes[k]] < domains[k][0] || v[axes[k]] > domains[k][1])
                }) {
                    admitted = false;
                    break;
                }
                let mut uv = if coedge.reversed {
                    curve.reverse()?
                } else {
                    curve.clone()
                };
                for point in &mut uv.control_points {
                    *point = axes.map(|axis| point[axis]).to_vec();
                }
                updates.push((wire, slot, uv));
            }
            if !admitted {
                break;
            }
        }
        if !admitted {
            continue;
        }
        let mut surface = s.clone();
        for u in 0..2 {
            for v in 0..2 {
                let mut point = vec![height; 3];
                point[axes[0]] = domains[0][u];
                point[axes[1]] = domains[1][v];
                surface.control_points[u][v] = point;
            }
        }
        surface.knots_u = vec![domains[0][0], domains[0][0], domains[0][1], domains[0][1]];
        surface.knots_v = vec![domains[1][0], domains[1][0], domains[1][1], domains[1][1]];
        for (wire, slot, uv) in updates {
            placed.loops[wire].coedges[slot].pcurve = uv;
        }
        placed.faces[face].surface = surface;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const POSE: [[f64; 4]; 4] = [
        [0., -1., 0., 123.],
        [0., 0., -1., -45.],
        [1., 0., 0., 67.],
        [0., 0., 0., 1.],
    ];
    #[test]
    fn placed_charts_preserve_source_edges_vertices_and_surface_corners() {
        let source =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        for (face, f) in source.faces.iter().enumerate() {
            if let Some(axes) = identity(&f.surface) {
                assert!(
                    authored_lifts_match(&source, face, axes),
                    "source cap {face} guard"
                );
            }
        }
        let before = format!("{source:?}");
        let placed = crate::transform::affine(&source, POSE).unwrap();
        for (i, f) in source.faces.iter().enumerate() {
            if identity(&f.surface).is_some() {
                assert!(
                    identity(&placed.faces[i].surface).is_some(),
                    "placed cap {i} was skipped"
                );
            }
        }

        let point = |p: &[f64]| vec![123. - p[1], -45. - p[2], 67. + p[0]];
        assert_eq!(placed.faces.len(), source.faces.len());
        assert_eq!(placed.loops.len(), source.loops.len());
        for (a, b) in source.vertices.iter().zip(&placed.vertices) {
            assert_eq!(point(&a.point), b.point);
        }
        for (a, b) in source.edges.iter().zip(&placed.edges) {
            assert_eq!(a.curve.weights, b.curve.weights);
            for (p, q) in a.curve.control_points.iter().zip(&b.curve.control_points) {
                assert_eq!(point(p), *q);
            }
        }
        for (a, b) in source.faces.iter().zip(&placed.faces) {
            let expected = a
                .surface
                .control_points
                .iter()
                .flatten()
                .map(|p| point(p))
                .collect::<Vec<_>>();
            let actual = b
                .surface
                .control_points
                .iter()
                .flatten()
                .collect::<Vec<_>>();
            assert_eq!(expected.len(), actual.len());
            assert!(expected.iter().all(|p| actual.iter().any(|q| *q == p)));
        }
        assert_eq!(format!("{source:?}"), before);
    }
    #[test]
    fn mismatched_authored_lift_is_never_repaired_by_placement() {
        let mut source =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let face = source
            .faces
            .iter()
            .position(|f| identity(&f.surface).is_some())
            .unwrap();
        let wire = source.faces[face].outer;
        let slot = source.loops[wire]
            .coedges
            .iter()
            .position(|c| c.pcurve.control_points.len() > 2)
            .unwrap();
        let center =
            (source.faces[face].surface.knots_u[1] + source.faces[face].surface.knots_u[2]) * 0.5;
        let uv = &mut source.loops[wire].coedges[slot].pcurve;
        uv.control_points[1][0] += if uv.control_points[1][0] > center {
            -1e-8
        } else {
            1e-8
        };
        assert!(!authored_lifts_match(
            &source,
            face,
            identity(&source.faces[face].surface).unwrap()
        ));
        let placed = crate::transform::affine(&source, POSE).unwrap();
        assert_eq!(
            format!("{:?}", placed.loops[wire].coedges[slot].pcurve),
            format!("{:?}", source.loops[wire].coedges[slot].pcurve)
        );
    }
}

//! Piecewise source-normal offset with explicit straight bevel joins.
//! Wire station parameters are distinct from retained source-cell parameters.
//! No region clipping, regularity or source-offset topology claim is made.
use crate::{
    Result, check,
    curve::Curve,
    curve_offset::{Segment, approximate, set_endpoint},
};
#[derive(Debug)]
pub enum Role {
    Source([f64; 2]),
    Bevel(f64),
}
#[derive(Debug)]
pub struct Wire {
    pub closed: bool,
    pub curves: Vec<Curve>,
    pub edges: Vec<Segment>,
    pub roles: Vec<Role>,
    pub error_upper_mm: f64,
}
/// Retain a continuous curve's offset spans and canonical bevels.
/// The bevel target joins the exact one-sided normal-offset endpoints. Its
/// linear interpolation inherits the maximum of both endpoint error bounds.
pub fn bevel_wire(source: &Curve, distance: f64, tolerance: f64, max_cells: usize) -> Result<Wire> {
    source.validate()?;
    check(
        distance.is_finite() && distance != 0.,
        "Bevel wire requires a nonzero finite offset",
    )?;
    let [start, end] = source.domain();
    // Clamped equal endpoint controls prove positional closure without
    // interpreting an arbitrary nonperiodic endpoint evaluation as periodic.
    let clamped_closed = !source.periodic
        && source.knots.iter().filter(|&&t| t == start).count() == source.degree + 1
        && source.knots.iter().filter(|&&t| t == end).count() == source.degree + 1
        && source.control_points.first() == source.control_points.last();
    let closed = source.periodic || clamped_closed;
    let mut corners = Vec::new();
    let mut i = 0;
    while i < source.knots.len() {
        let knot = source.knots[i];
        let mut j = i + 1;
        while j < source.knots.len() && source.knots[j] == knot {
            j += 1;
        }
        if knot > start && knot < end {
            check(
                j - i <= source.degree,
                "Bevel wire refuses discontinuous source knots",
            )?;
            if j - i == source.degree {
                corners.push(knot);
            }
        }
        i = j;
    }
    let reserved = tolerance * 0.5;
    check(
        reserved > 0.,
        "Bevel tolerance below representable subdivision budget",
    )?;
    let source_edges = approximate(source, distance, reserved, max_cells)?;
    let mut edges: Vec<Segment> = Vec::new();
    let mut roles = Vec::new();
    for mut edge in source_edges {
        let domain = edge.domain;
        if let Some(previous) = edges.last() {
            if corners
                .binary_search_by(|t| t.total_cmp(&domain[0]))
                .is_ok()
            {
                let error_upper_mm = previous.error_upper_mm.max(edge.error_upper_mm);
                let join = Segment {
                    domain: [edges.len() as f64, edges.len() as f64 + 1.],
                    points: [previous.points[1], edge.points[0]],
                    error_upper_mm,
                };
                edges.push(join);
                roles.push(Role::Bevel(domain[0]));
            } else {
                let common = previous.points[1];
                set_endpoint(&mut edge, 0, common, tolerance)?;
            }
        }
        edge.domain = [edges.len() as f64, edges.len() as f64 + 1.];
        edges.push(edge);
        roles.push(Role::Source(domain));
    }
    if closed {
        let first = edges[0].points[0];
        let last = edges.len() - 1;
        let seam_corner =
            clamped_closed || source.knots.iter().filter(|&&t| t == start).count() >= source.degree;
        if seam_corner {
            let bound = edges[last].error_upper_mm.max(edges[0].error_upper_mm);
            edges.push(Segment {
                domain: [edges.len() as f64, edges.len() as f64 + 1.],
                points: [edges[last].points[1], first],
                error_upper_mm: bound,
            });
            roles.push(Role::Bevel(start));
        } else {
            set_endpoint(&mut edges[last], 1, first, tolerance)?;
        }
    }
    check(
        edges.len() <= max_cells,
        "Bevel wire exceeds retained edge budget",
    )?;
    let mut curves = Vec::new();
    for chunk in edges.chunks(255) {
        let mut points = vec![chunk[0].points[0].to_vec()];
        points.extend(chunk.iter().map(|e| e.points[1].to_vec()));
        let a = chunk[0].domain[0];
        let b = chunk.last().unwrap().domain[1];
        let mut knots = vec![a, a];
        knots.extend(chunk.iter().take(chunk.len() - 1).map(|e| e.domain[1]));
        knots.extend([b, b]);
        let c = Curve {
            degree: 1,
            weights: vec![1.; points.len()],
            control_points: points,
            knots,
            periodic: false,
        };
        c.validate()?;
        curves.push(c);
    }
    let error_upper_mm = edges.iter().map(|e| e.error_upper_mm).fold(0., f64::max);
    Ok(Wire {
        closed,
        curves,
        edges,
        roles,
        error_upper_mm,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corner_bevel_keeps_roles_and_connected_station_parameters() {
        let c = Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 1., 1.],
            control_points: vec![vec![0., 0.], vec![10., 0.], vec![10., 10.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        for d in [-2., 2.] {
            let w = bevel_wire(&c, d, 1e-6, 64).unwrap();
            assert_eq!(w.edges.len(), 3);
            assert!(matches!(w.roles[1], Role::Bevel(0.5)));
            for p in w.edges.windows(2) {
                assert_eq!(p[0].points[1], p[1].points[0]);
                assert_eq!(p[0].domain[1], p[1].domain[0]);
            }
            for (p, q) in w.edges[1].points.iter().zip([[10., d], [10. - d, 0.]]) {
                assert!((p[0] - q[0]).hypot(p[1] - q[1]) <= w.error_upper_mm);
            }
            assert!(w.error_upper_mm <= 1e-6);
            assert_eq!(w.curves[0].control_points.len(), 4);
        }
    }
    #[cfg(feature = "transport")]
    #[test]
    fn inner_bevel_intersection_is_visible_without_a_region_claim() {
        let c = Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 1., 1.],
            control_points: vec![vec![0., 0.], vec![10., 0.], vec![10., 10.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let result=crate::dispatch(value_codec::json!({"op":"curve_offset_bevel_wire","curve":c,"distance":2.,"toleranceMm":1e-6,"maxCells":64})).unwrap();
        assert_eq!(result["report"]["cells"][1]["source"]["kind"], "bevel");
        assert_eq!(
            result["report"]["chainDiagnostics"]["crossings"],
            value_codec::json!([[0, 2]])
        );
        assert_eq!(result["report"]["regionTrimmed"], false);
        assert_eq!(result["report"]["regionTopologyCertified"], false);
    }
    #[test]
    fn periodic_square_has_exact_closure_and_explicit_seam_bevel() {
        let c = Curve {
            degree: 1,
            knots: (0..7).map(|i| i as f64).collect(),
            control_points: vec![
                vec![0., 0.],
                vec![10., 0.],
                vec![10., 10.],
                vec![0., 10.],
                vec![0., 0.],
            ],
            weights: vec![1.; 5],
            periodic: true,
        };
        let original = c.clone();
        for distance in [-2., 2.] {
            let w = bevel_wire(&c, distance, 1e-6, 128).unwrap();
            assert!(w.closed);
            assert_eq!(w.edges.len(), 8);
            assert_eq!(w.edges[0].points[0], w.edges.last().unwrap().points[1]);
            assert!(matches!(w.roles.last(), Some(Role::Bevel(1.))));
            for pair in w.edges.windows(2) {
                assert_eq!(pair[0].points[1], pair[1].points[0]);
            }
            let d =
                crate::curve_offset_diagnostics::inspect_chain(&w.edges, w.closed, 1000).unwrap();
            assert!(d.complete);
            if distance < 0. {
                assert!(d.crossings.is_empty() && d.contacts.is_empty());
            } else {
                assert!(!d.crossings.is_empty());
            }
            assert_eq!(c.control_points, original.control_points);
            assert!(c.periodic);
        }
    }
    #[test]
    fn smooth_periodic_seam_closes_with_bounded_perturbation() {
        let c = Curve {
            degree: 2,
            knots: (0..9).map(|i| i as f64).collect(),
            control_points: vec![
                vec![1., 0.],
                vec![0., 1.],
                vec![-1., 0.],
                vec![0., -1.],
                vec![1., 0.],
                vec![0., 1.],
            ],
            weights: vec![1., 0.7, 1.1, 0.9, 1., 0.7],
            periodic: true,
        };
        let w = bevel_wire(&c, 0.2, 0.001, 4096).unwrap();
        assert!(w.closed);
        assert_eq!(w.edges[0].points[0], w.edges.last().unwrap().points[1]);
        assert!(w.roles.iter().all(|r| matches!(r, Role::Source(_))));
        assert!(w.error_upper_mm <= 0.001);
        for pair in w.curves.windows(2) {
            assert_eq!(
                pair[0].control_points.last(),
                pair[1].control_points.first()
            );
        }
    }
    #[test]
    fn clamped_closed_triangle_keeps_nonperiodic_source_and_closes_wire() {
        let c = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 2., 3., 3.],
            control_points: vec![vec![0., 0.], vec![10., 0.], vec![10., 10.], vec![0., 0.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        let w = bevel_wire(&c, -1., 1e-6, 128).unwrap();
        assert!(w.closed);
        assert_eq!(w.edges.len(), 6);
        assert_eq!(w.edges[0].points[0], w.edges.last().unwrap().points[1]);
        assert!(!c.periodic);
        assert!(matches!(w.roles.last(), Some(Role::Bevel(0.))));
    }
}

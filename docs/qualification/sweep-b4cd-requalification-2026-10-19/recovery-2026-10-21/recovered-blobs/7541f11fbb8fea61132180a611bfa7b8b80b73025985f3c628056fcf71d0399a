//! Curved cap-boundary proposals; original plane qualification is mandatory.
use super::{Options, Trace, TraceReport};
use crate::{Result, check, curve::Curve, surface::Surface};

/// Bridge an isoparametric source trace to an arbitrary rational pcurve on an
/// affine plane. A ruled chart only proposes homogeneous transverse jets; the
/// final boundary is requalified on the original plane, not on that proxy.
/// `options.boundaries[1]` must be VStart and its gain uses the supplied UV
/// transverse vector. This constructs a surface, not a closed B-rep corner.
pub fn g2_to_plane_trace(
    source: &Surface,
    trace: Trace,
    plane: &Surface,
    target: &Curve,
    transverse: [f64; 2],
    options: Options,
    tolerances: [f64; 3],
    max_cells: usize,
    max_exact: u64,
) -> Result<TraceReport> {
    plane.validate()?;
    target.validate()?;
    check(
        plane.degree_u == 1
            && plane.degree_v == 1
            && plane.control_points.len() == 2
            && plane.control_points[0].len() == 2
            && !plane.periodic_u
            && !plane.periodic_v
            && plane
                .weights
                .iter()
                .flatten()
                .all(|w| *w == plane.weights[0][0])
            && target.control_points[0].len() == 2
            && !target.periodic
            && target.control_points.len() <= 4096
            && transverse.iter().all(|x| x.is_finite())
            && transverse != [0.; 2]
            && matches!(options.boundaries[1], super::Boundary::VStart),
        "Curved cap trace needs an affine plane, bounded UV curve and nonzero transverse direction",
    )?;
    let du = plane.knots_u[2] - plane.knots_u[1];
    let dv = plane.knots_v[2] - plane.knots_v[1];
    let origin = &plane.control_points[0][0];
    let map = |p: [f64; 2]| {
        (0..3)
            .map(|k| {
                origin[k]
                    + (plane.control_points[1][0][k] - origin[k]) * (p[0] - plane.knots_u[1]) / du
                    + (plane.control_points[0][1][k] - origin[k]) * (p[1] - plane.knots_v[1]) / dv
            })
            .collect::<Vec<_>>()
    };
    let proxy = Surface {
        degree_u: target.degree,
        degree_v: 1,
        knots_u: target.knots.clone(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: target
            .control_points
            .iter()
            .map(|p| {
                vec![
                    map([p[0], p[1]]),
                    map([p[0] + transverse[0], p[1] + transverse[1]]),
                ]
            })
            .collect(),
        weights: target.weights.iter().map(|w| vec![*w; 2]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let traces = [
        trace,
        Trace {
            at: 0.,
            along: target.domain(),
        },
    ];
    let construction = super::traces::construct(
        [source, &proxy],
        traces,
        options,
        tolerances,
        max_cells,
        max_exact,
    )?;
    super::traces::qualify(
        construction,
        [source, plane],
        traces,
        options,
        tolerances,
        max_cells,
        max_exact,
        Some(target),
    )
}

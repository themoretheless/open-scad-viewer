//! Complete trim-aware exclusion forest for a Cartesian witness ball. This
//! decides separation, rather than minimizing a point-to-surface distance.
use super::*;
use nurbs_core::{
    interval_eval::{self, Interval},
    trim_domain::Location,
};
pub(super) fn exclude(
    solid: &QualifiedClosedSolid,
    face: usize,
    witness: [f64; 3],
    radius: f64,
    max_cells: usize,
    max_domain: usize,
) -> Result<[usize; 2]> {
    let surface = &solid.model.faces[face].surface;
    let region =
        crate::face_domain::FaceDomain::new(&solid.model, face, solid.policy.tolerance_uv)?;
    let mut stack = Vec::new();
    for u in surface.degree_u..surface.control_points.len() {
        if surface.knots_u[u] >= surface.knots_u[u + 1] {
            continue;
        }
        for v in surface.degree_v..surface.control_points[0].len() {
            if surface.knots_v[v] >= surface.knots_v[v + 1] {
                continue;
            }
            if stack.len() == max_cells {
                return Err(unproved(
                    "Retained separation initial span forest exceeds work",
                ));
            }
            stack.push((
                [
                    [surface.knots_u[u], surface.knots_u[u + 1]],
                    [surface.knots_v[v], surface.knots_v[v + 1]],
                ],
                0usize,
            ));
        }
    }
    if stack.is_empty() {
        return Err(unproved("Retained separation has no original span roots"));
    }
    let mut cells = 0;
    let mut domains = 0;
    while let Some((d, depth)) = stack.pop() {
        if cells == max_cells {
            return Err(unproved("Retained separation forest work exhausted"));
        }
        cells += 1;
        let b = interval_eval::evaluate_surface_interval(
            surface,
            Interval::new(d[0][0], d[0][1])?,
            Interval::new(d[1][0], d[1][1])?,
        )?;
        let bounds: [[f64; 2]; 3] = std::array::from_fn(|k| [b[k].lo, b[k].hi]);
        if surface_distance::enclosure_distance(&bounds, &witness.map(|x| [x; 2]))?.0 > radius {
            continue;
        }
        if domains < max_domain {
            let classified = region.classify(d, (max_domain - domains).min(100000))?;
            domains += classified.cells;
            if classified.location == Location::Outside {
                continue;
            }
        }
        let widths = d.map(|r| r[1] - r[0]);
        let axis = usize::from(widths[1] > widths[0]);
        let midpoint = d[axis][0] * 0.5 + d[axis][1] * 0.5;
        if depth == 64 || !(d[axis][0] < midpoint && midpoint < d[axis][1]) {
            return Err(unproved(
                "Original retained ball contact or separation precision remains unresolved",
            ));
        }
        let mut a = d;
        let mut b = d;
        a[axis][1] = midpoint;
        b[axis][0] = midpoint;
        stack.push((b, depth + 1));
        stack.push((a, depth + 1));
    }
    Ok([cells, domains])
}

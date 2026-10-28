//! Conservative containment of a trimmed surface in an authored XYZ box.
//! This is not general B-rep material containment, embedding or solid admission.
use super::FaceDomain;
use crate::Model;
use nurbs_core::{
    Result,
    interval_eval::{self, Interval as I},
    trim_domain::Location,
};
fn check(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(nurbs_core::Error::new("BREP_FACE_BOX_UNPROVEN", message))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoxDecision {
    Within,
    Outside,
    Unresolved,
}
pub struct BoxContainment {
    pub decision: BoxDecision,
    pub cells: usize,
    pub domain_cells: usize,
    pub covered_cells: usize,
    pub omitted_cells: usize,
    pub witness_uv: Option<[f64; 2]>,
    pub witness_bounds: Option<[[f64; 2]; 3]>,
}
fn enclose(s: &nurbs_core::surface::Surface, r: [[f64; 2]; 2]) -> Result<[[f64; 2]; 3]> {
    let b = interval_eval::evaluate_surface_interval(
        s,
        I::new(r[0][0], r[0][1])?,
        I::new(r[1][0], r[1][1])?,
    )?;
    Ok(std::array::from_fn(|k| [b[k].lo, b[k].hi]))
}
fn contained(b: [[f64; 2]; 3], target: [[f64; 2]; 3]) -> bool {
    (0..3).all(|k| b[k][0] >= target[k][0] && b[k][1] <= target[k][1])
}
fn disjoint(b: [[f64; 2]; 3], target: [[f64; 2]; 3]) -> bool {
    (0..3).any(|k| b[k][1] < target[k][0] || b[k][0] > target[k][1])
}
/// Cover every natural-domain knot cell. A cell is discarded only after a
/// conservative complete UV outside classification; a midpoint never grants
/// whole-cell containment. An outside witness has both a certified world
/// enclosure and a fresh Inside classification on the actual trimmed region.
/// All limits are explicit. Exhaustion is unresolved, never successful.
pub fn inspect_box_containment(
    model: &Model,
    face: usize,
    target: [[f64; 2]; 3],
    tolerance_uv: f64,
    max_cells: usize,
    max_domain_cells: usize,
) -> Result<BoxContainment> {
    check(
        max_cells <= 50000
            && max_domain_cells <= 1000000
            && target
                .iter()
                .all(|b| b[0].is_finite() && b[1].is_finite() && b[0] <= b[1]),
        "Trimmed box inspection needs finite bounds and explicit bounded work",
    )?;
    let domain = FaceDomain::new(model, face, tolerance_uv)?;
    let s = &model.faces[face].surface;
    check(
        s.control_points[0][0].len() == 3
            && s.control_points
                .len()
                .saturating_mul(s.control_points[0].len())
                <= 4096,
        "Trimmed box inspection needs a bounded 3D support",
    )?;
    check(
        !s.periodic_u && !s.periodic_v && s.degree_u <= 16 && s.degree_v <= 16,
        "Trimmed box inspection needs a bounded nonperiodic support",
    )?;
    let natural = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    for wire in
        std::iter::once(model.faces[face].outer).chain(model.faces[face].holes.iter().copied())
    {
        let curves = model.loops[wire]
            .coedges
            .iter()
            .map(|c| c.pcurve.clone())
            .collect::<Vec<_>>();
        check(
            nurbs_core::trim_domain::exact_loop_joins(&curves)? == Some(true),
            "Trimmed box inspection needs original closed UV loops",
        )?;
        check(
            curves.iter().all(|c| {
                c.control_points
                    .iter()
                    .all(|p| (0..2).all(|k| p[k] >= natural[k][0] && p[k] <= natural[k][1]))
            }),
            "UV loop containment in the natural support is unproven",
        )?;
    }
    let intervals = |knots: &[f64], degree: usize, n: usize| {
        (degree..n)
            .filter(|&i| knots[i] < knots[i + 1])
            .map(|i| [knots[i], knots[i + 1]])
            .collect::<Vec<_>>()
    };
    let mut stack = Vec::new();
    for u in intervals(&s.knots_u, s.degree_u, s.control_points.len()) {
        for v in intervals(&s.knots_v, s.degree_v, s.control_points[0].len()) {
            stack.push([u, v]);
        }
    }
    let mut out = BoxContainment {
        decision: BoxDecision::Unresolved,
        cells: 0,
        domain_cells: 0,
        covered_cells: 0,
        omitted_cells: 0,
        witness_uv: None,
        witness_bounds: None,
    };
    while let Some(rectangle) = stack.pop() {
        if out.cells == max_cells {
            return Ok(out);
        }
        out.cells += 1;
        let bounds = enclose(s, rectangle)?;
        if contained(bounds, target) {
            out.covered_cells += 1;
            continue;
        }
        let remaining = max_domain_cells.saturating_sub(out.domain_cells);
        if remaining == 0 {
            return Ok(out);
        }
        let membership = domain.classify(rectangle, remaining.min(512))?;
        out.domain_cells += membership.cells;
        if membership.location == Location::Outside {
            out.omitted_cells += 1;
            continue;
        }
        let middle = rectangle.map(|r| r[0] * 0.5 + r[1] * 0.5);
        if membership.location == Location::Inside && disjoint(bounds, target) {
            out.decision = BoxDecision::Outside;
            out.witness_uv = Some(middle);
            out.witness_bounds = Some(bounds);
            return Ok(out);
        }
        let point = middle.map(|x| [x, x]);
        let point_bounds = enclose(s, point)?;
        if disjoint(point_bounds, target) {
            let remaining = max_domain_cells.saturating_sub(out.domain_cells);
            if remaining == 0 {
                return Ok(out);
            }
            let witness = domain.classify(point, remaining.min(512))?;
            out.domain_cells += witness.cells;
            if witness.location == Location::Inside {
                out.decision = BoxDecision::Outside;
                out.witness_uv = Some(middle);
                out.witness_bounds = Some(point_bounds);
                return Ok(out);
            }
        }
        let axis =
            usize::from(rectangle[1][1] - rectangle[1][0] > rectangle[0][1] - rectangle[0][0]);
        if middle[axis] <= rectangle[axis][0] || middle[axis] >= rectangle[axis][1] {
            return Ok(out);
        }
        let mut a = rectangle;
        let mut b = rectangle;
        a[axis][1] = middle[axis];
        b[axis][0] = middle[axis];
        stack.push(b);
        stack.push(a);
    }
    out.decision = BoxDecision::Within;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_surface_bounds_admit_padded_cube_face() {
        let m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let r = inspect_box_containment(&m, 0, [[-1e-6, 1. + 1e-6]; 3], 1e-8, 1000, 10000).unwrap();
        assert_eq!(r.decision, BoxDecision::Within);
        assert_eq!(r.covered_cells, 1);
    }
    #[test]
    fn interior_witness_rejects_smaller_world_box() {
        let m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let r = inspect_box_containment(
            &m,
            0,
            [[-1e-6, 0.4], [-2., 2.], [-2., 2.]],
            1e-8,
            1000,
            10000,
        )
        .unwrap();
        assert_eq!(r.decision, BoxDecision::Outside);
        assert!(r.witness_uv.is_some());
        assert!(r.witness_bounds.unwrap()[0][0] > 0.4);
    }
    #[test]
    fn zero_cover_budget_is_unresolved() {
        let m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let r = inspect_box_containment(&m, 0, [[-1e-6, 1. + 1e-6]; 3], 1e-8, 0, 10000).unwrap();
        assert_eq!(r.decision, BoxDecision::Unresolved);
        assert_eq!(r.cells, 0);
    }
    #[test]
    fn an_open_trim_cannot_use_the_whole_surface_shortcut() {
        let mut m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let wire = m.faces[0].outer;
        m.loops[wire].coedges[0].pcurve.control_points[0][0] += 0.001;
        assert!(
            inspect_box_containment(&m, 0, [[-1e-6, 1. + 1e-6]; 3], 1e-8, 1000, 10000).is_err()
        );
    }
}

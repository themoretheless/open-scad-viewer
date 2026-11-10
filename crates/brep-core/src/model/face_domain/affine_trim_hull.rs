//! Complete filled-region enclosure under an exact affine support chart.
//! Every bounded winding component lies in the convex hull of its UV boundary;
//! an affine map commutes with that hull and with positive rational curves.
use crate::Model;
use nurbs_core::{
    Result,
    interval_eval::{self, Interval as I},
};
fn sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let v = s - a;
    (s, (a - (s - v)) + (b - v))
}
fn guarded(x: f64) -> bool {
    x.is_finite() && (x == 0. || (1e-100..=1e100).contains(&x.abs()))
}
pub(crate) fn affine_support(s: &nurbs_core::surface::Surface) -> bool {
    if s.degree_u != 1
        || s.degree_v != 1
        || s.control_points.len() != 2
        || s.control_points.iter().any(|r| r.len() != 2)
        || s.knots_u.len() != 4
        || s.knots_v.len() != 4
        || [&s.knots_u, &s.knots_v]
            .iter()
            .any(|k| k[0] != k[1] || k[2] != k[3] || k[1] >= k[2])
        || s.weights
            .iter()
            .flatten()
            .any(|w| !w.is_finite() || *w <= 0. || *w != s.weights[0][0])
        || s.control_points
            .iter()
            .flatten()
            .any(|p| p.len() != 3 || p.iter().any(|x| !guarded(*x)))
    {
        return false;
    }
    (0..3).all(|k| {
        sum(s.control_points[0][0][k], s.control_points[1][1][k])
            == sum(s.control_points[1][0][k], s.control_points[0][1][k])
    })
}
pub(super) struct Attempt {
    pub within: bool,
    pub cells: usize,
}
/// Caller has checked original UV loop closure and containment in the support.
/// Failed hull enclosure is not an outside witness. All spent field cells are
/// charged even when the regular inspector must continue afterwards.
pub(super) fn inspect(
    model: &Model,
    face: usize,
    target: [[f64; 2]; 3],
    max_cells: usize,
) -> Result<Attempt> {
    let f = &model.faces[face];
    let s = &f.surface;
    let mut out = Attempt {
        within: false,
        cells: 0,
    };
    if !affine_support(s) {
        return Ok(out);
    }
    let curves = std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .flat_map(|w| model.loops[w].coedges.iter().map(|c| &c.pcurve))
        .collect::<Vec<_>>();
    let points = curves.iter().map(|c| c.control_points.len()).sum::<usize>();
    if points == 0
        || points > max_cells
        || curves
            .iter()
            .any(|c| c.weights.iter().any(|w| !w.is_finite() || *w <= 0.))
    {
        return Ok(out);
    }
    for c in curves {
        for p in &c.control_points {
            out.cells += 1;
            let b = interval_eval::evaluate_surface_interval(s, I::point(p[0]), I::point(p[1]))?;
            if !(0..3).all(|k| b[k].lo >= target[k][0] && b[k].hi <= target[k][1]) {
                return Ok(out);
            }
        }
    }
    out.within = true;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_nonaffine_or_one_ulp_mixed_pole_cannot_inherit_the_hull() {
        let m = crate::cuboid([0.; 3], [4.; 3]).unwrap();
        let s = &m.faces[0].surface;
        assert!(affine_support(s));
        let mut changed = s.clone();
        changed.control_points[1][1][0] =
            f64::from_bits(changed.control_points[1][1][0].to_bits() + 1);
        assert!(!affine_support(&changed));
        changed = s.clone();
        changed.weights[1][1] = 2.;
        assert!(!affine_support(&changed));
    }
}

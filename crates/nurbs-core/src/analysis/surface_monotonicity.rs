//! Sufficient whole-rectangle injectivity via a strictly monotone coordinate
//! projection. Positive-definite symmetric projected Jacobians imply that
//! distinct parameters have distinct projected (and hence spatial) points.
use crate::{
    Result, check, distance_bounds::Interval as I, surface::Surface, surface_measure::jets,
};
#[derive(Clone, Debug)]
pub struct Report {
    pub certified: bool,
    pub cells: usize,
    pub projection: Option<([usize; 2], [i8; 2])>,
    pub reason: Option<&'static str>,
}
pub fn inspect(s: &Surface, max_cells: usize) -> Result<Report> {
    s.validate()?;
    check(
        max_cells <= 100000 && s.control_points.iter().flatten().all(|p| p.len() == 3),
        "Invalid injectivity budget/dimension",
    )?;
    let mut out = Report {
        certified: false,
        cells: 0,
        projection: None,
        reason: Some("monotone-projection-unproved"),
    };
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let degrees = [s.degree_u, s.degree_v];
    let knots = [&s.knots_u, &s.knots_v];
    if s.periodic_u
        || s.periodic_v
        || (0..2).any(|k| {
            let a = knots[k][degrees[k]];
            let b = knots[k][counts[k]];
            knots[k]
                .iter()
                .filter(|v| **v > a && **v < b)
                .any(|v| knots[k].iter().filter(|x| *x == v).count() > degrees[k])
        })
    {
        out.reason = Some("continuous-rectangle-domain-unproved");
        return Ok(out);
    }
    let mut grid = Vec::new();
    for u in degrees[0]..counts[0] {
        for v in degrees[1]..counts[1] {
            if knots[0][u] < knots[0][u + 1] && knots[1][v] < knots[1][v + 1] {
                // Every original rectangle must be visited for any policy.
                // Bound allocation as well as evaluation before collecting it.
                if grid.len() == max_cells {
                    out.reason = Some("cell-budget-exhausted");
                    return Ok(out);
                }
                grid.push((
                    [u, v],
                    [
                        [knots[0][u], knots[0][u + 1]],
                        [knots[1][v], knots[1][v + 1]],
                    ],
                ));
            }
        }
    }
    let mut numerical_failure = false;
    let mut subdivision_limit = false;
    let mut unsplittable = false;
    for a in 0..3 {
        for b in 0..3 {
            if a == b {
                continue;
            }
            // An exactly constant represented coordinate has identically zero
            // derivative for every positive rational basis combination. Such a
            // projection cannot be strictly monotone; do not subdivide interval
            // rounding around zero before considering a useful projection.
            if [a,b].iter().any(|axis| {
                let value = s.control_points[0][0][*axis];
                s.control_points.iter().flatten().all(|point| point[*axis] == value)
            }) {
                continue;
            }
            for signs in [[1, 1], [1, -1], [-1, 1], [-1, -1]] {
                let mut passed = true;
                let mut pending: Vec<_> = grid
                    .iter()
                    .map(|(span, domain)| (*span, *domain, 0usize))
                    .collect();
                while let Some((span, domain, depth)) = pending.pop() {
                    if out.cells == max_cells {
                        out.reason = Some("cell-budget-exhausted");
                        return Ok(out);
                    }
                    out.cells += 1;
                    let result = (|| -> Result<i8> {
                        let j = jets::calculate(s, span, domain, None)?;
                        let width = |k: usize| I::point(domain[k][1]).sub(I::point(domain[k][0]));
                        let uu = j[1][0][a].div(width(0)?)?.mul(I::point(signs[0] as f64))?;
                        let uv = j[0][1][a].div(width(1)?)?.mul(I::point(signs[0] as f64))?;
                        let vu = j[1][0][b].div(width(0)?)?.mul(I::point(signs[1] as f64))?;
                        let vv = j[0][1][b].div(width(1)?)?.mul(I::point(signs[1] as f64))?;
                        let off = uv.add(vu)?.mul(I::point(0.5))?;
                        let radius = I::point(off.lo.abs().max(off.hi.abs()));
                        if uu.hi <= 0. || vv.hi <= 0. {
                            return Ok(-1);
                        }
                        Ok(i8::from(
                            uu.lo > 0.
                                && vv.lo > 0.
                                && uu.mul(vv)?.sub(radius.mul(radius)?)?.lo > 0.,
                        ))
                    })();
                    match result {
                        Ok(1) => (),
                        Ok(0) if depth < 12 => {
                            let axis = depth % 2;
                            let middle = domain[axis][0] * 0.5 + domain[axis][1] * 0.5;
                            if !(domain[axis][0] < middle && middle < domain[axis][1]) {
                                unsplittable = true;
                                passed = false;
                                break;
                            }
                            let mut left = domain;
                            let mut right = domain;
                            left[axis][1] = middle;
                            right[axis][0] = middle;
                            pending.push((span, right, depth + 1));
                            pending.push((span, left, depth + 1));
                        }
                        other => {
                            numerical_failure |= other.is_err();
                            subdivision_limit |= matches!(other, Ok(0));
                            passed = false;
                            break;
                        }
                    }
                }
                if passed && !grid.is_empty() {
                    out.certified = true;
                    out.projection = Some(([a, b], signs));
                    out.reason = None;
                    return Ok(out);
                }
            }
        }
    }
    out.reason = Some(if numerical_failure {
        "numerical-enclosure-unresolved"
    } else if unsplittable {
        "parameter-subdivision-unresolved"
    } else if subdivision_limit {
        "subdivision-depth-exhausted"
    } else {
        "monotone-projection-unproved"
    });
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn patch() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![3., 3., 7., 7.],
            knots_v: vec![-2., -2., 2., 2.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 1.]],
                vec![vec![1., 0., 1.], vec![1., 1., 2.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn oblique_patch_is_injective_and_exhaustion_is_unproved() {
        let s = patch();
        let r = inspect(&s, 1).unwrap();
        assert!(r.certified && r.projection.is_some() && r.cells == 1);
        let r = inspect(&s, 0).unwrap();
        assert!(!r.certified && r.projection.is_none());
        let mut s = patch();
        s.control_points[1] = s.control_points[0].clone();
        assert!(!inspect(&s, 24).unwrap().certified);
    }
    #[test]
    fn constant_coordinate_does_not_spend_budget_on_impossible_projections() {
        let mut s = patch();
        s.control_points = vec![
            vec![vec![3.,0.,0.],vec![3.,0.,1.]],
            vec![vec![3.,1.,0.],vec![3.,1.,1.]],
        ];
        let report = inspect(&s,1).unwrap();
        assert!(report.certified);
        assert_eq!(report.cells,1);
        assert_eq!(report.projection,Some(([1,2],[1,1])));
        assert!(!inspect(&s,0).unwrap().certified);
    }
    #[test]
    fn rational_reversed_projection_is_certified() {
        let mut s = patch();
        for row in &mut s.control_points {
            for point in row {
                point[0] = -point[0];
                point[1] = -point[1];
            }
        }
        s.weights = vec![vec![1., 1.001], vec![1.001, 1.]];
        let r = inspect(&s, 24).unwrap();
        assert!(r.certified, "{r:?}");
        assert_eq!(r.projection, Some(([0, 1], [-1, -1])));
    }
    #[test]
    fn positive_rational_reparameterization_needs_bounded_refinement() {
        let mut s = patch();
        // Tensor-product positive weights give independent strictly increasing
        // rational reparameterizations of the two planar coordinates.
        s.weights = vec![vec![1., 2.], vec![2., 4.]];
        let coarse = inspect(&s, 1).unwrap();
        assert!(!coarse.certified && coarse.projection.is_none());
        let refined = inspect(&s, 10000).unwrap();
        assert!(refined.certified, "{refined:?}");
        assert!(refined.cells > 1 && refined.cells <= 10000);
    }
    #[test]
    fn locally_regular_fold_cannot_use_different_projections_per_span() {
        let mut s = patch();
        s.knots_u = vec![3., 3., 5., 7., 7.];
        s.control_points.push(s.control_points[0].clone());
        s.weights.push(vec![1.; 2]);
        // Each affine half is regular, but the two parameter endpoints coincide.
        // A policy chosen separately per span would incorrectly accept the fold.
        let r = inspect(&s, 100).unwrap();
        assert!(!r.certified && r.projection.is_none(), "{r:?}");
        let r = inspect(&s, 1).unwrap();
        assert!(!r.certified && r.reason == Some("cell-budget-exhausted"));
    }
}

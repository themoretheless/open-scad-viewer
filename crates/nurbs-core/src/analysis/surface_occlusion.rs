//! Uniform orthographic curve/surface ray coverage. All decisions enclose the
//! original rational charts; display triangles and point sampling are unused.
use crate::{
    Result, check, curve::Curve, distance_bounds::Interval as I, surface::Surface,
    surface_measure::jets,
};

pub type Rectangle = [[f64; 2]; 2];
#[derive(Debug)]
pub struct Report {
    /// For every point of the complete curve span, a surface point exists in
    /// each listed UV box with identical XY and strictly greater Z.
    pub covers: Vec<Rectangle>,
    /// Cells which may still occlude. A caller may exclude them using trims;
    /// otherwise it must refine the curve or explicitly retain uncertainty.
    pub unresolved: Vec<Rectangle>,
    pub cells: usize,
    pub budget_exhausted: bool,
}
fn bounds(c: &Curve) -> [I; 3] {
    std::array::from_fn(|k| I {
        lo: c
            .control_points
            .iter()
            .map(|p| p[k])
            .fold(f64::INFINITY, f64::min),
        hi: c
            .control_points
            .iter()
            .map(|p| p[k])
            .fold(f64::NEG_INFINITY, f64::max),
    })
}
fn absolute(a: I) -> f64 {
    a.lo.abs().max(a.hi.abs())
}
/// Parametric Krawczyk inclusion: the XY right-hand side ranges over the
/// complete curve hull. Strict inclusion and contraction establish a root for
/// EVERY right-hand side, not merely for the curve midpoint.
fn uniform_root(
    s: &Surface,
    span: [usize; 2],
    uv: Rectangle,
    target: [I; 3],
    j: jets::FirstJets,
) -> Result<Option<Rectangle>> {
    let midpoint = uv.map(|d| d[0] * 0.5 + d[1] * 0.5);
    let center = jets::calculate_first_stable(s, span, midpoint.map(|t| [t, t]))?;
    let matrix: [[f64; 2]; 2] = std::array::from_fn(|k| {
        std::array::from_fn(|axis| {
            let v = center[axis + 1][k];
            v.lo * 0.5 + v.hi * 0.5
        })
    });
    let determinant = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
    if !determinant.is_finite() || determinant == 0. {
        return Ok(None);
    }
    let inverse = [
        [matrix[1][1] / determinant, -matrix[0][1] / determinant],
        [-matrix[1][0] / determinant, matrix[0][0] / determinant],
    ];
    if inverse.iter().flatten().any(|v| !v.is_finite()) {
        return Ok(None);
    }
    let mut result = uv;
    for i in 0..2 {
        let mut value = I::point(midpoint[i]);
        for k in 0..2 {
            value = value.sub(I::point(inverse[i][k]).mul(center[0][k].sub(target[k])?)?)?;
        }
        let mut contraction = I::point(0.);
        for axis in 0..2 {
            let mut coefficient = I::point(if axis == i { 1. } else { 0. });
            for k in 0..2 {
                coefficient = coefficient.sub(I::point(inverse[i][k]).mul(j[axis + 1][k])?)?;
            }
            contraction = contraction.add(I::point(absolute(coefficient)))?;
            value = value.add(
                coefficient
                    .mul(I::new(uv[axis][0], uv[axis][1])?.sub(I::point(midpoint[axis]))?)?,
            )?;
        }
        if contraction.hi >= 1. || value.lo <= uv[i][0] || value.hi >= uv[i][1] {
            return Ok(None);
        }
        result[i] = [value.lo, value.hi];
    }
    Ok(Some(result))
}
/// Full-multiplicity knot boundaries are exact retained control rows/columns.
/// When one projected coordinate is constant on that entire row, the remaining
/// coordinate is a one-dimensional uniform root problem. This handles chart
/// seams without perturbing a ray off its authored drawing coordinate.
fn boundary_cover(
    s: &Surface,
    span: [usize; 2],
    uv: Rectangle,
    target: [I; 3],
) -> Result<Option<Rectangle>> {
    for fixed in 0..2 {
        let degree = if fixed == 0 { s.degree_u } else { s.degree_v };
        let knots = if fixed == 0 { &s.knots_u } else { &s.knots_v };
        let free = 1 - fixed;
        for (parameter, index) in [
            (knots[span[fixed]], span[fixed] - degree),
            (knots[span[fixed] + 1], span[fixed]),
        ] {
            if parameter < uv[fixed][0]
                || parameter > uv[fixed][1]
                || knots.iter().filter(|t| **t == parameter).count() < degree
            {
                continue;
            }
            for coordinate in 0..2 {
                if target[coordinate].lo != target[coordinate].hi {
                    continue;
                }
                let row: Vec<_> = if fixed == 0 {
                    s.control_points[index].iter().collect()
                } else {
                    s.control_points.iter().map(|r| &r[index]).collect()
                };
                if !row.iter().all(|p| p[coordinate] == target[coordinate].lo) {
                    continue;
                }
                let remaining = 1 - coordinate;
                let mid = uv[free][0] * 0.5 + uv[free][1] * 0.5;
                let mut domain = uv;
                domain[fixed] = [parameter, parameter];
                let jet = jets::calculate_first_stable(s, span, domain)?;
                let mut center_domain = domain;
                center_domain[free] = [mid, mid];
                let center = jets::calculate_first_stable(s, span, center_domain)?;
                let derivative = center[free + 1][remaining];
                let approximate = derivative.lo * 0.5 + derivative.hi * 0.5;
                if approximate == 0. || !approximate.is_finite() {
                    continue;
                }
                let inverse = 1. / approximate;
                if !inverse.is_finite() {
                    continue;
                }
                let contraction =
                    I::point(1.).sub(I::point(inverse).mul(jet[free + 1][remaining])?)?;
                if absolute(contraction) >= 1. {
                    continue;
                }
                let root = I::point(mid)
                    .sub(I::point(inverse).mul(center[0][remaining].sub(target[remaining])?)?)?
                    .add(contraction.mul(I::new(uv[free][0], uv[free][1])?.sub(I::point(mid))?)?)?;
                if root.lo <= uv[free][0] || root.hi >= uv[free][1] {
                    continue;
                }
                domain[free] = [root.lo, root.hi];
                if jets::calculate_first_stable(s, span, domain)?[0][2]
                    .sub(target[2])?
                    .lo
                    > 0.
                {
                    return Ok(Some(domain));
                }
            }
        }
    }
    Ok(None)
}
/// A sufficient closed-rectangle cover for rational extrusion charts. One
/// paper coordinate depends only on the nonlinear parameter; the other is
/// affine in the extrusion parameter. Endpoint bracketing supplies existence
/// even on a chart seam. No absence/visibility conclusion follows from None.
pub fn extrusion_cover(
    s: &Surface,
    c: &Curve,
    tolerance: f64,
    max_spans: usize,
) -> Result<Option<Rectangle>> {
    s.validate()?;
    c.validate()?;
    check(
        s.control_points.iter().flatten().all(|p| p.len() == 3)
            && c.control_points.iter().all(|p| p.len() == 3)
            && c.control_points.len() == c.degree + 1
            && !c.periodic
            && c.knots[..=c.degree].iter().all(|t| *t == c.domain()[0])
            && c.knots[c.degree + 1..].iter().all(|t| *t == c.domain()[1])
            && tolerance.is_finite()
            && tolerance > 0.
            && (1..=4096).contains(&max_spans),
        "Invalid extrusion coverage request",
    )?;
    let target = bounds(c);
    for free in 0..2 {
        let nonlinear = 1 - free;
        let degrees = [s.degree_u, s.degree_v];
        let counts = [s.control_points.len(), s.control_points[0].len()];
        let knots = [&s.knots_u, &s.knots_v];
        if degrees[free] != 1 || counts[free] != 2 {
            continue;
        }
        let spans: Vec<_> = (degrees[nonlinear]..counts[nonlinear])
            .filter(|i| knots[nonlinear][*i] < knots[nonlinear][*i + 1])
            .collect();
        if spans.len() > max_spans {
            continue;
        }
        let point = |i: usize, j: usize| {
            if free == 1 {
                &s.control_points[i][j]
            } else {
                &s.control_points[j][i]
            }
        };
        let weight = |i: usize, j: usize| {
            if free == 1 {
                s.weights[i][j]
            } else {
                s.weights[j][i]
            }
        };
        for span in spans {
            let first = span - degrees[nonlinear];
            let last = span;
            let ends = [knots[nonlinear][span], knots[nonlinear][span + 1]];
            if ends
                .iter()
                .any(|t| knots[nonlinear].iter().filter(|q| **q == *t).count() < degrees[nonlinear])
                || !(first..=last).all(|i| weight(i, 0) == weight(i, 1))
            {
                continue;
            }
            let minimum_z = (first..=last)
                .flat_map(|i| [point(i, 0)[2], point(i, 1)[2]])
                .fold(f64::INFINITY, f64::min);
            if I::point(minimum_z).sub(target[2])?.lo <= tolerance {
                continue;
            }
            for coordinate in 0..2 {
                let other = 1 - coordinate;
                if !(first..=last).all(|i| {
                    point(i, 0)[coordinate] == point(i, 1)[coordinate]
                        && point(i, 0)[other] == point(first, 0)[other]
                        && point(i, 1)[other] == point(first, 1)[other]
                }) {
                    continue;
                }
                let x = [point(first, 0)[coordinate], point(last, 0)[coordinate]];
                let y = [point(first, 0)[other], point(first, 1)[other]];
                if x[0] == x[1]
                    || y[0] == y[1]
                    || target[coordinate].lo < x[0].min(x[1])
                    || target[coordinate].hi > x[0].max(x[1])
                    || target[other].lo < y[0].min(y[1])
                    || target[other].hi > y[0].max(y[1])
                {
                    continue;
                }
                let free_domain = [knots[free][1], knots[free][2]];
                let normalized = target[other]
                    .sub(I::point(y[0]))?
                    .div_signed(I::point(y[1]).sub(I::point(y[0]))?)?;
                let parameters = I::point(free_domain[0])
                    .add(normalized.mul(I::point(free_domain[1]).sub(I::point(free_domain[0]))?)?)?
                    .intersect(free_domain[0], free_domain[1])?;
                let mut cover = [[0.; 2]; 2];
                cover[nonlinear] = ends;
                cover[free] = [parameters.lo, parameters.hi];
                return Ok(Some(cover));
            }
        }
    }
    Ok(None)
}
/// Coordinates must already be in the chosen drawing frame: XY on paper,
/// positive Z toward the viewer. The curve must be one positive-weight,
/// clamped Bézier span so its original controls enclose the complete image.
pub fn inspect(s: &Surface, c: &Curve, tolerance_mm: f64, max_cells: usize) -> Result<Report> {
    s.validate()?;
    c.validate()?;
    check(
        s.control_points.iter().flatten().all(|p| p.len() == 3)
            && c.control_points.iter().all(|p| p.len() == 3)
            && c.control_points.len() == c.degree + 1
            && !c.periodic
            && c.knots[..=c.degree].iter().all(|t| *t == c.domain()[0])
            && c.knots[c.degree + 1..].iter().all(|t| *t == c.domain()[1])
            && tolerance_mm.is_finite()
            && tolerance_mm > 0.
            && (1..=100000).contains(&max_cells),
        "Surface occlusion requires a 3D clamped curve, positive tolerance and 1..100000 cells",
    )?;
    let target = bounds(c);
    let mut pending = Vec::new();
    for u in s.degree_u..s.control_points.len() {
        for v in s.degree_v..s.control_points[0].len() {
            if s.knots_u[u] < s.knots_u[u + 1] && s.knots_v[v] < s.knots_v[v + 1] {
                pending.push((
                    [u, v],
                    [
                        [s.knots_u[u], s.knots_u[u + 1]],
                        [s.knots_v[v], s.knots_v[v + 1]],
                    ],
                    0usize,
                ));
            }
        }
    }
    check(
        pending.len() <= 4096,
        "Surface occlusion exceeds 4096 original knot rectangles",
    )?;
    let mut result = Report {
        covers: Vec::new(),
        unresolved: Vec::new(),
        cells: 0,
        budget_exhausted: false,
    };
    while let Some((span, uv, depth)) = pending.pop() {
        if result.cells == max_cells {
            result.budget_exhausted = true;
            result.unresolved.push(uv);
            result
                .unresolved
                .extend(pending.into_iter().map(|(_, uv, _)| uv));
            break;
        }
        result.cells += 1;
        let j = jets::calculate_first_stable(s, span, uv)?;
        if (0..2).any(|k| j[0][k].hi < target[k].lo || target[k].hi < j[0][k].lo)
            || j[0][2].sub(target[2])?.hi <= tolerance_mm
        {
            continue;
        }
        if let Some(root) = boundary_cover(s, span, uv, target)? {
            if jets::calculate_first_stable(s, span, root)?[0][2]
                .sub(target[2])?
                .lo
                > tolerance_mm
            {
                result.covers.push(root);
                // The boundary root proves coverage, but does not exclude other
                // roots of this cell or the unvisited charts. Preserve them for
                // callers whose UV trims exclude this particular cover.
                result.unresolved.push(uv);
                result
                    .unresolved
                    .extend(pending.into_iter().map(|(_, uv, _)| uv));
                break;
            }
        }
        if let Some(root) = uniform_root(s, span, uv, target, j)? {
            let z = jets::calculate_first_stable(s, span, root)?[0][2];
            let gap = z.sub(target[2])?;
            if gap.lo > tolerance_mm {
                result.covers.push(root);
            } else if gap.hi > tolerance_mm {
                result.unresolved.push(root);
            }
            continue;
        }
        let axis = usize::from(
            (uv[1][1] - uv[1][0]) / (s.knots_v[span[1] + 1] - s.knots_v[span[1]])
                > (uv[0][1] - uv[0][0]) / (s.knots_u[span[0] + 1] - s.knots_u[span[0]]),
        );
        let mid = uv[axis][0] * 0.5 + uv[axis][1] * 0.5;
        if depth == 24 || mid <= uv[axis][0] || mid >= uv[axis][1] {
            result.unresolved.push(uv);
            continue;
        }
        let mut left = uv;
        let mut right = uv;
        left[axis][1] = mid;
        right[axis][0] = mid;
        pending.push((span, right, depth + 1));
        pending.push((span, left, depth + 1));
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn arch() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![0.5, 0., 3.], vec![0.5, 1., 3.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn rational_extrusion_cover_includes_closed_chart_seams_without_a_false_absence_claim() {
        let mut surface = arch();
        surface.weights[1] = vec![0.7; 2];
        let span = Curve::from_polyline(vec![vec![0., 0., -1.], vec![0.5, 1., -1.]]).unwrap();
        let cover = extrusion_cover(&surface, &span, 1e-6, 64).unwrap().unwrap();
        assert_eq!(cover, [[0., 1.], [0., 1.]]);
        let outside = Curve::from_polyline(vec![vec![-1., 0., -1.], vec![2., 1., -1.]]).unwrap();
        assert!(
            extrusion_cover(&surface, &outside, 1e-6, 64)
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn exact_knot_boundary_cover_preserves_the_authored_ray_coordinate() {
        let surface = arch();
        let ray_span =
            Curve::from_polyline(vec![vec![0., 0.25, -1.], vec![0., 0.75, -1.]]).unwrap();
        let report = inspect(&surface, &ray_span, 1e-6, 128).unwrap();
        assert!(
            report
                .covers
                .iter()
                .any(|uv| uv[0] == [0., 0.] && uv[1][0] < 0.25 && uv[1][1] > 0.75)
        );
        // The boundary cover does not assert all other charts were excluded.
        assert!(!report.unresolved.is_empty());
    }
    #[test]
    fn curved_chart_covers_entire_span_and_excludes_front_or_outside() {
        let surface = arch();
        let behind = Curve::from_polyline(vec![vec![0.4, 0.4, -1.], vec![0.6, 0.6, -1.]]).unwrap();
        let report = inspect(&surface, &behind, 1e-6, 4096).unwrap();
        assert!(!report.covers.is_empty());
        for points in [
            vec![vec![0.4, 0.4, 5.], vec![0.6, 0.6, 5.]],
            vec![vec![2., 0.4, -1.], vec![3., 0.6, -1.]],
        ] {
            let report =
                inspect(&surface, &Curve::from_polyline(points).unwrap(), 1e-6, 4096).unwrap();
            assert!(report.covers.is_empty());
            assert!(report.unresolved.is_empty());
        }
    }
    #[test]
    fn rational_chart_and_work_exhaustion_never_report_false_visibility() {
        let mut surface = arch();
        surface.weights[1] = vec![0.7; 2];
        let behind =
            Curve::from_polyline(vec![vec![0.45, 0.45, -1.], vec![0.46, 0.46, -1.]]).unwrap();
        assert!(
            !inspect(&surface, &behind, 1e-6, 4096)
                .unwrap()
                .covers
                .is_empty()
        );
        let wide = Curve::from_polyline(vec![vec![-1., 0.5, -1.], vec![2., 0.5, -1.]]).unwrap();
        let limited = inspect(&surface, &wide, 1e-6, 1).unwrap();
        assert!(limited.budget_exhausted);
        assert!(!limited.unresolved.is_empty());
        assert!(limited.covers.is_empty());
    }
}

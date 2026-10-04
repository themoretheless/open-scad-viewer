//! Existence certificates for isolated roots of a surface/surface section.
//! One parameter of the first surface is fixed. A certificate proves a contact
//! point, not completeness of a 4D intersection curve or B-rep trim membership.
use crate::{Result, check, distance_bounds::Interval as I, surface::Surface};
#[derive(Clone, Debug)]
pub struct Witness {
    pub first_uv: [[f64; 2]; 2],
    pub second_uv: [[f64; 2]; 2],
    pub point: [[f64; 2]; 3],
    pub contraction_upper: f64,
}
#[derive(Clone, Debug)]
pub enum Verdict {
    Excluded,
    Unresolved,
    Witness(Witness),
}
fn inverse(a: [[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let mut m = [[0.; 6]; 3];
    for i in 0..3 {
        for j in 0..3 {
            m[i][j] = a[i][j];
        }
        m[i][i + 3] = 1.;
    }
    for k in 0..3 {
        let pivot = (k..3).max_by(|&i, &j| m[i][k].abs().total_cmp(&m[j][k].abs()))?;
        m.swap(k, pivot);
        let scale = m[k][k];
        if !scale.is_finite() || scale == 0. {
            return None;
        }
        for j in 0..6 {
            m[k][j] /= scale;
        }
        for i in 0..3 {
            if i == k {
                continue;
            }
            let scale = m[i][k];
            for j in 0..6 {
                m[i][j] -= scale * m[k][j];
            }
        }
    }
    let result = std::array::from_fn(|i| std::array::from_fn(|j| m[i][j + 3]));
    if result.iter().flatten().any(|v| !v.is_finite()) {
        None
    } else {
        Some(result)
    }
}
fn span(s: &Surface, d: [[f64; 2]; 2]) -> Option<[usize; 2]> {
    let find = |k: &[f64], p: usize, n: usize, r: [f64; 2]| {
        (p..n).find(|&i| k[i] < k[i + 1] && r[0] >= k[i] && r[1] <= k[i + 1])
    };
    Some([
        find(&s.knots_u, s.degree_u, s.control_points.len(), d[0])?,
        find(&s.knots_v, s.degree_v, s.control_points[0].len(), d[1])?,
    ])
}
fn bounds(s: &Surface, d: [[f64; 2]; 2]) -> Result<Vec<I>> {
    crate::surface_distance::rectangle_bounds(s, d)?
        .into_iter()
        .map(|[lo, hi]| I::new(lo, hi))
        .collect()
}
/// Every interval is in the original natural parameter domain. Boxes crossing
/// an interior knot return Unresolved and must be split by the caller.
pub fn certify(
    a: &Surface,
    b: &Surface,
    fixed_axis: usize,
    fixed: f64,
    first_other: [f64; 2],
    second: [[f64; 2]; 2],
) -> Result<Verdict> {
    a.validate()?;
    b.validate()?;
    check(
        fixed_axis < 2 && fixed.is_finite(),
        "Choose a finite fixed surface parameter",
    )?;
    check(
        std::iter::once(&first_other)
            .chain(&second)
            .all(|d| d.iter().all(|x| x.is_finite()) && d[0] < d[1]),
        "Contact intervals must have positive finite widths",
    )?;
    let free = 1 - fixed_axis;
    let mut first = [first_other; 2];
    first[fixed_axis] = [fixed; 2];
    let pa = bounds(a, first)?;
    let pb = bounds(b, second)?;
    if (0..3).any(|k| pa[k].hi < pb[k].lo || pb[k].hi < pa[k].lo) {
        return Ok(Verdict::Excluded);
    }
    let (Some(sa), Some(sb)) = (span(a, first), span(b, second)) else {
        return Ok(Verdict::Unresolved);
    };
    let ja = crate::surface_injectivity::section_jacobian(a, sa, first)?;
    let jb = crate::surface_injectivity::section_jacobian(b, sb, second)?;
    let mut jac = [[I::point(0.); 3]; 3];
    for k in 0..3 {
        jac[k] = [
            ja[k][free],
            I::point(0.).sub(jb[k][0])?,
            I::point(0.).sub(jb[k][1])?,
        ];
    }
    let Some(y) = inverse(jac.map(|r| r.map(|v| v.lo * 0.5 + v.hi * 0.5))) else {
        return Ok(Verdict::Unresolved);
    };
    let domain = [first_other, second[0], second[1]];
    let center = domain.map(|d| d[0] * 0.5 + d[1] * 0.5);
    if (0..3).any(|k| center[k] <= domain[k][0] || center[k] >= domain[k][1]) {
        return Ok(Verdict::Unresolved);
    }
    let mut ac = first;
    ac[free] = [center[0]; 2];
    let ac = bounds(a, ac)?;
    let bc = bounds(b, [[center[1]; 2], [center[2]; 2]])?;
    let mut f = [I::point(0.); 3];
    for k in 0..3 {
        f[k] = ac[k].sub(bc[k])?;
    }
    let mut image = center.map(I::point);
    let mut contraction = 0_f64;
    for i in 0..3 {
        let mut row = I::point(0.);
        for j in 0..3 {
            image[i] = image[i].sub(I::point(y[i][j]).mul(f[j])?)?;
        }
        for k in 0..3 {
            let mut residual = I::point(if i == k { 1. } else { 0. });
            for j in 0..3 {
                residual = residual.sub(I::point(y[i][j]).mul(jac[j][k])?)?;
            }
            row = row.add(I::point(residual.lo.abs().max(residual.hi.abs())))?;
            let offset = I::new(domain[k][0], domain[k][1])?.sub(I::point(center[k]))?;
            image[i] = image[i].add(residual.mul(offset)?)?;
        }
        contraction = contraction.max(row.hi);
    }
    if (0..3).any(|k| image[k].hi < domain[k][0] || image[k].lo > domain[k][1]) {
        return Ok(Verdict::Excluded);
    }
    // A norm bound below one for I-YJ proves YJ, and therefore Y,
    // invertible. The invariant image gives a fixed point with F=0, not merely
    // a small Newton residual. The whole 3D section box has a unique root.
    if contraction >= 0.5
        || (0..3).any(|k| image[k].lo <= domain[k][0] || image[k].hi >= domain[k][1])
    {
        return Ok(Verdict::Unresolved);
    }
    first[free] = [image[0].lo, image[0].hi];
    let second = [[image[1].lo, image[1].hi], [image[2].lo, image[2].hi]];
    let pa = bounds(a, first)?;
    let pb = bounds(b, second)?;
    let mut point = [[0.; 2]; 3];
    for k in 0..3 {
        let v = I::new(pa[k].lo.max(pb[k].lo), pa[k].hi.min(pb[k].hi))?;
        point[k] = [v.lo, v.hi];
    }
    Ok(Verdict::Witness(Witness {
        first_uv: first,
        second_uv: second,
        point,
        contraction_upper: contraction,
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn plane() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            weights: vec![vec![1.; 2]; 2],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn crossing_planes_have_a_certified_contact() {
        let a = plane();
        let mut b = plane();
        for row in &mut b.control_points {
            for p in row {
                p[2] = p[1] - 0.5;
                p[1] = 0.5;
            }
        }
        let Verdict::Witness(w) = certify(&a, &b, 0, 0.375, [0., 1.], [[0., 1.]; 2]).unwrap()
        else {
            panic!("contact not certified")
        };
        for (box_, x) in w.point.iter().zip([0.375, 0.5, 0.]) {
            assert!(box_[0] <= x && box_[1] >= x);
        }
        assert!(w.contraction_upper < 0.5);
        for row in &mut b.control_points {
            for p in row {
                p[2] += 3.;
            }
        }
        assert!(matches!(
            certify(&a, &b, 0, 0.375, [0., 1.], [[0., 1.]; 2]).unwrap(),
            Verdict::Excluded
        ));
    }
    #[test]
    fn curved_contact_and_singular_coincidence() {
        let a = plane();
        let mut b = plane();
        for row in &mut b.control_points {
            for p in row {
                p[2] = p[0] * p[1] - 0.1875;
            }
        }
        assert!(matches!(
            certify(&a, &b, 0, 0.375, [0.4, 0.6], [[0.3, 0.45], [0.4, 0.6]]).unwrap(),
            Verdict::Witness(_)
        ));
        assert!(matches!(
            certify(&a, &a, 0, 0.375, [0., 1.], [[0., 1.]; 2]).unwrap(),
            Verdict::Unresolved
        ));
    }
    #[test]
    fn alternate_axis_rational_weights_and_nonunit_domains() {
        let mut a = plane();
        let mut b = plane();
        for row in &mut b.control_points {
            for p in row {
                p[2] = p[0] - 0.5;
                p[0] = 0.5;
            }
        }
        for s in [&mut a, &mut b] {
            s.knots_u = vec![0.1, 0.1, 1.3, 1.3];
            s.knots_v = vec![-0.7, -0.7, 2.1, 2.1];
            s.weights = vec![vec![1., 1.02], vec![1.01, 1.0302]];
        }
        let Verdict::Witness(w) =
            certify(&a, &b, 1, 0.7, [0.65, 0.75], [[0.65, 0.75], [0.65, 0.75]]).unwrap()
        else {
            panic!("rational contact not certified")
        };
        assert!(w.point[0][0] <= 0.5 && w.point[0][1] >= 0.5);
        assert!(w.point[2][0] <= 0. && w.point[2][1] >= 0.);
    }
    #[test]
    fn boundary_contact_and_invalid_boxes_are_not_certificates() {
        let a = plane();
        let mut b = plane();
        for row in &mut b.control_points {
            for p in row {
                p[2] = p[1];
                p[1] = 0.5;
            }
        }
        assert!(matches!(
            certify(&a, &b, 0, 0.375, [0., 1.], [[0., 1.]; 2]).unwrap(),
            Verdict::Unresolved
        ));
        assert!(certify(&a, &b, 2, 0.375, [0., 1.], [[0., 1.]; 2]).is_err());
        assert!(certify(&a, &b, 0, 0.375, [0.5, 0.5], [[0., 1.]; 2]).is_err());
        assert!(certify(&a, &b, 0, f64::NAN, [0., 1.], [[0., 1.]; 2]).is_err());
    }
}

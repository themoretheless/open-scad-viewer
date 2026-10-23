//! Existence certificates for isolated roots of a surface/surface section.
//! One parameter of the first surface is fixed. A certificate proves a contact
//! point, not completeness of a 4D intersection curve or B-rep trim membership.
use crate::foundation::guards::{require_finite_f64, require_finite_point};
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
const CACHE_LIMIT: usize = 128;
type DomainKey = [u64; 4];
type Jacobian = [[I; 2]; 3];
fn domain_key(d: [[f64; 2]; 2]) -> DomainKey {
    [d[0][0], d[0][1], d[1][0], d[1][1]].map(f64::to_bits)
}
// Owned by one search and tied to one immutable surface borrow. Every cached
// value is the exact result of the original interval routine, not a new bound.
// Fixed-size LRU storage cannot grow with subdivision or outlive its source.
pub(crate) struct PreparedSurface<'a> {
    surface: &'a Surface,
    pub(crate) controls: crate::surface_distance::PreparedControlSurface<'a>,
    bounds: std::collections::VecDeque<(DomainKey, Vec<I>)>,
    jacobians: std::collections::VecDeque<(([usize; 2], DomainKey), Jacobian)>,
}
impl<'a> PreparedSurface<'a> {
    pub(crate) fn new(surface: &'a Surface) -> Result<Self> {
        surface.validate()?;
        Ok(Self { surface, controls: crate::surface_distance::PreparedControlSurface::new(surface), bounds: Default::default(), jacobians: Default::default() })
    }
    pub(crate) fn bounds(&mut self, domain: [[f64; 2]; 2]) -> Result<Vec<I>> {
        let key = domain_key(domain);
        if let Some(index) = self.bounds.iter().position(|entry| entry.0 == key) {
            let entry = self.bounds.remove(index).unwrap();
            let result = entry.1.clone();
            self.bounds.push_back(entry);
            return Ok(result);
        }
        let value = match self.controls.single_span_bounds(domain)? {
            Some(value) => value,
            None => bounds(self.surface, domain)?,
        };
        if self.bounds.len() == CACHE_LIMIT { self.bounds.pop_front(); }
        self.bounds.push_back((key, value.clone()));
        Ok(value)
    }
    fn jacobian(&mut self, span: [usize; 2], domain: [[f64; 2]; 2]) -> Result<Jacobian> {
        let key = (span, domain_key(domain));
        if let Some(index) = self.jacobians.iter().position(|entry| entry.0 == key) {
            let entry = self.jacobians.remove(index).unwrap();
            let result = entry.1;
            self.jacobians.push_back(entry);
            return Ok(result);
        }
        let value = crate::surface_injectivity::section_jacobian(self.surface, span, domain)?;
        if self.jacobians.len() == CACHE_LIMIT { self.jacobians.pop_front(); }
        self.jacobians.push_back((key, value));
        Ok(value)
    }
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
    let mut a = PreparedSurface::new(a)?;
    let mut b = PreparedSurface::new(b)?;
    certify_prepared(&mut a, &mut b, fixed_axis, fixed, first_other, second)
}
pub(crate) fn certify_prepared(
    a: &mut PreparedSurface<'_>,
    b: &mut PreparedSurface<'_>,
    fixed_axis: usize,
    fixed: f64,
    first_other: [f64; 2],
    second: [[f64; 2]; 2],
) -> Result<Verdict> {
    check(fixed_axis < 2, "Choose a fixed surface parameter axis")?;
    // Boundary validation through the unified finite guards (item 1093).
    require_finite_f64(fixed, "fixed")?;
    require_finite_point(&first_other, "first_other")?;
    require_finite_point(&second[0], "second u")?;
    require_finite_point(&second[1], "second v")?;
    check(
        std::iter::once(&first_other)
            .chain(&second)
            .all(|d| d[0] < d[1]),
        "Contact intervals must have positive widths",
    )?;
    let free = 1 - fixed_axis;
    let mut first = [first_other; 2];
    first[fixed_axis] = [fixed; 2];
    let pa = a.bounds(first)?;
    let pb = b.bounds(second)?;
    if (0..3).any(|k| pa[k].hi < pb[k].lo || pb[k].hi < pa[k].lo) {
        return Ok(Verdict::Excluded);
    }
    let (Some(sa), Some(sb)) = (span(a.surface, first), span(b.surface, second)) else {
        return Ok(Verdict::Unresolved);
    };
    let ja = a.jacobian(sa, first)?;
    let jb = b.jacobian(sb, second)?;
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
    let ac = a.bounds(ac)?;
    let bc = b.bounds([[center[1]; 2], [center[2]; 2]])?;
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
    let pa = a.bounds(first)?;
    let pb = b.bounds(second)?;
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
    fn cached_sections_keep_exact_witnesses_bounded_storage_and_source_ownership() {
        let a = plane();
        let mut b = plane();
        for row in &mut b.control_points { for p in row {
            p[2] = p[1] - 0.5; p[1] = 0.5;
        }}
        let bits = |v: Verdict| match v {
            Verdict::Excluded => vec![0], Verdict::Unresolved => vec![1],
            Verdict::Witness(w) => std::iter::once(2).chain(w.first_uv.into_iter().flatten()
                .chain(w.second_uv.into_iter().flatten()).chain(w.point.into_iter().flatten())
                .chain(std::iter::once(w.contraction_upper)).map(f64::to_bits)).collect(),
        };
        let mut ca = PreparedSurface::new(&a).unwrap();
        let mut cb = PreparedSurface::new(&b).unwrap();
        for i in 0..80 {
            let fixed = 0.125 + 0.75 * i as f64 / 80.;
            let original = certify(&a, &b, 0, fixed, [0., 1.], [[0., 1.]; 2]).unwrap();
            assert!(matches!(original, Verdict::Witness(_)));
            let cached = certify_prepared(&mut ca, &mut cb, 0, fixed, [0., 1.], [[0., 1.]; 2]).unwrap();
            assert_eq!(bits(original), bits(cached));
            assert!(ca.bounds.len() <= CACHE_LIMIT && ca.jacobians.len() <= CACHE_LIMIT);
            assert!(cb.bounds.len() <= CACHE_LIMIT && cb.jacobians.len() <= CACHE_LIMIT);
        }
        drop(cb);
        // A changed surface must get a fresh owner; an address cannot select
        // cached intervals belonging to the old geometry.
        for row in &mut b.control_points { for p in row { p[2] += 3.; }}
        let mut changed = PreparedSurface::new(&b).unwrap();
        assert!(matches!(certify_prepared(&mut ca, &mut changed, 0, 0.375,
            [0., 1.], [[0., 1.]; 2]).unwrap(), Verdict::Excluded));
    }
    #[test]
    fn cached_rectangles_match_original_multispan_rational_bounds_and_guards() {
        let mut s = plane();
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
        s.control_points.insert(1, vec![vec![0.5, 0., 0.2], vec![0.5, 1., -0.2]]);
        s.weights = vec![vec![1., 0.5], vec![0.75, 1.25], vec![0.5, 1.]];
        let mut cached = PreparedSurface::new(&s).unwrap();
        let bits = |v: Vec<I>| v.into_iter().flat_map(|x| [x.lo.to_bits(), x.hi.to_bits()]).collect::<Vec<_>>();
        for i in 0..80 {
            let u = i as f64 / 160.;
            for domain in [[[u, 1.-u], [0., 1.]], [[0.5, 0.5], [u, 1.-u]],
                [[0.5_f64.next_down(), 0.5_f64.next_up()], [0., 1.]]] {
                let expected = bits(bounds(&s, domain).unwrap());
                assert_eq!(bits(cached.bounds(domain).unwrap()), expected);
                assert_eq!(bits(cached.bounds(domain).unwrap()), expected);
                assert!(cached.bounds.len() <= CACHE_LIMIT);
            }
        }
        let before = cached.bounds.len();
        for domain in [[[1., 0.], [0., 1.]], [[-0.1, 0.5], [0., 1.]],
            [[0., f64::NAN], [0., 1.]]] {
            assert!(bounds(&s, domain).is_err());
            assert!(cached.bounds(domain).is_err());
            assert_eq!(cached.bounds.len(), before);
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

    #[test]
    fn non_finite_inputs_name_the_offending_parameter() {
        let a = plane();
        let b = plane();
        let err = certify(&a, &b, 0, f64::NAN, [0., 1.], [[0., 1.]; 2]).unwrap_err();
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
        assert!(err.contains("fixed"), "{err}");
        let err = certify(&a, &b, 0, 0.375, [0., f64::INFINITY], [[0., 1.]; 2]).unwrap_err();
        assert!(err.contains("first_other[1]"), "{err}");
        let err = certify(&a, &b, 1, 0.5, [0., 1.], [[0., 1.], [f64::NAN, 1.]]).unwrap_err();
        assert!(err.contains("second v[0]"), "{err}");
    }
}

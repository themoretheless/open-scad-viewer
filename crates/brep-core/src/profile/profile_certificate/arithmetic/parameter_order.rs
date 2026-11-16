//! Exact comparison of selected algebraic images; overlapping intervals alone
//! never grant equality. Common value factors plus a unique selected root do.
use super::*;
use algebraic_events as roots;
use event_coordinates as coordinates;
use std::cmp::Ordering;
fn bound(p: &Poly) -> Result<()> {
    if p.len() > 513
        || p.iter()
            .any(|q| q.numer().bits() > 16384 || q.denom().bits() > 16384)
    {
        return Err(refused(
            "Algebraic parameter comparison arithmetic allowance exceeded",
        ));
    }
    Ok(())
}
fn reduced(p: &Poly, f: &Poly) -> Result<Poly> {
    bound(p)?;
    let r = divide(p, f)?.1;
    bound(&r)?;
    Ok(r)
}
fn compose(p: &Poly, t: &Poly, f: &Poly) -> Result<Poly> {
    p.iter().rev().try_fold(Vec::new(), |v, c| {
        let mut r = reduced(&product(&v, t), f)?;
        if r.is_empty() {
            r.push(c.clone());
        } else {
            r[0] += c;
        }
        Ok(trim(r))
    })
}
fn image_polynomial(f: &Poly, p: &Poly, max_matrix: usize) -> Result<Poly> {
    if p.len() <= 1 {
        return Ok(vec![-p.first().cloned().unwrap_or_else(Q::zero), Q::one()]);
    }
    if f.len() + p.len() - 2 > max_matrix {
        return Err(refused(
            "Algebraic value eliminant matrix allowance exceeded",
        ));
    }
    // Q[value][alpha]: eliminate alpha from F(alpha), value-P(alpha).
    let a = f.iter().map(|q| trim(vec![q.clone()])).collect::<Vec<_>>();
    let mut b = p.iter().map(|q| trim(vec![-q.clone()])).collect::<Vec<_>>();
    b[0] = vec![-p[0].clone(), Q::one()];
    let h = bivariate::resultant(&a, &b)?;
    bound(&h)?;
    if h.len() < 2 {
        return Err(refused("Algebraic value has no finite eliminant"));
    }
    let mut h = quotient(&h, &gcd(&h, &derivative(&h))?)?;
    let lead = h.last().cloned().unwrap();
    for q in &mut h {
        *q /= &lead;
    }
    Ok(h)
}
pub(in crate::profile::profile_certificate) fn range(p: &Poly, i: &[Q; 2]) -> [Q; 2] {
    p.iter().rev().fold([Q::zero(), Q::zero()], |v, c| {
        let q = [&v[0] * &i[0], &v[0] * &i[1], &v[1] * &i[0], &v[1] * &i[1]];
        [q.iter().min().unwrap() + c, q.iter().max().unwrap() + c]
    })
}
pub(in crate::profile::profile_certificate) fn refine(f: &Poly, i: &mut [Q; 2]) -> Result<()> {
    if i[0] == i[1] {
        return Ok(());
    }
    let m = (&i[0] + &i[1]) / Q::from_integer(2.into());
    if at(f, &m).is_zero() {
        *i = [m.clone(), m];
    } else if roots::count(f, &i[0], &m)? == 1 {
        i[1] = m;
    } else {
        i[0] = m;
    }
    Ok(())
}
pub(super) fn closed_roots(f: &Poly, i: &[Q; 2]) -> Result<usize> {
    if i[0] == i[1] {
        return Ok(usize::from(at(f, &i[0]).is_zero()));
    }
    Ok(roots::count(f, &i[0], &i[1])?
        + usize::from(at(f, &i[0]).is_zero())
        + usize::from(at(f, &i[1]).is_zero()))
}
/// A borrowed algebraic parameter value: the root of `primitive` isolated by
/// `interval`, mapped through the parameter polynomial `value`.
#[derive(Clone, Copy)]
pub(in crate::profile::profile_certificate) struct ParameterRef<'a> {
    pub primitive: &'a Poly,
    pub value: &'a Poly,
    pub interval: &'a [Q; 2],
}
impl<'a> ParameterRef<'a> {
    pub(in crate::profile::profile_certificate) fn new(
        primitive: &'a Poly,
        value: &'a Poly,
        interval: &'a [Q; 2],
    ) -> Self {
        Self {
            primitive,
            value,
            interval,
        }
    }
}
pub(in crate::profile::profile_certificate) fn compare(
    first: ParameterRef<'_>,
    second: ParameterRef<'_>,
    max_refinements: usize,
    max_matrix: usize,
) -> Result<Ordering> {
    let ParameterRef {
        primitive: f,
        value: p,
        interval: a,
    } = first;
    let ParameterRef {
        primitive: g,
        value: q,
        interval: b,
    } = second;
    bound(f)?;
    bound(g)?;
    let p = reduced(p, f)?;
    let q = reduced(q, g)?;
    if f == g && p == q {
        let intersection = [
            a[0].clone().max(b[0].clone()),
            a[1].clone().min(b[1].clone()),
        ];
        if intersection[0] <= intersection[1] && closed_roots(f, &intersection)? == 1 {
            return Ok(Ordering::Equal);
        }
    }
    if p.len() <= 1
        && coordinates::zero_at(
            &sub(&q, &[p.first().cloned().unwrap_or_else(Q::zero)]),
            g,
            b,
        )?
    {
        return Ok(Ordering::Equal);
    }
    if q.len() <= 1
        && coordinates::zero_at(
            &sub(&p, &[q.first().cloned().unwrap_or_else(Q::zero)]),
            f,
            a,
        )?
    {
        return Ok(Ordering::Equal);
    }
    let (mut a, mut b) = (a.clone(), b.clone());
    let mut common: Option<Poly> = None;
    let mut equality_possible = false;
    let mut attempted_eliminant = false;
    for step in 0..=max_refinements {
        let x = range(&p, &a);
        let y = range(&q, &b);
        if x[1] < y[0] {
            return Ok(Ordering::Less);
        }
        if x[0] > y[1] {
            return Ok(Ordering::Greater);
        }
        if x[0] == x[1] && y[0] == y[1] {
            return Ok(x[0].cmp(&y[0]));
        }
        if !attempted_eliminant && (step >= 8 || step == max_refinements) {
            attempted_eliminant = true;
            // An eliminant budget refusal does not prevent independently
            // proving strict order by refined original root enclosures.
            if let (Ok(left), Ok(right)) = (
                image_polynomial(f, &p, max_matrix),
                image_polynomial(g, &q, max_matrix),
            ) {
                let h = gcd(&left, &right)?;
                equality_possible = h.len() > 1
                    && coordinates::zero_at(&compose(&h, &p, f)?, f, &a)?
                    && coordinates::zero_at(&compose(&h, &q, g)?, g, &b)?;
                common = Some(h);
            }
        }
        if equality_possible {
            let interval = [
                x[0].clone().min(y[0].clone()),
                x[1].clone().max(y[1].clone()),
            ];
            // Both selected values are roots of the SAME exact polynomial;
            // exactly one root in their joint enclosure proves identity.
            if closed_roots(common.as_ref().unwrap(), &interval)? == 1 {
                return Ok(Ordering::Equal);
            }
        }
        if step < max_refinements {
            refine(f, &mut a)?;
            refine(g, &mut b)?;
        }
    }
    Err(refused(
        "Original algebraic parameter order remains unproven within allowance",
    ))
}

fn prepare_fraction(f: &Poly, n: &Poly, d: &Poly, root: &[Q; 2]) -> Result<(Poly, Poly, Poly)> {
    bound(f)?;
    bound(n)?;
    bound(d)?;
    if coordinates::zero_at(d, f, root)? {
        return Err(refused("Selected algebraic fraction denominator vanishes"));
    }
    // Remove poles at OTHER roots before eliminating value*D-N. The selected
    // original root is retained; no illegal inversion in a reducible quotient.
    let factor = quotient(f, &gcd(f, d)?)?;
    if closed_roots(&factor, root)? != 1 {
        return Err(refused(
            "Algebraic fraction lost its selected original root",
        ));
    }
    Ok((factor.clone(), reduced(n, &factor)?, reduced(d, &factor)?))
}
fn constant_fraction(f: &Poly, n: &Poly, d: &Poly, root: &[Q; 2]) -> Result<Option<Q>> {
    let index = d
        .iter()
        .position(|q| !q.is_zero())
        .ok_or_else(|| refused("Zero algebraic fraction denominator"))?;
    let value = n.get(index).cloned().unwrap_or_else(Q::zero) / &d[index];
    let difference = sub(n, &d.iter().map(|q| q * &value).collect::<Poly>());
    Ok(coordinates::zero_at(&difference, f, root)?.then_some(value))
}
pub(super) fn fraction_range(n: &Poly, d: &Poly, i: &[Q; 2]) -> Option<[Q; 2]> {
    let x = range(n, i);
    let y = range(d, i);
    if y[0] <= Q::zero() && y[1] >= Q::zero() {
        return None;
    }
    let q = [&x[0] / &y[0], &x[0] / &y[1], &x[1] / &y[0], &x[1] / &y[1]];
    Some([
        q.iter().min().unwrap().clone(),
        q.iter().max().unwrap().clone(),
    ])
}
fn fraction_image(f: &Poly, n: &Poly, d: &Poly, max_matrix: usize) -> Result<Poly> {
    let degree = n.len().max(d.len()).saturating_sub(1);
    if f.len() - 1 + degree > max_matrix {
        return Err(refused(
            "Algebraic fraction eliminant matrix allowance exceeded",
        ));
    }
    let a = f.iter().map(|q| trim(vec![q.clone()])).collect::<Vec<_>>();
    let b = (0..=degree)
        .map(|i| {
            trim(vec![
                -n.get(i).cloned().unwrap_or_else(Q::zero),
                d.get(i).cloned().unwrap_or_else(Q::zero),
            ])
        })
        .collect::<Vec<_>>();
    let h = bivariate::resultant(&a, &b)?;
    bound(&h)?;
    if h.len() < 2 {
        return Err(refused("Algebraic fraction has no finite value eliminant"));
    }
    let mut h = quotient(&h, &gcd(&h, &derivative(&h))?)?;
    let lead = h.last().cloned().unwrap();
    for q in &mut h {
        *q /= &lead;
    }
    Ok(h)
}
fn compose_fraction(h: &Poly, n: &Poly, d: &Poly, f: &Poly) -> Result<Poly> {
    let Some(last) = h.last() else {
        return Ok(Vec::new());
    };
    let mut v = vec![last.clone()];
    let mut power_d = vec![Q::one()];
    for c in h[..h.len() - 1].iter().rev() {
        power_d = reduced(&product(&power_d, d), f)?;
        v = reduced(
            &sub(
                &product(&v, n),
                &power_d.iter().map(|q| -q * c).collect::<Poly>(),
            ),
            f,
        )?;
    }
    Ok(v)
}
/// A borrowed algebraic parameter value given by a rational map: the root of
/// `primitive` isolated by `interval`, mapped through `fraction[0] / fraction[1]`.
#[derive(Clone, Copy)]
pub(in crate::profile::profile_certificate) struct RationalParameterRef<'a> {
    pub primitive: &'a Poly,
    pub fraction: [&'a Poly; 2],
    pub interval: &'a [Q; 2],
}
impl<'a> RationalParameterRef<'a> {
    pub(in crate::profile::profile_certificate) fn new(
        primitive: &'a Poly,
        fraction: [&'a Poly; 2],
        interval: &'a [Q; 2],
    ) -> Self {
        Self {
            primitive,
            fraction,
            interval,
        }
    }
}
/// Refinement step at which the shared image polynomial is first tried.
const IMAGE_ATTEMPT_STEP: usize = 8;
/// The value `n / d` selected at one isolated root of `f`.
struct Fraction {
    f: Poly,
    n: Poly,
    d: Poly,
    root: [Q; 2],
    /// Set when the fraction is one rational number at the root.
    constant: Option<Q>,
}
impl Fraction {
    fn new((f, n, d): (Poly, Poly, Poly), root: &[Q; 2]) -> Result<Self> {
        let constant = constant_fraction(&f, &n, &d, root)?;
        Ok(Self {
            f,
            n,
            d,
            root: root.clone(),
            constant,
        })
    }
    fn range(&self) -> Option<[Q; 2]> {
        self.constant
            .as_ref()
            .map(|c| [c.clone(), c.clone()])
            .or_else(|| fraction_range(&self.n, &self.d, &self.root))
    }
    fn equals(&self, c: &Q) -> Result<bool> {
        let scaled = self.d.iter().map(|q| q * c).collect::<Poly>();
        coordinates::zero_at(&sub(&self.n, &scaled), &self.f, &self.root)
    }
    fn annihilated_by(&self, h: &Poly) -> Result<bool> {
        coordinates::zero_at(
            &compose_fraction(h, &self.n, &self.d, &self.f)?,
            &self.f,
            &self.root,
        )
    }
}
/// Order decided by exact identities alone: constants, a constant against a
/// fraction, or two fractions at the same root of one primitive.
fn exact_order(x: &Fraction, y: &Fraction) -> Result<Option<Ordering>> {
    if let (Some(a), Some(b)) = (&x.constant, &y.constant) {
        return Ok(Some(a.cmp(b)));
    }
    if let Some(c) = &x.constant {
        if y.equals(c)? {
            return Ok(Some(Ordering::Equal));
        }
    }
    if let Some(c) = &y.constant {
        if x.equals(c)? {
            return Ok(Some(Ordering::Equal));
        }
    }
    if x.f == y.f {
        let intersection = [
            x.root[0].clone().max(y.root[0].clone()),
            x.root[1].clone().min(y.root[1].clone()),
        ];
        if intersection[0] <= intersection[1]
            && closed_roots(&x.f, &intersection)? == 1
            && coordinates::zero_at(
                &sub(&product(&x.n, &y.d), &product(&y.n, &x.d)),
                &x.f,
                &intersection,
            )?
        {
            return Ok(Some(Ordering::Equal));
        }
    }
    Ok(None)
}
/// Order decided by two value enclosures, when they are disjoint or exact.
fn enclosure_order(x: &[Q; 2], y: &[Q; 2]) -> Option<Ordering> {
    if x[1] < y[0] {
        Some(Ordering::Less)
    } else if x[0] > y[1] {
        Some(Ordering::Greater)
    } else if x[0] == x[1] && y[0] == y[1] {
        Some(x[0].cmp(&y[0]))
    } else {
        None
    }
}
/// A polynomial that vanishes at both selected values, when their image
/// polynomials share a factor that does.
fn common_image(x: &Fraction, y: &Fraction, max_matrix: usize) -> Result<Option<Poly>> {
    let (Ok(left), Ok(right)) = (
        fraction_image(&x.f, &x.n, &x.d, max_matrix),
        fraction_image(&y.f, &y.n, &y.d, max_matrix),
    ) else {
        return Ok(None);
    };
    let h = gcd(&left, &right)?;
    Ok((h.len() > 1 && x.annihilated_by(&h)? && y.annihilated_by(&h)?).then_some(h))
}
pub(in crate::profile::profile_certificate) fn compare_rational(
    first: RationalParameterRef<'_>,
    second: RationalParameterRef<'_>,
    max_refinements: usize,
    max_matrix: usize,
) -> Result<Ordering> {
    let left = prepare_fraction(
        first.primitive,
        first.fraction[0],
        first.fraction[1],
        first.interval,
    )?;
    let right = prepare_fraction(
        second.primitive,
        second.fraction[0],
        second.fraction[1],
        second.interval,
    )?;
    let mut x = Fraction::new(left, first.interval)?;
    let mut y = Fraction::new(right, second.interval)?;
    if let Some(order) = exact_order(&x, &y)? {
        return Ok(order);
    }
    let mut attempted = false;
    let mut common = None;
    for step in 0..=max_refinements {
        if let (Some(r), Some(s)) = (x.range(), y.range()) {
            if let Some(order) = enclosure_order(&r, &s) {
                return Ok(order);
            }
            if !attempted && (step >= IMAGE_ATTEMPT_STEP || step == max_refinements) {
                attempted = true;
                common = common_image(&x, &y, max_matrix)?;
            }
            if let Some(h) = &common {
                let hull = [
                    r[0].clone().min(s[0].clone()),
                    r[1].clone().max(s[1].clone()),
                ];
                if closed_roots(h, &hull)? == 1 {
                    return Ok(Ordering::Equal);
                }
            }
        }
        if step < max_refinements {
            refine(&x.f, &mut x.root)?;
            refine(&y.f, &mut y.root)?;
        }
    }
    Err(refused(
        "Original algebraic fraction order remains unproven within allowance",
    ))
}

#[cfg(test)]
mod rational_tests {
    use super::*;
    #[test]
    fn unrelated_primitive_poles_do_not_destroy_a_valid_selected_fraction() {
        let f = vec![-Q::one(), Q::zero(), Q::one()];
        let n = vec![Q::zero(), Q::one(), Q::one()];
        let d = vec![Q::one(), Q::one()];
        let g = vec![Q::zero(), Q::one()];
        let one = vec![Q::one()];
        assert_eq!(
            compare_rational(
                RationalParameterRef::new(&f, [&n, &d], &[Q::one(), Q::one()]),
                RationalParameterRef::new(&g, [&one, &one], &[Q::zero(), Q::zero()]),
                0,
                1,
            )
            .unwrap(),
            Ordering::Equal
        );
        assert!(
            compare_rational(
                RationalParameterRef::new(&f, [&n, &d], &[-Q::one(), -Q::one()]),
                RationalParameterRef::new(&g, [&one, &one], &[Q::zero(), Q::zero()]),
                0,
                1,
            )
            .is_err()
        );
    }
    #[test]
    fn rational_images_in_distinct_real_extensions_have_checked_value_identity() {
        let q = |n: i64, d: i64| Q::new(n.into(), d.into());
        let f = vec![q(-2, 1), Q::zero(), Q::one()];
        let n = vec![Q::one(), Q::one()];
        let d = vec![-Q::one(), Q::one()];
        let g = vec![-Q::one(), q(-2, 1), Q::one()];
        let m = vec![Q::zero(), Q::zero(), Q::one()];
        let one = vec![Q::one()];
        assert_eq!(
            compare_rational(
                RationalParameterRef::new(&f, [&n, &d], &[Q::one(), q(3, 2)]),
                RationalParameterRef::new(&g, [&m, &one], &[q(2, 1), q(5, 2)]),
                192,
                64,
            )
            .unwrap(),
            Ordering::Equal
        );
    }
}

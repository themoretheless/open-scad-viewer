//! Exact source event coordinates in a checked real algebraic root extension.
//! Denominators stay symbolic: no inversion at unrelated complex roots is needed.
use super::*;
use algebraic_events as roots;
pub(super) fn bounded(p: &Poly) -> Result<()> {
    if p.len() > 513
        || p.iter()
            .any(|q| q.numer().bits() > 16384 || q.denom().bits() > 16384)
    {
        return Err(refused("Original algebraic coordinate allowance exceeded"));
    }
    Ok(())
}
pub(super) fn modulo(p: &Poly, f: &Poly) -> Result<Poly> {
    bounded(p)?;
    let r = divide(p, f)?.1;
    bounded(&r)?;
    Ok(r)
}
pub(in crate::profile_certificate) fn compose(p: &Poly, t: &Poly, f: &Poly) -> Result<Poly> {
    p.iter().rev().try_fold(Vec::new(), |v, c| {
        let mut r = modulo(&product(&v, t), f)?;
        if r.is_empty() {
            r.push(c.clone());
        } else {
            r[0] += c;
        }
        let r = trim(r);
        bounded(&r)?;
        Ok(r)
    })
}
pub(in crate::profile_certificate) fn first_parameter(t: &Poly, k: u8) -> Poly {
    sub(
        &vec![Q::zero(), Q::one()],
        &t.iter()
            .map(|q| q * Q::from_integer(k.into()))
            .collect::<Poly>(),
    )
}
pub(in crate::profile_certificate) fn zero_at(p: &Poly, f: &Poly, root: &[Q; 2]) -> Result<bool> {
    if p.is_empty() {
        return Ok(true);
    }
    let g = gcd(p, f)?;
    Ok(g.len() > 1 && roots::count(&g, &root[0], &root[1])? == 1)
}
pub(in crate::profile_certificate) fn equals_one(
    p: &Poly,
    f: &Poly,
    root: &[Q; 2],
) -> Result<bool> {
    zero_at(&sub(p, &[Q::one()]), f, root)
}
pub(in crate::profile_certificate) fn coordinates(
    a: &[[Q; 3]],
    b: &[[Q; 3]],
    domains: [[Q; 2]; 2],
    f: &Poly,
    t: &Poly,
    k: u8,
    root: &[Q; 2],
) -> Result<([Poly; 2], [[Poly; 2]; 2])> {
    if f.len() < 2 || k > 16 {
        return Err(refused("Invalid original algebraic event primitive"));
    }
    bounded(f)?;
    bounded(t)?;
    let s = first_parameter(t, k);
    let parameters = std::array::from_fn(|axis| {
        let local = if axis == 0 { &s } else { t };
        let mut p = local
            .iter()
            .map(|q| q * (&domains[axis][1] - &domains[axis][0]))
            .collect::<Poly>();
        if p.is_empty() {
            p.push(domains[axis][0].clone());
        } else {
            p[0] += &domains[axis][0];
        }
        trim(p)
    });
    for p in &parameters {
        bounded(p)?;
    }
    let left = [
        compose(&power(a, 0), &s, f)?,
        compose(&power(a, 1), &s, f)?,
        compose(&power(a, 2), &s, f)?,
    ];
    let right = [
        compose(&power(b, 0), t, f)?,
        compose(&power(b, 1), t, f)?,
        compose(&power(b, 2), t, f)?,
    ];
    if zero_at(&left[2], f, root)? || zero_at(&right[2], f, root)? {
        return Err(refused("Actual event coordinate denominator vanishes"));
    }
    for axis in 0..2 {
        let difference = modulo(
            &sub(
                &product(&left[axis], &right[2]),
                &product(&right[axis], &left[2]),
            ),
            f,
        )?;
        if !zero_at(&difference, f, root)? {
            return Err(refused("Original source event world coordinates disagree"));
        }
    }
    Ok((
        parameters,
        [
            [left[0].clone(), left[2].clone()],
            [left[1].clone(), left[2].clone()],
        ],
    ))
}

/// Sign at one ACTUAL isolated real root, never sign at its midpoint.
pub(in crate::profile_certificate) fn sign_at(
    p: &Poly,
    f: &Poly,
    root: &[Q; 2],
    max_refinements: usize,
) -> Result<i8> {
    if zero_at(p, f, root)? {
        return Ok(0);
    }
    let mut i = root.clone();
    for step in 0..=max_refinements {
        let r = p.iter().rev().fold([Q::zero(), Q::zero()], |v, c| {
            let q = [&v[0] * &i[0], &v[0] * &i[1], &v[1] * &i[0], &v[1] * &i[1]];
            [q.iter().min().unwrap() + c, q.iter().max().unwrap() + c]
        });
        if r[0] > Q::zero() {
            return Ok(1);
        }
        if r[1] < Q::zero() {
            return Ok(-1);
        }
        if step < max_refinements && i[0] != i[1] {
            let m = (&i[0] + &i[1]) / Q::from_integer(2.into());
            if at(f, &m).is_zero() {
                i = [m.clone(), m];
            } else if roots::count(f, &i[0], &m)? == 1 {
                i[1] = m;
            } else {
                i[0] = m;
            }
        }
    }
    Err(refused(
        "Original algebraic sign remains unproven within allowance",
    ))
}

/// Exact b-a=shift in one selected root extension; no rounded UV comparison.
pub(in crate::profile_certificate) fn translated_equal(
    an:&Poly,ad:&Poly,bn:&Poly,bd:&Poly,shift:&Q,f:&Poly,root:&[Q;2],
)->Result<bool>{
    if zero_at(ad,f,root)? || zero_at(bd,f,root)?{return Err(refused("Periodic transition coordinate denominator vanishes"));}
    let denominators=product(ad,bd);
    let difference=sub(&sub(&product(bn,ad),&product(an,bd)),&denominators.iter().map(|q|q*shift).collect::<Poly>());
    zero_at(&modulo(&difference,f)?,f,root)
}

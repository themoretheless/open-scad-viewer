//! Sturm isolation in Q(alpha)[t]. Zero divisors split the defining polynomial;
//! signs are decided at the particular isolated real root, never by floats.
use super::*;
fn sign(p: &Poly, f: &Poly, interval: &[Q; 2]) -> Result<i8> {
    if zero_at(p, f, interval)? {
        return Ok(0);
    }
    let mut i = interval.clone();
    for _ in 0..256 {
        let r = range(p, &i);
        if r[0] > Q::zero() {
            return Ok(1);
        }
        if r[1] < Q::zero() {
            return Ok(-1);
        }
        i = refine(f, &i)?;
    }
    Err(refused("Algebraic fiber sign budget exhausted"))
}
fn value(p: &Bi, t: &Q) -> Poly {
    p.iter().rev().fold(Vec::new(), |v, c| {
        sub(c, &v.iter().map(|x| -x * t).collect::<Poly>())
    })
}
fn variation(chain: &[Bi], f: &Poly, i: &[Q; 2], t: &Q) -> Result<usize> {
    let mut previous = 0;
    let mut changes = 0;
    for p in chain {
        let s = sign(&value(p, t), f, i)?;
        if s != 0 {
            changes += usize::from(previous != 0 && previous != s);
            previous = s
        }
    }
    Ok(changes)
}
fn isolate(p: &Bi, f: &Poly, i: &[Q; 2], width: &Q) -> MResult<Vec<[Q; 2]>> {
    let d = clean(
        p.iter()
            .enumerate()
            .skip(1)
            .map(|(j, r)| r.iter().map(|x| x * Q::from_integer(j.into())).collect())
            .collect(),
    );
    let g = mgcd(p, &d, f)?;
    let (p, remainder) = division(p, &g, f)?;
    if !remainder.is_empty() {
        return Err(refused("False radical fiber division").into());
    }
    if p.len() < 2 {
        return Ok(Vec::new());
    }
    let d = clean(
        p.iter()
            .enumerate()
            .skip(1)
            .map(|(j, r)| r.iter().map(|x| x * Q::from_integer(j.into())).collect())
            .collect(),
    );
    let mut chain = vec![p.clone(), d];
    loop {
        let n = chain.len();
        let r = division(&chain[n - 2], &chain[n - 1], f)?.1;
        if r.is_empty() {
            break;
        }
        chain.push(
            r.into_iter()
                .map(|r| r.into_iter().map(|x| -x).collect())
                .collect(),
        );
        if chain.len() > 64 {
            return Err(refused("Algebraic fiber Sturm budget exhausted").into());
        }
    }
    let roots = |a: &Q, b: &Q| -> Result<usize> {
        let va = variation(&chain, f, i, a)?;
        let vb = variation(&chain, f, i, b)?;
        va.checked_sub(vb + usize::from(sign(&value(&p, b), f, i)? == 0))
            .ok_or_else(|| refused("False algebraic fiber root census"))
    };
    let mut out = Vec::new();
    for t in [Q::zero(), Q::one()] {
        if sign(&value(&p, &t), f, i)? == 0 {
            out.push([t.clone(), t])
        }
    }
    let mut stack = vec![(Q::zero(), Q::one(), 0)];
    let mut work = 0;
    while let Some((a, b, depth)) = stack.pop() {
        work += 1;
        if work > 32768 || depth > 256 || out.len() > 64 {
            return Err(refused("Algebraic fiber isolation budget exhausted").into());
        }
        let count = roots(&a, &b)?;
        if count == 0 {
            continue;
        }
        if count == 1
            && &b - &a <= *width
            && sign(&value(&p, &a), f, i)? != 0
            && sign(&value(&p, &b), f, i)? != 0
        {
            out.push([a, b]);
            continue;
        }
        let m = (&a + &b) / Q::from_integer(2.into());
        if sign(&value(&p, &m), f, i)? == 0 {
            out.push([m.clone(), m.clone()])
        }
        stack.push((m.clone(), b, depth + 1));
        stack.push((a, m, depth + 1));
    }
    out.sort();
    out.dedup();
    Ok(out)
}
pub(in crate::profile_certificate) fn isolate_at(
    p: &Bi,
    f: &Poly,
    i: &[Q; 2],
    width: &Q,
) -> Result<Vec<[Q; 2]>> {
    let mut f = f.clone();
    for _ in 0..128 {
        let attempt = (|| -> MResult<Vec<[Q; 2]>> {
            let p = reduce(p, &f)?;
            if p.is_empty() {
                return Err(refused("Whole vertical fiber needs an explicit component").into());
            }
            // Every surviving leading coefficient is a unit on this factor.
            for r in &p {
                if !r.is_empty() {
                    inv(r, &f)?;
                }
            }
            isolate(&p, &f, i, width)
        })();
        match attempt {
            Ok(r) => return Ok(r),
            Err(ModularError::Geometry(e)) => return Err(e),
            Err(ModularError::Split(g)) => {
                if g.len() < 2 || g.len() >= f.len() {
                    return Err(refused("Improper algebraic fiber factor split"));
                }
                f = if count(&g, &i[0], &i[1])? == 1 {
                    g
                } else {
                    monic(quotient(&f, &g)?)
                };
            }
        }
    }
    Err(refused("Algebraic fiber factor budget exhausted"))
}
pub(in crate::profile_certificate) fn zero(p: &Poly, f: &Poly, i: &[Q; 2]) -> Result<bool> {
    zero_at(p, f, i)
}
pub(in crate::profile_certificate) fn refine_root(f: &Poly, i: &[Q; 2]) -> Result<[Q; 2]> {
    refine(f, i)
}

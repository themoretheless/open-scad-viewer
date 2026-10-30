use super::*;
pub(super) type Bi = Vec<Vec<Q>>;
fn trim(p: &mut Bi) {
    for row in p.iter_mut() {
        while row.last().is_some_and(Q::is_zero) {
            row.pop();
        }
    }
    while p.last().is_some_and(Vec::is_empty) {
        p.pop();
    }
}
pub(super) fn encode(p: &Bi) -> DomainPolynomial {
    p.iter()
        .map(|r| r.iter().map(roots::ratio).collect())
        .collect()
}
pub(super) fn decode(p: &DomainPolynomial) -> Result<Bi> {
    if p.len() > 9 || p.iter().any(|r| r.len() > 9) {
        return Err(fail("algebraic domain cut degree budget exceeded"));
    }
    let mut out = p
        .iter()
        .map(|r| r.iter().map(roots::decode).collect::<Result<Vec<_>>>())
        .collect::<Result<Bi>>()?;
    trim(&mut out);
    if encode(&out) != *p {
        return Err(fail("algebraic domain polynomial is not canonical"));
    }
    Ok(out)
}
fn product(a: &Bi, b: &Bi, work: &mut Work) -> Result<Bi> {
    if a.is_empty() || b.is_empty() {
        return Ok(Vec::new());
    }
    let n = a.len() + b.len() - 1;
    let m = a.iter().map(Vec::len).max().unwrap() + b.iter().map(Vec::len).max().unwrap() - 1;
    if n > 17 || m > 17 {
        return Err(fail("joint algebraic domain degree budget exceeded"));
    }
    let mut out = vec![vec![Q::zero(); m]; n];
    for (i, row) in a.iter().enumerate() {
        for (j, x) in row.iter().enumerate() {
            for (k, s) in b.iter().enumerate() {
                for (l, y) in s.iter().enumerate() {
                    work.charge(1)?;
                    out[i + k][j + l] += x * y;
                }
            }
        }
    }
    for q in out.iter().flatten() {
        if q.numer().bits() > 8192 || q.denom().bits() > 8192 {
            return Err(fail("joint algebraic domain coefficient budget exceeded"));
        }
    }
    trim(&mut out);
    Ok(out)
}
pub(super) fn joint(cuts: &[Bi]) -> Result<Bi> {
    let mut work = Work::default();
    // Include v=0, v=1, and v=1-u in the SAME arrangement, so every
    // boundary stratum and every crossing/tangency is present in its census.
    let mut p = vec![vec![Q::zero()], vec![Q::one()]];
    p = product(&p, &vec![vec![Q::one()], vec![-Q::one()]], &mut work)?;
    p = product(
        &p,
        &vec![vec![Q::one(), -Q::one()], vec![-Q::one()]],
        &mut work,
    )?;
    for cut in cuts {
        // An identically zero cut labels the entire chart zero; it introduces
        // no further boundary. Nonzero constants also have empty zero sets.
        if cut.is_empty() || (cut.len() == 1 && cut[0].len() == 1) {
            continue;
        }
        p = product(&p, cut, &mut work)?;
    }
    Ok(p)
}
pub(super) fn at(p: &Bi, u: &Q, v: &Q) -> Q {
    p.iter().rev().fold(Q::zero(), |value, row| {
        value * v + row.iter().rev().fold(Q::zero(), |value, c| value * u + c)
    })
}

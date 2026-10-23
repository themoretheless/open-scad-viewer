//! Exact changes of chart; these preserve images, never repair source cracks.
use super::*;

pub(super) fn reverse(h: &Net) -> Net {
    let n = h.len() - 1;
    (0..=n)
        .map(|i| (0..=n - i).map(|j| h[j][i].clone()).collect())
        .collect()
}
/// Rotate a tensor square counterclockwise, preserving orientation.
pub(super) fn rotate(h: &sb::Net) -> sb::Net {
    let n = h.len() - 1;
    (0..h[0].len())
        .map(|i| (0..=n).map(|j| h[n - j][i].clone()).collect())
        .collect()
}
fn root(q: &Q, n: usize) -> Option<Q> {
    if *q <= Q::zero() || n == 0 {
        return None;
    }
    let a = q.numer().nth_root(n as u32);
    let b = q.denom().nth_root(n as u32);
    (a.pow(n as u32) == *q.numer() && b.pow(n as u32) == *q.denom()).then(|| Q::new(a, b))
}
fn choose(n: usize, k: usize) -> Q {
    (0..k).fold(Q::one(), |q, i| {
        q * integer((n - i) as i64) / integer((i + 1) as i64)
    })
}
pub(super) fn check(h: &Net) -> Result<()> {
    for p in h.iter().flatten() {
        if p[3] <= Q::zero()
            || p.iter()
                .any(|q| q.numer().bits() > 8192 || q.denom().bits() > 8192)
        {
            return Err(fail("atlas transformed chart weight/bit budget exceeded"));
        }
        guard(&sb::point(p))?;
    }
    Ok(())
}
/// Blow down a rectangle whose v=0 edge is a point. In homogeneous power
/// coefficients substitute x=v, y=u*v. Divisibility j>=i is an exact identity,
/// and gives a regular candidate on 0<=y<=x<=1. The caller must still prove
/// the candidate's Jacobian, links and separation. No singular cell is dropped.
pub(super) fn pole(h: &sb::Net, work: &mut Work) -> Result<Option<Net>> {
    let (n, m) = (h.len() - 1, h[0].len() - 1);
    let p = sb::point(&h[0][0]);
    if h.iter().any(|r| sb::point(&r[0]) != p) {
        return Ok(None);
    }
    let mut power = vec![vec![std::array::from_fn::<_, 4, _>(|_| Q::zero()); m + 1]; n + 1];
    for (i, row) in h.iter().enumerate() {
        for (j, p) in row.iter().enumerate() {
            for a in i..=n {
                for b in j..=m {
                    work.charge(1)?;
                    let f = choose(n, i)
                        * choose(n - i, a - i)
                        * choose(m, j)
                        * choose(m - j, b - j)
                        * integer(if (a - i + b - j) % 2 == 0 { 1 } else { -1 });
                    for k in 0..4 {
                        power[a][b][k] += &p[k] * &f;
                    }
                }
            }
        }
    }
    for (i, row) in power.iter().enumerate() {
        if row.iter().take(i).any(|p| p.iter().any(|q| !q.is_zero())) {
            return Ok(None);
        }
    }
    // Coordinates x=lambda1+lambda2, y=lambda2. Thus u=y/x,
    // v=x maps vertices to pole, (u=0,v=1), (u=1,v=1).
    let d = m;
    let mut polynomial = vec![vec![std::array::from_fn::<_, 4, _>(|_| Q::zero()); d + 1]; d + 1];
    for (i, row) in power.iter().enumerate() {
        for (j, p) in row.iter().enumerate().skip(i) {
            for a in 0..=j - i {
                let f = choose(j - i, a);
                for k in 0..4 {
                    polynomial[a][j - a][k] += &p[k] * &f;
                }
            }
        }
    }
    let mut out = (0..=d)
        .map(|i| vec![std::array::from_fn(|_| Q::zero()); d + 1 - i])
        .collect::<Net>();
    for (i, row) in out.iter_mut().enumerate() {
        for (j, p) in row.iter_mut().enumerate() {
            for a in 0..=i {
                for b in 0..=j {
                    if a + b > d {
                        continue;
                    }
                    work.charge(1)?;
                    let f = choose(i, a) * choose(j, b) / (choose(d, a + b) * choose(a + b, a));
                    for k in 0..4 {
                        p[k] += &polynomial[a][b][k] * &f;
                    }
                }
            }
        }
    }
    check(&out)?;
    Ok(Some(out))
}



fn exponent(q: &Q, p: &Q) -> Result<Q> {
    let n = p
        .numer()
        .to_i32()
        .filter(|n| n.unsigned_abs() <= 64)
        .ok_or_else(|| fail("atlas clock exponent budget exceeded"))?;
    let d = p
        .denom()
        .to_usize()
        .filter(|d| *d <= 64)
        .ok_or_else(|| fail("atlas clock root budget exceeded"))?;
    let r = root(q, d)
        .ok_or_else(|| fail("atlas clock requires an algebraic scale"))?
        .pow(n);
    if r.numer().bits() > 8192 || r.denom().bits() > 8192 {
        return Err(fail("atlas clock coefficient budget exceeded"));
    }
    Ok(r)
}
fn clock(a: &[sb::H], b: &[sb::H]) -> Result<Q> {
    if sb::same(a, b) {
        return Ok(Q::one());
    }
    fn jet(h: &[sb::H]) -> Option<(usize, P)> {
        let n = h.len() - 1;
        let mut differences = h.to_vec();
        for k in 1..=n {
            differences = differences
                .windows(2)
                .map(|p| std::array::from_fn(|i| &p[1][i] - &p[0][i]))
                .collect();
            let c = choose(n, k);
            let v = std::array::from_fn(|i| {
                &c * (&differences[0][i] * &h[0][3] - &h[0][i] * &differences[0][3])
                    / (&h[0][3] * &h[0][3])
            });
            if !zero(&v) {
                return Some((k, v));
            }
        }
        None
    }
    let (ka, da) = jet(a).ok_or_else(|| fail("atlas seam is constant"))?;
    let (kb, db) = jet(b).ok_or_else(|| fail("atlas seam is constant"))?;
    if ka != kb {
        return Err(fail("atlas seam endpoint orders disagree"));
    }
    let k = (0..3).find(|k| !da[*k].is_zero()).unwrap();
    let ratio = &db[k] / &da[k];
    if ratio <= Q::zero() || (0..3).any(|k| db[k] != &ratio * &da[k]) {
        return Err(fail("atlas seam endpoint jets have no positive transport"));
    }
    let r =
        root(&ratio, ka).ok_or_else(|| fail("atlas seam endpoint jet needs an algebraic clock"))?;
    if r.numer().bits() > 8192 || r.denom().bits() > 8192 {
        return Err(fail("atlas seam clock bit budget exceeded"));
    }
    let scaled = a
        .iter()
        .enumerate()
        .map(|(i, p)| std::array::from_fn(|k| &p[k] * r.pow(i as i32)))
        .collect::<Vec<_>>();
    if !sb::same(&scaled, b) {
        return Err(fail("atlas seam is not a proved projective correspondence"));
    }
    Ok(r)
}
/// Solve simultaneous seam clocks, including cycle consistency. A transport on
/// one edge alone does not define a compatible triangular chart. This exact
/// multiplicative linear system determines positive vertex scales for every
/// patch; free scales are one. Unproved roots or inconsistent cycles refuse.
pub(super) fn synchronize(patches: &mut [Patch], work: &mut Work) -> Result<()> {
    if patches.len() > 128 {
        return Err(fail("atlas seam transport chart budget exceeded"));
    }
    let mut edges = BTreeMap::<(u32, u32), Vec<(usize, usize, usize, Vec<sb::H>)>>::new();
    for (i, p) in patches.iter().enumerate() {
        for (k, mut h) in bernstein::edges(&p.h).into_iter().enumerate() {
            let (a, b) = (p.ids[k], p.ids[(k + 1) % 3]);
            let (x, y) = if a < b {
                (k, (k + 1) % 3)
            } else {
                h.reverse();
                ((k + 1) % 3, k)
            };
            edges
                .entry((a.min(b), a.max(b)))
                .or_default()
                .push((i, x, y, h));
        }
    }
    type Row = (BTreeMap<usize, Q>, Q);
    let mut basis = BTreeMap::<usize, Row>::new();
    for e in edges.values() {
        if e.len() != 2 {
            return Err(fail("atlas seam incidence is not two"));
        }
        let (a, x, y, ha) = &e[0];
        let (b, u, v, hb) = &e[1];
        let mut rhs = clock(ha, hb)?;
        let mut row = BTreeMap::<usize, Q>::new();
        for (p, k, sign) in [(*a, *y, 1), (*a, *x, -1), (*b, *v, -1), (*b, *u, 1)] {
            if k != 0 {
                *row.entry(2 * p + k - 1).or_default() += integer(sign);
            }
        }
        row.retain(|_, q| !q.is_zero());
        loop {
            let Some((&pivot, scale)) = row.first_key_value() else {
                if rhs != Q::one() {
                    return Err(fail("atlas seam clock cycle is inconsistent"));
                }
                break;
            };
            let scale = scale.clone();
            if let Some((old, r)) = basis.get(&pivot) {
                rhs /= exponent(r, &scale)?;
                for (k, q) in old {
                    work.charge(1)?;
                    *row.entry(*k).or_default() -= &scale * q;
                }
                row.retain(|_, q| !q.is_zero());
            } else {
                rhs = exponent(&rhs, &(Q::one() / &scale))?;
                for q in row.values_mut() {
                    *q /= &scale;
                }
                basis.insert(pivot, (row, rhs));
                break;
            }
        }
    }
    let mut scales = vec![Q::one(); 2 * patches.len()];
    for (&pivot, (row, r)) in basis.iter().rev() {
        let mut value = r.clone();
        for (&j, q) in row {
            if j != pivot {
                work.charge(1)?;
                value /= exponent(&scales[j], q)?;
            }
        }
        scales[pivot] = value;
    }
    for (i, p) in patches.iter_mut().enumerate() {
        let n = p.h.len() - 1;
        for (j, row) in p.h.iter_mut().enumerate() {
            for (k, h) in row.iter_mut().enumerate() {
                work.charge(1)?;
                let factor = scales[2 * i].pow(j as i32) * scales[2 * i + 1].pow(k as i32);
                for q in h {
                    *q *= &factor;
                }
            }
        }
        debug_assert_eq!(p.h.len() - 1, n);
        check(&p.h)?;
        p.derivatives = bernstein::derivatives(&p.h, work)?;
        p.motion = bernstein::homotopy(&p.h, work)?;
    }
    Ok(())
}



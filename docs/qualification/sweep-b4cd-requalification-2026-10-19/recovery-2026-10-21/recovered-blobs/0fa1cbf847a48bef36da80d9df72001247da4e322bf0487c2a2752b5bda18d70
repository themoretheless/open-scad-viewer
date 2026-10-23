//! Complete exact rational ray crossings, including stationary roots/endpoints.
use super::super::algebraic_intersections::{ExactRatio, decode_ratio, ratio};
use super::*;
use algebraic_events as roots;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RayRootWitness {
    pub interval: [ExactRatio; 2],
    /// Sign of X-px*W at the actual algebraic root, never a midpoint test.
    pub sign_x: i8,
    /// Half-open upward/downward contribution before selecting x>px.
    pub jump: i8,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RayCellWitness {
    pub roots: Vec<RayRootWitness>,
}
fn sign(q: &Q) -> i8 {
    if q > &Q::zero() {
        1
    } else if q < &Q::zero() {
        -1
    } else {
        0
    }
}
fn polynomials(net: &[[Q; 3]], point: &[Q; 2]) -> [Poly; 2] {
    let w = power(net, 2);
    std::array::from_fn(|k| {
        sub(
            &power(net, k),
            &w.iter().map(|v| v * &point[k]).collect::<Vec<_>>(),
        )
    })
}
fn has_root(p: &Poly) -> Result<bool> {
    Ok(p.is_empty()
        || at(p, &Q::zero()).is_zero()
        || at(p, &Q::one()).is_zero()
        || roots::count(p, &Q::zero(), &Q::one())? > 0)
}
fn range(p: &Poly, i: &[Q; 2]) -> [Q; 2] {
    p.iter().rev().fold([Q::zero(), Q::zero()], |v, c| {
        let x = [&v[0] * &i[0], &v[0] * &i[1], &v[1] * &i[0], &v[1] * &i[1]];
        [x.iter().min().unwrap() + c, x.iter().max().unwrap() + c]
    })
}
fn x_sign(x: &Poly, i: &[Q; 2]) -> Option<i8> {
    let r = range(x, i);
    if r[0] > Q::zero() {
        Some(1)
    } else if r[1] < Q::zero() {
        Some(-1)
    } else {
        None
    }
}
fn jump(g: &Poly, i: &[Q; 2]) -> Result<i8> {
    if i[0] != i[1] {
        let a = sign(&at(g, &i[0]));
        let b = sign(&at(g, &i[1]));
        if a == 0 || b == 0 {
            return Err(refused("Ray root interval endpoint is a root"));
        }
        return Ok(if a == b { 0 } else { b });
    }
    let q = &i[0];
    let mut d = g.clone();
    let mut order = 0;
    while at(&d, q).is_zero() {
        d = derivative(&d);
        order += 1;
        if d.is_empty() {
            return Err(refused("Ray root has no finite nonzero germ"));
        }
    }
    let right = sign(&at(&d, q));
    Ok(if q == &Q::zero() {
        i8::from(right > 0)
    } else if q == &Q::one() {
        let left = if order % 2 == 0 { right } else { -right };
        -i8::from(left > 0)
    } else if order % 2 == 0 {
        0
    } else {
        right
    })
}
fn prepare(net: &[[Q; 3]], p: &[Q; 2]) -> Result<[Poly; 2]> {
    let [x, g] = polynomials(net, p);
    if g.is_empty() {
        if has_root(&x)? {
            return Err(refused("Winding query lies on a horizontal source image"));
        }
    } else if has_root(&gcd(&x, &g)?)? {
        return Err(refused("Winding query lies on the actual source curve"));
    }
    Ok([x, g])
}
pub(in crate::profile_certificate) fn produce(
    net: &[[Q; 3]],
    p: &[Q; 2],
) -> Result<RayCellWitness> {
    let [x, g] = prepare(net, p)?;
    if g.is_empty() {
        return Ok(RayCellWitness { roots: vec![] });
    }
    let mut witnesses = Vec::new();
    for mut i in roots::isolates(&g, Q::zero(), Q::one(), &Q::new(1.into(), 1024.into()))? {
        let mut sx = None;
        for _ in 0..=192 {
            if let Some(s) = x_sign(&x, &i) {
                sx = Some(s);
                break;
            }
            if i[0] == i[1] {
                break;
            }
            let m = (&i[0] + &i[1]) / Q::from_integer(2.into());
            if at(&g, &m).is_zero() {
                i = [m.clone(), m];
            } else if roots::count(&g, &i[0], &m)? == 1 {
                i[1] = m;
            } else {
                i[0] = m;
            }
        }
        witnesses.push(RayRootWitness {
            sign_x: sx.ok_or_else(|| refused("Exact ray side isolation allowance exhausted"))?,
            jump: jump(&g, &i)?,
            interval: i.map(|q| ratio(&q)),
        });
    }
    let c = RayCellWitness { roots: witnesses };
    verify(net, p, &c)?;
    Ok(c)
}
pub(in crate::profile_certificate) fn verify(
    net: &[[Q; 3]],
    p: &[Q; 2],
    c: &RayCellWitness,
) -> Result<i64> {
    let [x, g] = prepare(net, p)?;
    if g.is_empty() {
        if !c.roots.is_empty() {
            return Err(refused("Horizontal source has spurious ray roots"));
        }
        return Ok(0);
    }
    if c.roots.len() > 8 {
        return Err(refused("Ray root witness count exceeds source degree"));
    }
    let expected = roots::count(&g, &Q::zero(), &Q::one())?
        + usize::from(at(&g, &Q::zero()).is_zero())
        + usize::from(at(&g, &Q::one()).is_zero());
    if expected != c.roots.len() {
        return Err(refused("Ray census omits or repeats a source root"));
    }
    let mut previous: Option<[Q; 2]> = None;
    let mut value = 0;
    for r in &c.roots {
        let i = [decode_ratio(&r.interval[0])?, decode_ratio(&r.interval[1])?];
        if i[0] < Q::zero()
            || i[1] > Q::one()
            || i[0] > i[1]
            || previous
                .as_ref()
                .is_some_and(|a| a[1] > i[0] || a[1] == i[0] && at(&g, &i[0]).is_zero())
            || (if i[0] == i[1] {
                !at(&g, &i[0]).is_zero()
            } else {
                at(&g, &i[0]).is_zero()
                    || at(&g, &i[1]).is_zero()
                    || roots::count(&g, &i[0], &i[1])? != 1
            })
        {
            return Err(refused(
                "Exact ray isolating interval or complete ordering is false",
            ));
        }
        let sx =
            x_sign(&x, &i).ok_or_else(|| refused("Ray root side has no exact interval proof"))?;
        let j = jump(&g, &i)?;
        if r.sign_x != sx || r.jump != j {
            return Err(refused(
                "Ray root side or stationary/end germ claim is false",
            ));
        }
        if sx > 0 {
            value += i64::from(j);
        }
        previous = Some(i);
    }
    Ok(value)
}

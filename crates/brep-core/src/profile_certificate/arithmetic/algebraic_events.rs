//! Exact rational-univariate representation of finite bivariate intersections.
//! Unit-checked modular Euclid splits zero divisors, preserving every fiber.
use super::bivariate::{Bi, intersection_polynomials, resultant};
use super::*;
pub(in super::super) mod fibers;
#[derive(Clone, Debug)]
pub(in super::super) struct Factor {
    pub polynomial: Poly,
    pub map: Option<Poly>,
    pub roots: Vec<Root>,
}
#[derive(Clone, Debug)]
pub(in super::super) struct Root {
    pub interval: [Q; 2],
    pub kind: String,
}
#[derive(Clone, Debug)]
pub(in super::super) struct Event {
    pub first: [Q; 2],
    pub second: [Q; 2],
    pub singular: bool,
}
#[derive(Debug)]
enum ModularError {
    Split(Poly),
    Geometry(Error),
}
impl From<Error> for ModularError {
    fn from(e: Error) -> Self {
        Self::Geometry(e)
    }
}
type MResult<T> = std::result::Result<T, ModularError>;
fn bound(p: &Poly) -> Result<()> {
    if p.len() > 513
        || p.iter()
            .any(|q| q.numer().bits() > 16384 || q.denom().bits() > 16384)
    {
        return Err(refused("Algebraic event arithmetic budget exceeded"));
    }
    Ok(())
}
fn monic(mut p: Poly) -> Poly {
    if let Some(v) = p.last().cloned() {
        for q in &mut p {
            *q /= &v;
        }
    }
    p
}
fn modulo(p: &Poly, f: &Poly) -> Result<Poly> {
    bound(p)?;
    Ok(divide(p, f)?.1)
}
fn mul(a: &Poly, b: &Poly, f: &Poly) -> Result<Poly> {
    modulo(&product(a, b), f)
}
fn inv(a: &Poly, f: &Poly) -> MResult<Poly> {
    let (mut x, mut y) = (f.clone(), modulo(a, f)?);
    let (mut u, mut v) = (Vec::new(), vec![Q::one()]);
    while !y.is_empty() {
        let (q, r) = divide(&x, &y)?;
        let next = modulo(&sub(&u, &product(&q, &v)), f)?;
        x = y;
        y = r;
        u = v;
        v = next;
    }
    if x.len() != 1 {
        return Err(ModularError::Split(monic(x)));
    }
    let scale = x[0].clone();
    for q in &mut u {
        *q /= &scale;
    }
    Ok(u)
}
fn clean(mut p: Bi) -> Bi {
    while p.last().is_some_and(Vec::is_empty) {
        p.pop();
    }
    p
}
fn reduce(p: &Bi, f: &Poly) -> Result<Bi> {
    Ok(clean(
        p.iter().map(|p| modulo(p, f)).collect::<Result<_>>()?,
    ))
}
fn division(a: &Bi, b: &Bi, f: &Poly) -> MResult<(Bi, Bi)> {
    let mut r = reduce(a, f)?;
    let b = reduce(b, f)?;
    if b.is_empty() {
        return Err(refused("Zero modular event divisor").into());
    }
    let inverse = inv(b.last().unwrap(), f)?;
    let mut out = vec![Vec::new(); r.len().saturating_sub(b.len()) + 1];
    let mut work = 0;
    while !r.is_empty() && r.len() >= b.len() {
        work += 1;
        if work > 33 {
            return Err(refused("Modular event division budget exceeded").into());
        }
        let k = r.len() - b.len();
        let c = mul(r.last().unwrap(), &inverse, f)?;
        out[k] = c.clone();
        for (i, p) in b.iter().enumerate() {
            r[i + k] = modulo(&sub(&r[i + k], &mul(&c, p, f)?), f)?;
        }
        r = clean(r);
    }
    Ok((clean(out), r))
}
fn mgcd(a: &Bi, b: &Bi, f: &Poly) -> MResult<Bi> {
    let (mut a, mut b) = (reduce(a, f)?, reduce(b, f)?);
    while !b.is_empty() {
        let r = division(&a, &b, f)?.1;
        a = b;
        b = r;
    }
    if a.is_empty() {
        return Err(refused("Nonisolated algebraic intersection fiber").into());
    }
    let scale = inv(a.last().unwrap(), f)?;
    Ok(a.iter().map(|p| mul(p, &scale, f)).collect::<Result<_>>()?)
}
fn map(e: &[Bi; 2], f: &Poly) -> MResult<Option<Poly>> {
    let g = mgcd(&e[0], &e[1], f)?;
    if g.len() == 1 {
        return Ok(None);
    }
    let d: Bi = g
        .iter()
        .enumerate()
        .skip(1)
        .map(|(i, p)| p.iter().map(|q| q * Q::from_integer(i.into())).collect())
        .collect();
    let repeated = mgcd(&g, &d, f)?;
    let (radical, remainder) = division(&g, &repeated, f)?;
    if !remainder.is_empty() || radical.len() != 2 {
        return Err(refused("Algebraic fibers need a separating parameter projection").into());
    }
    let inverse = inv(&radical[1], f)?;
    let t = mul(&radical[0].iter().map(|q| -q).collect(), &inverse, f)?;
    // Independent substitution identities supplement the modular root census.
    for p in e {
        let mut v = Vec::new();
        for row in p.iter().rev() {
            v = modulo(
                &sub(
                    &mul(&v, &t, f)?,
                    &row.iter().map(|q| -q).collect::<Vec<_>>(),
                ),
                f,
            )?;
        }
        if !v.is_empty() {
            return Err(refused("Algebraic event substitution identity failed").into());
        }
    }
    Ok(Some(t))
}
fn shear(p: &Bi, k: u8) -> Result<Bi> {
    let mut out: Bi = vec![Vec::new(); 33];
    for (j, row) in p.iter().enumerate() {
        for (i, c) in row.iter().enumerate() {
            for h in 0..=i {
                let t = j + h;
                if t >= out.len() {
                    return Err(refused("Algebraic projection degree exceeded"));
                }
                let length = out[t].len().max(i - h + 1);
                out[t].resize(length, Q::zero());
                out[t][i - h] += c
                    * Q::from_integer(choose(i, h).into())
                    * Q::from_integer((-(k as i64)).into()).pow(h as i32);
            }
        }
    }
    for row in &mut out {
        *row = trim(std::mem::take(row));
        bound(row)?;
    }
    Ok(clean(out))
}
fn eliminant(e: &[Bi; 2]) -> Result<Poly> {
    let r = resultant(&e[0], &e[1])?;
    if r.is_empty() {
        return Err(refused("Algebraic events have a common curve component"));
    }
    let squarefree = quotient(&r, &gcd(&r, &derivative(&r))?)?;
    bound(&squarefree)?;
    Ok(monic(squarefree))
}
fn affine(p: &Poly, a: &Q, b: &Q) -> Poly {
    p.iter().rev().fold(Vec::new(), |v, q| {
        let mut v = product(&v, &[a.clone(), b - a]);
        if v.is_empty() {
            v.push(q.clone());
        } else {
            v[0] += q;
        }
        trim(v)
    })
}
pub(in super::super) fn count(p: &Poly, a: &Q, b: &Q) -> Result<usize> {
    if a == b {
        return Ok(usize::from(at(p, a).is_zero()));
    }
    interior_roots(&affine(p, a, b))
}
fn refine(p: &Poly, i: &[Q; 2]) -> Result<[Q; 2]> {
    if i[0] == i[1] {
        return Ok(i.clone());
    }
    let mid = (&i[0] + &i[1]) / Q::from_integer(2.into());
    if at(p, &mid).is_zero() {
        return Ok([mid.clone(), mid]);
    }
    if count(p, &i[0], &mid)? == 1 {
        Ok([i[0].clone(), mid])
    } else {
        Ok([mid, i[1].clone()])
    }
}
pub(in super::super) fn isolates(p: &Poly, lo: Q, hi: Q, width: &Q) -> Result<Vec<[Q; 2]>> {
    let mut out = Vec::new();
    for x in [&lo, &hi] {
        if at(p, x).is_zero() {
            out.push([x.clone(), x.clone()]);
        }
    }
    let mut stack = vec![(lo, hi, 0u16)];
    let mut work = 0;
    while let Some((a, b, d)) = stack.pop() {
        work += 1;
        if work > 32768 || d > 192 || out.len() > 256 {
            return Err(refused("Algebraic root isolation budget exceeded"));
        }
        let roots = count(p, &a, &b)?;
        if roots == 0 {
            continue;
        }
        if roots == 1 && &b - &a <= *width && !at(p, &a).is_zero() && !at(p, &b).is_zero() {
            out.push([a, b]);
            continue;
        }
        let mid = (&a + &b) / Q::from_integer(2.into());
        if at(p, &mid).is_zero() {
            out.push([mid.clone(), mid.clone()]);
        }
        stack.push((mid.clone(), b, d + 1));
        stack.push((a, mid, d + 1));
    }
    out.sort_by(|a, b| a[0].cmp(&b[0]));
    Ok(out)
}
fn range(p: &Poly, i: &[Q; 2]) -> [Q; 2] {
    p.iter().rev().fold([Q::zero(), Q::zero()], |v, c| {
        let products = [&v[0] * &i[0], &v[0] * &i[1], &v[1] * &i[0], &v[1] * &i[1]];
        [
            products.iter().min().unwrap() + c,
            products.iter().max().unwrap() + c,
        ]
    })
}
fn zero_at(p: &Poly, f: &Poly, i: &[Q; 2]) -> Result<bool> {
    if p.is_empty() {
        return Ok(true);
    }
    let g = gcd(p, f)?;
    if g.len() < 2 {
        return Ok(false);
    }
    Ok(count(&g, &i[0], &i[1])? == 1)
}
fn coordinate(p: &Poly, f: &Poly, i: &[Q; 2]) -> Result<[Q; 2]> {
    if zero_at(p, f, i)? {
        return Ok([Q::zero(), Q::zero()]);
    }
    if zero_at(&sub(p, &[Q::one()]), f, i)? {
        return Ok([Q::one(), Q::one()]);
    }
    Ok(range(p, i))
}
fn classify(
    f: &Poly,
    t: &Poly,
    i: &[Q; 2],
    k: u8,
    self_pair: bool,
    width: &Q,
) -> Result<Option<(String, Event)>> {
    let s = sub(
        &[Q::zero(), Q::one()],
        &t.iter()
            .map(|q| q * Q::from_integer(k.into()))
            .collect::<Poly>(),
    );
    let a = coordinate(&s, f, i)?;
    let b = coordinate(t, f, i)?;
    let event = Event {
        first: a.clone(),
        second: b.clone(),
        singular: false,
    };
    if a[1] < Q::zero() || a[0] > Q::one() || b[1] < Q::zero() || b[0] > Q::one() {
        return Ok(Some(("outside".into(), event)));
    }
    if a[0] < Q::zero()
        || a[1] > Q::one()
        || b[0] < Q::zero()
        || b[1] > Q::one()
        || &a[1] - &a[0] > *width
        || &b[1] - &b[0] > *width
    {
        return Ok(None);
    }
    if self_pair {
        if zero_at(&sub(&s, t), f, i)? {
            return Ok(Some((
                "diagonal".into(),
                Event {
                    singular: true,
                    ..event
                },
            )));
        }
        if a[0] > b[1] {
            return Ok(Some(("reverse".into(), event)));
        }
        if a[1] >= b[0] {
            return Ok(None);
        }
    }
    Ok(Some(("event".into(), event)))
}
pub(in super::super) fn produce(
    a: &[[Q; 3]],
    b: &[[Q; 3]],
    self_pair: bool,
    width: &Q,
) -> Result<(u8, Vec<Factor>)> {
    produce_system(&intersection_polynomials(a,b,self_pair)?,self_pair,width)
}
pub(in super::super) fn produce_system(system:&[Bi;2],self_pair:bool,width:&Q)->Result<(u8,Vec<Factor>)>{
    if system.iter().any(|p|p.len()==1&&p[0].len()==1&&!p[0][0].is_zero()){return Ok((0,Vec::new()))}
    for k in [0, 1, 2, 3, 5, 7, 11] {
        let attempt = (|| -> Result<Vec<Factor>> {
            let e = [shear(&system[0],k)?,shear(&system[1],k)?];
            let r = eliminant(&e)?;
            if r.len() == 1 {
                return Ok(Vec::new());
            }
            let mut queue = vec![r];
            let mut factors = Vec::new();
            while let Some(f) = queue.pop() {
                if factors.len() + queue.len() > 256 {
                    return Err(refused("Algebraic factor census exceeded"));
                }
                match map(&e, &f) {
                    Err(ModularError::Split(g)) => {
                        if g.len() < 2 || g.len() >= f.len() {
                            return Err(refused("Algebraic factor split is not proper"));
                        }
                        queue.push(monic(quotient(&f, &g)?));
                        queue.push(g);
                    }
                    Err(ModularError::Geometry(e)) => return Err(e),
                    Ok(map) => {
                        let mut roots = Vec::new();
                        if let Some(t) = &map {
                            for mut interval in isolates(
                                &f,
                                Q::zero(),
                                Q::from_integer((k as i64 + 1).into()),
                                width,
                            )? {
                                let mut found = None;
                                for _ in 0..192 {
                                    if let Some((kind, _)) =
                                        classify(&f, t, &interval, k, self_pair, width)?
                                    {
                                        found = Some(kind);
                                        break;
                                    }
                                    interval = refine(&f, &interval)?;
                                }
                                roots.push(Root {
                                    interval,
                                    kind: found.ok_or_else(|| {
                                        refused(
                                            "Algebraic event domain classification is unresolved",
                                        )
                                    })?,
                                });
                            }
                        }
                        factors.push(Factor {
                            polynomial: f,
                            map,
                            roots,
                        });
                    }
                }
            }
            Ok(factors)
        })();
        if let Ok(factors) = attempt {
            verify_system(system,self_pair,width,k,&factors)?;
            return Ok((k, factors));
        }
    }
    Err(refused(
        "No complete algebraic intersection representation within projection budgets",
    ))
}
pub(in super::super) fn verify(
    a: &[[Q; 3]],
    b: &[[Q; 3]],
    self_pair: bool,
    width: &Q,
    k: u8,
    factors: &[Factor],
) -> Result<Vec<Event>> {
    verify_system(&intersection_polynomials(a,b,self_pair)?,self_pair,width,k,factors)
}
pub(in super::super) fn verify_system(system:&[Bi;2],self_pair:bool,width:&Q,k:u8,factors:&[Factor])->Result<Vec<Event>>{
    if system.iter().any(|p|p.len()==1&&p[0].len()==1&&!p[0][0].is_zero()){if k!=0||!factors.is_empty(){return Err(refused("Unit residual system has extraneous events"))}return Ok(Vec::new())}
    if ![0, 1, 2, 3, 5, 7, 11].contains(&k) || factors.len() > 256 {
        return Err(refused("Invalid algebraic projection or factor count"));
    }
    let e = [shear(&system[0],k)?,shear(&system[1],k)?];
    let r = eliminant(&e)?;
    let mut product_all = vec![Q::one()];
    let mut events = Vec::new();
    for (n, f) in factors.iter().enumerate() {
        bound(&f.polynomial)?;
        if f.polynomial.len() < 2
            || monic(f.polynomial.clone()) != f.polynomial
            || gcd(&f.polynomial, &derivative(&f.polynomial))?.len() != 1
        {
            return Err(refused("Algebraic factor is not monic squarefree"));
        }
        for other in &factors[..n] {
            if gcd(&f.polynomial, &other.polynomial)?.len() != 1 {
                return Err(refused("Algebraic factors are not disjoint"));
            }
        }
        product_all = product(&product_all, &f.polynomial);
        bound(&product_all)?;
        let actual = map(&e, &f.polynomial)
            .map_err(|_| refused("Algebraic fiber representation is not independently proved"))?;
        if actual != f.map {
            return Err(refused("Algebraic parameter map is false"));
        }
        let Some(t) = &f.map else {
            if !f.roots.is_empty() {
                return Err(refused("Empty fiber has root events"));
            }
            continue;
        };
        let end = Q::from_integer((k as i64 + 1).into());
        let mut total = 0;
        let mut previous: Option<Q> = None;
        for root in &f.roots {
            let [lo, hi] = &root.interval;
            if lo < &Q::zero()
                || hi > &end
                || lo > hi
                || previous
                    .as_ref()
                    .is_some_and(|p| p > lo || (p == lo && at(&f.polynomial, lo).is_zero()))
            {
                return Err(refused(
                    "Algebraic root intervals overlap or leave the projection domain",
                ));
            }
            if lo != hi && (at(&f.polynomial, lo).is_zero() || at(&f.polynomial, hi).is_zero())
                || count(&f.polynomial, lo, hi)? != 1
            {
                return Err(refused("Algebraic interval does not isolate one root"));
            }
            previous = Some(hi.clone());
            total += 1;
            let (kind, event) = classify(&f.polynomial, t, &root.interval, k, self_pair, width)?
                .ok_or_else(|| refused("Algebraic root classification remains unproved"))?;
            if kind != root.kind {
                return Err(refused("Algebraic event classification is false"));
            }
            if kind == "event" || kind == "diagonal" {
                events.push(event);
            }
        }
        let expected = count(&f.polynomial, &Q::zero(), &end)?
            + usize::from(at(&f.polynomial, &Q::zero()).is_zero())
            + usize::from(at(&f.polynomial, &end).is_zero());
        if total != expected {
            return Err(refused(
                "Algebraic root census omits or adds a projection root",
            ));
        }
    }
    if monic(product_all) != r {
        return Err(refused(
            "Algebraic factor census does not cover the eliminant",
        ));
    }
    Ok(events)
}

pub(in super::super) fn stationary(net: &[[Q; 3]], width: &Q) -> Result<Vec<Factor>> {
    let d = super::tangent_numerators(net);
    let polynomials = d.map(|v| {
        power(
            &v.into_iter()
                .map(|q| [q, Q::zero(), Q::zero()])
                .collect::<Vec<_>>(),
            0,
        )
    });
    let g = gcd(&polynomials[0], &polynomials[1])?;
    if g.is_empty() {
        return Err(refused("A constant curve has a nonisolated stationary set"));
    }
    let p = monic(quotient(&g, &gcd(&g, &derivative(&g))?)?);
    if p.len() < 2 {
        return Ok(Vec::new());
    }
    let roots = isolates(&p, Q::zero(), Q::one(), width)?
        .into_iter()
        .map(|interval| Root {
            interval,
            kind: "diagonal".into(),
        })
        .collect();
    Ok(vec![Factor {
        polynomial: p,
        map: Some(vec![Q::zero(), Q::one()]),
        roots,
    }])
}
pub(in super::super) fn verify_stationary(
    net: &[[Q; 3]],
    width: &Q,
    factors: &[Factor],
) -> Result<Vec<Event>> {
    let expected = stationary(net, width)?;
    if factors.len() != expected.len() {
        return Err(refused("Stationary factor census is incomplete"));
    }
    let mut events = Vec::new();
    for (f, e) in factors.iter().zip(&expected) {
        if f.polynomial != e.polynomial || f.map != e.map {
            return Err(refused("Stationary polynomial identity is false"));
        }
        let mut previous = None;
        for r in &f.roots {
            let [a, b] = &r.interval;
            if a < &Q::zero()
                || b > &Q::one()
                || a > b
                || b - a > *width
                || previous
                    .as_ref()
                    .is_some_and(|p: &Q| p > a || (p == a && at(&f.polynomial, a).is_zero()))
                || r.kind != "diagonal"
                || count(&f.polynomial, a, b)? != 1
                || (a != b && (at(&f.polynomial, a).is_zero() || at(&f.polynomial, b).is_zero()))
            {
                return Err(refused("Stationary root isolation claim is false"));
            }
            previous = Some(b.clone());
            events.push(Event {
                first: r.interval.clone(),
                second: r.interval.clone(),
                singular: true,
            });
        }
        if f.roots.len()
            != count(&f.polynomial, &Q::zero(), &Q::one())?
                + usize::from(at(&f.polynomial, &Q::zero()).is_zero())
                + usize::from(at(&f.polynomial, &Q::one()).is_zero())
        {
            return Err(refused("Stationary root census omits or adds an event"));
        }
    }
    Ok(events)
}

// Exact translated/overlap admissions reuse the current complete arbitrary-system engine.
#[cfg(test)]
pub(in super::super) fn produce_translated(
    a: &[[Q; 3]], b: &[[Q; 3]], offsets: &[[Q; 2]; 2],
    self_pair: bool, width: &Q,
) -> Result<(u8, Vec<Factor>)> {
    if self_pair && (a != b || offsets[0] != offsets[1]) {
        return Err(refused("Translated self-pair diagonal has distinct source expressions"));
    }
    let a = shore_events::translated_net(a, &offsets[0])?;
    let b = shore_events::translated_net(b, &offsets[1])?;
    let equations = intersection_polynomials(&a, &b, self_pair)?;
    produce_equations(&equations, self_pair, width)
}
pub(super) fn produce_equations(equations:&[Bi;2],self_pair:bool,width:&Q)->Result<(u8,Vec<Factor>)>{produce_system(equations,self_pair,width)}
pub(in super::super) fn coordinate_boundary(
    a: &[[Q; 3]],
    b: &[[Q; 3]],
    axis: usize,
    value: &Q,
    width: &Q,
) -> Result<(u8, Vec<Factor>)> {
    if axis > 1 || a.is_empty() || b.is_empty() || width <= &Q::zero() {
        return Err(refused("Invalid overlap boundary constraint"));
    }
    let scalar = |net: &[[Q; 3]]| {
        sub(
            &power(net, axis),
            &power(net, 2).iter().map(|q| q * value).collect::<Vec<_>>(),
        )
    };
    let first = scalar(a);
    let second = scalar(b);
    if first.is_empty() || second.is_empty() {
        return Err(refused(
            "Overlap boundary has a nonisolated parameter fiber",
        ));
    }
    let equations = [
        vec![first],
        second.into_iter().map(|q| trim(vec![q])).collect(),
    ];
    produce_equations(&equations, false, width)
}
#[cfg(test)]
pub(in super::super) fn verify_translated(
    a: &[[Q; 3]], b: &[[Q; 3]], offsets: &[[Q; 2]; 2],
    self_pair: bool, width: &Q, k: u8, factors: &[Factor],
) -> Result<Vec<Event>> {
    if self_pair && (a != b || offsets[0] != offsets[1]) {
        return Err(refused("Translated self-pair verifier has distinct source expressions"));
    }
    let a = shore_events::translated_net(a, &offsets[0])?;
    let b = shore_events::translated_net(b, &offsets[1])?;
    let equations = intersection_polynomials(&a, &b, self_pair)?;
    verify_equations(&equations, self_pair, width, k, factors)
}
#[cfg(test)]
fn verify_equations(equations:&[Bi;2],self_pair:bool,width:&Q,k:u8,factors:&[Factor])->Result<Vec<Event>>{verify_system(equations,self_pair,width,k,factors)}
#[cfg(test)]
mod overlap_boundary_tests {
    use super::*;
    #[test]
    fn independent_square_and_cube_parameters_share_complete_joint_extension() {
        let q = Q::new(1.into(), 2.into());
        // s^2=q and t^3=q. Neither parameter is a rational polynomial
        // of the other; a separating joint primitive is required.
        let equations = [
            vec![vec![-q.clone(), Q::zero(), Q::one()]],
            vec![vec![-q], Vec::new(), Vec::new(), vec![Q::one()]],
        ];
        let width = Q::new(1.into(), 1024.into());
        let first = vec![
            [Q::zero(), Q::zero(), Q::one()],
            [Q::zero(), Q::zero(), Q::one()],
            [Q::one(), Q::zero(), Q::one()],
        ];
        let second = vec![
            [Q::zero(), Q::zero(), Q::one()],
            [Q::zero(), Q::zero(), Q::one()],
            [Q::zero(), Q::zero(), Q::one()],
            [Q::one(), Q::zero(), Q::one()],
        ];
        let (k, factors) =
            coordinate_boundary(&first, &second, 0, &Q::new(1.into(), 2.into()), &width).unwrap();
        assert!(coordinate_boundary(&first, &second, 1, &Q::zero(), &width).is_err());
        assert!(coordinate_boundary(&first, &second, 2, &Q::zero(), &width).is_err());
        assert_ne!(k, 0);
        let events = verify_equations(&equations, false, &width, k, &factors).unwrap();
        assert_eq!(events.len(), 1);
        assert!(events[0].first[0] > Q::new(7.into(), 10.into()));
        assert!(events[0].second[0] > Q::new(79.into(), 100.into()));
        let mut tampered = factors.clone();
        let mapped = tampered.iter_mut().find(|f| f.map.is_some()).unwrap();
        mapped.map = Some(vec![Q::zero()]);
        assert!(verify_equations(&equations, false, &width, k, &tampered).is_err());
        let mut omitted = factors.clone();
        omitted.pop();
        assert!(verify_equations(&equations, false, &width, k, &omitted).is_err());
    }
}

#[cfg(test)]
mod translated_tests {
    use super::*;
    #[test]
    fn translated_pair_keeps_both_original_clocks_and_rejects_stale_proof() {
        let q = |n: i64| Q::from_integer(n.into());
        let a = [[q(0),q(0),q(1)],[q(1),q(0),q(1)]];
        let b = [[q(0),q(-1),q(1)],[q(0),q(1),q(1)]];
        let third = Q::new(1.into(),3.into());
        let offsets = [[q(0),q(0)],[third.clone(),q(0)]];
        let width = Q::new(1.into(),4096.into());
        let (k,f) = produce_translated(&a,&b,&offsets,false,&width).unwrap();
        let events = verify_translated(&a,&b,&offsets,false,&width,k,&f).unwrap();
        assert_eq!(events.len(),1);
        assert!(events[0].first[0] <= third && events[0].first[1] >= third);
        let half = Q::new(1.into(),2.into());
        assert!(events[0].second[0] <= half && events[0].second[1] >= half);
        assert!(verify_translated(&a,&b,&offsets,false,&width,k,&[]).is_err());
        assert!(verify_translated(&a,&b,&[[q(0),q(0)],[q(2),q(0)]],false,&width,k,&f).is_err());
        assert!(produce_translated(&a,&a,&offsets,true,&width).is_err());
        assert!(verify_translated(&a,&a,&offsets,true,&width,k,&f).is_err());
    }
}

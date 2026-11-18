//! Exact rational (not necessarily projective) clocks on curved graph seams.
//! A candidate is accepted only after a complete homogeneous composition identity
//! and an embedded triangular parameter extension. No image-only substitution.
use super::*;
type Clock = Vec<[Q; 2]>;
fn choose(n: usize, k: usize) -> Q {
    (0..k).fold(Q::one(), |x, i| {
        x * integer((n - i) as i64) / integer((i + 1) as i64)
    })
}
fn compose(a: &[sb::H], p: &Clock, work: &mut Work) -> Result<Vec<sb::H>> {
    let n = a.len() - 1;
    let d = p.len() - 1;
    if n * d > 64 {
        return Err(fail("nonlinear seam identity degree budget exceeded"));
    }
    let num = p.iter().map(|p| p[0].clone()).collect::<Vec<_>>();
    let rest = p.iter().map(|p| &p[1] - &p[0]).collect::<Vec<_>>();
    let powers = |v: &[Q], work: &mut Work| -> Result<Vec<Vec<Q>>> {
        let mut out = vec![vec![Q::one()]];
        for i in 0..n {
            work.charge(out[i].len() * v.len())?;
            out.push(sb::multiply(&out[i], v));
        }
        Ok(out)
    };
    let (u, v) = (powers(&num, work)?, powers(&rest, work)?);
    let mut h = vec![std::array::from_fn(|_| Q::zero()); n * d + 1];
    for i in 0..=n {
        work.charge(u[i].len() * v[n - i].len())?;
        let b = sb::multiply(&u[i], &v[n - i]);
        let c = choose(n, i);
        for (j, b) in b.iter().enumerate() {
            for k in 0..4 {
                h[j][k] += &a[i][k] * b * &c;
            }
        }
    }
    if h.iter().any(|p| {
        p[3] <= Q::zero()
            || p.iter()
                .any(|q| q.numer().bits() > 8192 || q.denom().bits() > 8192)
    }) {
        return Err(fail("nonlinear seam identity weight/bit budget exceeded"));
    }
    Ok(h)
}
fn graph_proposal(a: &[sb::H], b: &[sb::H], work: &mut Work) -> Result<Clock> {
    let (p, q) = (sb::point(&a[0]), sb::point(a.last().unwrap()));
    let mut axes = (0..3)
        .map(|k| std::array::from_fn(|j| if j == k { Q::one() } else { Q::zero() }))
        .collect::<Vec<P>>();
    // Recover rotated polynomial graph coordinates by the nullspace of the
    // exact residual A(t) - [A(0)+(A(1)-A(0))*t].
    let n = a.len() - 1;
    let mut residual = Vec::<P>::new();
    for i in 0..=n + 1 {
        let t = Q::new(i.into(), (n + 1).into());
        let r = std::array::from_fn(|k| {
            (if i <= n {
                (Q::one() - &t) * (&a[i][k] - &a[i][3] * &p[k])
            } else {
                Q::zero()
            }) + (if i > 0 {
                &t * (&a[i - 1][k] - &a[i - 1][3] * &q[k])
            } else {
                Q::zero()
            })
        });
        if !zero(&r) {
            residual.push(r);
        }
    }
    if residual.is_empty() {
        axes.push(sub(&q, &p));
    } else {
        for e in &axes.clone() {
            axes.push(cross(&residual[0], e));
        }
        for r in residual.iter().skip(1) {
            axes.push(cross(&residual[0], r));
        }
    }
    for axis in axes {
        work.charge(1)?;
        let (x0, x1) = (dot(&p, &axis), dot(&q, &axis));
        if x0 == x1 {
            continue;
        }
        let derivative = integer(n as i64)
            * (dot(
                &std::array::from_fn(|k| &a[1][k] * &a[0][3] - &a[0][k] * &a[1][3]),
                &axis,
            ))
            / (&a[0][3] * &a[0][3]);
        let ratio = derivative / (&x1 - &x0);
        if ratio <= Q::zero() {
            continue;
        }
        let projected = a
            .iter()
            .map(|p| {
                [
                    dot(&std::array::from_fn(|k| p[k].clone()), &axis),
                    Q::zero(),
                    Q::zero(),
                    p[3].clone(),
                ]
            })
            .collect::<Vec<_>>();
        let line = vec![
            [x0.clone(), Q::zero(), Q::zero(), Q::one()],
            [&x1 * &ratio, Q::zero(), Q::zero(), ratio.clone()],
        ];
        if !sb::same(&projected, &line) {
            continue;
        }
        let sign = if x1 > x0 { Q::one() } else { -Q::one() };
        let clock = b
            .iter()
            .map(|p| {
                let x = dot(&std::array::from_fn(|k| p[k].clone()), &axis);
                let num = x.clone() - &x0 * &p[3];
                [&sign * &num, &sign * (num + &ratio * (&x1 * &p[3] - x))]
            })
            .collect::<Clock>();
        if clock
            .iter()
            .any(|p| p[1] <= Q::zero() || p[0] < Q::zero() || p[0] > p[1])
        {
            continue;
        }
        if clock[0][0] != Q::zero() || clock.last().unwrap()[0] != clock.last().unwrap()[1] {
            continue;
        }
        let curve = clock
            .iter()
            .map(|p| [p[0].clone(), Q::zero(), Q::zero(), p[1].clone()])
            .collect::<Vec<_>>();
        if !monotone(
            &[curve.clone(), curve],
            &[Q::zero(), Q::zero(), Q::one()],
            work,
        )? {
            continue;
        }
        if sb::same(&compose(a, &clock, work)?, b) {
            return Ok(clock);
        }
    }
    Err(fail(
        "no proved rational graph transport for this curved seam",
    ))
}
// Search homogeneous rational inverse observables through degree four.
// The nullspace only proposes a clock; full spatial composition verifies it.
fn observables(a: &[sb::H], degree: usize, work: &mut Work) -> Result<Vec<Vec<Q>>> {
    let c = (0..4)
        .map(|k| a.iter().map(|p| p[k].clone()).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    fn monomials(
        c: &[Vec<Q>],
        left: usize,
        start: usize,
        p: Vec<Q>,
        out: &mut Vec<Vec<Q>>,
        work: &mut Work,
    ) -> Result<()> {
        if left == 0 {
            out.push(p);
            return Ok(());
        }
        for i in start..4 {
            work.charge(p.len() * c[i].len())?;
            let next = sb::multiply(&p, &c[i]);
            if next
                .iter()
                .any(|q| q.numer().bits() > 8192 || q.denom().bits() > 8192)
            {
                return Err(fail("seam observable bit budget exceeded"));
            }
            monomials(c, left - 1, i, next, out, work)?;
        }
        Ok(())
    }
    let mut out = Vec::new();
    monomials(&c, degree, 0, vec![Q::one()], &mut out, work)?;
    Ok(out)
}

fn inverse_candidates(a: &[Vec<Q>], work: &mut Work) -> Result<Vec<Vec<Q>>> {
    let (n, count) = (a[0].len() - 1, a.len());
    let mut rows = Vec::new();
    for i in 0..=n + 1 {
        let t = Q::new(i.into(), (n + 1).into());
        rows.push(
            (0..2 * count)
                .map(|k| {
                    if k < count {
                        (if i <= n {
                            (Q::one() - &t) * &a[k][i]
                        } else {
                            Q::zero()
                        }) + if i > 0 { &t * &a[k][i - 1] } else { Q::zero() }
                    } else if i > 0 {
                        -&t * &a[k - count][i - 1]
                    } else {
                        Q::zero()
                    }
                })
                .collect::<Vec<_>>(),
        );
    }
    super::linear::kernel(rows, 2 * count, work)
}

fn reduce_clock(mut p: Clock) -> Clock {
    while p.len() > 2 {
        let n = p.len() - 1;
        let mut q = vec![p[0].clone()];
        for i in 1..n {
            q.push(std::array::from_fn(|k| {
                (integer(n as i64) * &p[i][k] - integer(i as i64) * &q[i - 1][k])
                    / integer((n - i) as i64)
            }));
        }
        if q.last() != p.last() {
            break;
        }
        p = q;
    }
    p
}
fn inverse_proposal(a: &[sb::H], b: &[sb::H], work: &mut Work) -> Result<Clock> {
    for degree in 1..=4 {
        let ac = observables(a, degree, work)?;
        let bc = observables(b, degree, work)?;
        let count = ac.len();
        for inverse in inverse_candidates(&ac, work)? {
            work.charge(count * (ac[0].len() + 2 * bc[0].len()))?;
            let eval = |c: &[Vec<Q>], offset: usize| -> Vec<Q> {
                (0..c[0].len())
                    .map(|i| (0..count).map(|k| &inverse[offset + k] * &c[k][i]).sum())
                    .collect()
            };
            // A nullspace vector with a vanishing denominator is not an inverse.
            let Ok(source_clock) = super::nonlinear_cancel::clock(
                eval(&ac, 0)
                    .into_iter()
                    .zip(eval(&ac, count))
                    .map(|(n, d)| [n, d])
                    .collect(),
                work,
            ) else {
                continue;
            };
            let den = source_clock
                .iter()
                .map(|p| p[1].clone())
                .collect::<Vec<_>>();
            let sign = if den.iter().all(|q| *q > Q::zero()) {
                Q::one()
            } else if den.iter().all(|q| *q < Q::zero()) {
                -Q::one()
            } else {
                continue;
            };
            let num = eval(&bc, 0);
            let den = eval(&bc, count);
            let Ok(clock) = super::nonlinear_cancel::clock(
                num.iter()
                    .zip(den)
                    .map(|(n, d)| [&sign * n, &sign * d])
                    .collect(),
                work,
            ) else {
                continue;
            };
            let clock = reduce_clock(clock);
            if clock
                .iter()
                .any(|p| p[1] <= Q::zero() || p[0] < Q::zero() || p[0] > p[1])
                || clock[0][0] != Q::zero()
                || clock.last().unwrap()[0] != clock.last().unwrap()[1]
            {
                continue;
            }
            let c = clock
                .iter()
                .map(|p| [p[0].clone(), Q::zero(), Q::zero(), p[1].clone()])
                .collect::<Vec<_>>();
            if monotone(&[c.clone(), c], &[Q::zero(), Q::zero(), Q::one()], work)?
                && compose(a, &clock, work).is_ok_and(|h| sb::same(&h, b))
            {
                return Ok(clock);
            }
        }
    }
    Err(fail(
        "no proved degree-one-through-four rational inverse on this spatial seam",
    ))
}
fn proposal(a: &[sb::H], b: &[sb::H], work: &mut Work) -> Result<Clock> {
    graph_proposal(a, b, work).or_else(|_| inverse_proposal(a, b, work))
}
// A common cover avoids explicitly representing an algebraic inverse. For
// example A(t)=(t²,t⁴), B(s)=(s³,s⁶) agree at t=r³, s=r².
// The complete homogeneous identity is checked; powers alone prove nothing.
fn common_cover(a: &[sb::H], b: &[sb::H], work: &mut Work) -> Result<(Clock, Clock)> {
    let power = |degree: usize| -> Clock {
        (0..=degree)
            .map(|i| [if i == degree { Q::one() } else { Q::zero() }, Q::one()])
            .collect()
    };
    // Vanishing order at the common endpoint is invariant under equality.
    // It selects admissible powers before expensive composition proposals.
    let order = |h: &[sb::H]| -> usize {
        let p = sb::point(&h[0]);
        let mut residual = h
            .iter()
            .map(|q| std::array::from_fn::<_, 3, _>(|k| &q[k] - &q[3] * &p[k]))
            .collect::<Vec<_>>();
        for degree in 0..h.len() {
            if residual[0].iter().any(|q| !q.is_zero()) {
                return degree;
            }
            residual = residual
                .windows(2)
                .map(|v| std::array::from_fn(|k| &v[1][k] - &v[0][k]))
                .collect();
        }
        0
    };
    let (oa, ob) = (order(a), order(b));
    if oa == 0 || ob == 0 {
        return Err(fail("constant common-cover seam"));
    }
    for total in 3usize..=16 {
        for p in 1..=8 {
            let Some(q) = total.checked_sub(p) else {
                continue;
            };
            if q == 0
                || p * oa != q * ob
                || q > 8
                || p == q
                || (a.len() - 1) * p > 64
                || (b.len() - 1) * q > 64
            {
                continue;
            }
            let (ca, cb) = (power(p), power(q));
            if sb::same(&compose(a, &ca, work)?, &compose(b, &cb, work)?) {
                return Ok((ca, cb));
            }
        }
    }
    Err(fail("no proved bounded common algebraic seam cover"))
}
fn orient_clock(mut clock: Clock, reversed: bool) -> Clock {
    if reversed {
        clock.reverse();
        for p in &mut clock {
            p[0] = &p[1] - &p[0];
        }
    }
    clock
}
pub(super) fn synchronize(patches: &mut [Patch], work: &mut Work) -> Result<()> {
    if patches.len() > 128 {
        return Err(fail("nonlinear seam chart budget exceeded"));
    }
    let mut edges = BTreeMap::<(u32, u32), Vec<(usize, usize, bool, Vec<sb::H>)>>::new();
    for (i, p) in patches.iter().enumerate() {
        for (k, mut h) in bernstein::edges(&p.h).into_iter().enumerate() {
            let (a, b) = (p.ids[k], p.ids[(k + 1) % 3]);
            let reversed = a > b;
            if reversed {
                h.reverse();
            }
            edges
                .entry((a.min(b), a.max(b)))
                .or_default()
                .push((i, k, reversed, h));
        }
    }
    let mut clocks = vec![BTreeMap::<usize, Clock>::new(); patches.len()];
    let mut stationary = vec![false; patches.len()];
    for e in edges.values() {
        if e.len() != 2 {
            return Err(fail("nonlinear seam incidence is not two"));
        }
        let (a, k, rev, ha) = &e[0];
        let (b, l, other, hb) = &e[1];
        if sb::same(ha, hb) || segment_seam(ha, hb, work)? {
            continue;
        }
        let (patch, edge, reversed, mut clock) = if let Ok(p) = proposal(ha, hb, work) {
            (*a, *k, *rev, p)
        } else if let Ok(p) = proposal(hb, ha, work) {
            (*b, *l, *other, p)
        } else {
            let (ca, cb) = common_cover(ha, hb, work)?;
            stationary[*a] = true;
            stationary[*b] = true;
            clocks[*a].insert(*k, orient_clock(ca, *rev));
            clocks[*b].insert(*l, orient_clock(cb, *other));
            continue;
        };
        if reversed {
            clock.reverse();
            for p in &mut clock {
                p[0] = &p[1] - &p[0];
            }
        }
        clocks[patch].insert(edge, clock);
    }
    let uv = [[0, 0], [1, 0], [0, 1]];
    for (i, p) in patches.iter_mut().enumerate() {
        if clocks[i].is_empty() {
            continue;
        }
        let mut curves = Vec::new();
        for k in 0..3 {
            let clock = clocks[i]
                .get(&k)
                .cloned()
                .unwrap_or_else(|| vec![[Q::zero(), Q::one()], [Q::one(), Q::one()]]);
            curves.push(
                clock
                    .iter()
                    .map(|q| {
                        [
                            integer(uv[k][0]) * &q[1]
                                + integer(uv[(k + 1) % 3][0] - uv[k][0]) * &q[0],
                            integer(uv[k][1]) * &q[1]
                                + integer(uv[(k + 1) % 3][1] - uv[k][1]) * &q[0],
                            Q::zero(),
                            q[1].clone(),
                        ]
                    })
                    .collect(),
            );
        }
        let map = trim::disk_uv_parameter(curves, work, stationary[i])?;
        let h = bernstein::compose_triangle(&p.h, &map, work)?;
        if bernstein::corners(&h) != bernstein::corners(&p.h) {
            return Err(fail("nonlinear chart transport changed its vertices"));
        }
        p.h = h;
        p.derivatives = bernstein::derivatives(&p.h, work)?;
        p.motion = bernstein::homotopy(&p.h, work)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn algebraic_seams_have_an_exact_common_cover() {
        let curve = |d, exponent| {
            (0..=d)
                .map(|i| {
                    let term = |k| {
                        if i >= k {
                            choose(i, k) / choose(d, k)
                        } else {
                            Q::zero()
                        }
                    };
                    [term(exponent), term(2 * exponent), Q::zero(), Q::one()]
                })
                .collect::<Vec<_>>()
        };
        let (a, b) = (curve(4, 2), curve(6, 3));
        let (ca, cb) = common_cover(&a, &b, &mut Work::default()).unwrap();
        assert_eq!((ca.len(), cb.len()), (4, 3));
        assert!(sb::same(
            &compose(&a, &ca, &mut Work::default()).unwrap(),
            &compose(&b, &cb, &mut Work::default()).unwrap()
        ));
        // The algebraic candidate must also extend over the actual UV disk.
        for clock in [&ca, &cb] {
            let edge = clock
                .iter()
                .map(|q| [q[0].clone(), Q::zero(), Q::zero(), q[1].clone()])
                .collect();
            let line = |a: [i64; 2], b: [i64; 2]| {
                vec![
                    [integer(a[0]), integer(a[1]), Q::zero(), Q::one()],
                    [integer(b[0]), integer(b[1]), Q::zero(), Q::one()],
                ]
            };
            trim::disk_uv_parameter(
                vec![edge, line([1, 0], [0, 1]), line([0, 1], [0, 0])],
                &mut Work::default(),
                true,
            )
            .unwrap();
        }
        let mut bad = b;
        bad[2][2] += Q::one();
        assert!(common_cover(&a, &bad, &mut Work::default()).is_err());
    }
    #[test]
    fn spatial_rational_carrier_recovers_a_homogeneous_inverse() {
        // A(t)=(t,t²,t³)/(1+2t³). The denominator has no rational
        // linear factor; t=X/(W-2Z) is a homogeneous rational inverse.
        let a = vec![
            [integer(0), integer(0), integer(0), integer(1)],
            [
                Q::new(1.into(), 3.into()),
                integer(0),
                integer(0),
                integer(1),
            ],
            [
                Q::new(2.into(), 3.into()),
                Q::new(1.into(), 3.into()),
                integer(0),
                integer(1),
            ],
            [integer(1), integer(1), integer(1), integer(3)],
        ];
        let clock = vec![
            [integer(0), integer(1)],
            [Q::new(1.into(), 4.into()), integer(1)],
            [integer(1), integer(1)],
        ];
        let b = compose(&a, &clock, &mut Work::default()).unwrap();
        assert!(graph_proposal(&a, &b, &mut Work::default()).is_err());
        let p = proposal(&a, &b, &mut Work::default()).unwrap();
        assert!(sb::same(
            &compose(&a, &p, &mut Work::default()).unwrap(),
            &b
        ));
        let mut bad = b;
        bad[2][2] += Q::new(1.into(), 128.into());
        assert!(proposal(&a, &bad, &mut Work::default()).is_err());
    }
    #[test]
    fn spatial_seam_with_quadratic_inverse_is_exact() {
        // A(t)=(t²,t³,t+t⁵); t=Z-XY requires a quadratic observable.
        let a = (0..=5)
            .map(|i| {
                let power = |k| {
                    if i >= k {
                        choose(i, k) / choose(5, k)
                    } else {
                        Q::zero()
                    }
                };
                [power(2), power(3), power(1) + power(5), Q::one()]
            })
            .collect::<Vec<_>>();
        let clock = vec![
            [integer(0), integer(1)],
            [Q::new(1.into(), 4.into()), integer(1)],
            [integer(1), integer(1)],
        ];
        let b = compose(&a, &clock, &mut Work::default()).unwrap();
        assert!(graph_proposal(&a, &b, &mut Work::default()).is_err());
        let recovered = proposal(&a, &b, &mut Work::default()).unwrap();
        assert_eq!(recovered.len(), 3);
        assert!(sb::same(
            &compose(&a, &recovered, &mut Work::default()).unwrap(),
            &b
        ));
        let mut bad = b;
        bad[4][0] += Q::new(1.into(), 256.into());
        assert!(proposal(&a, &bad, &mut Work::default()).is_err());
    }
    #[test]
    fn spatial_seam_inverse_can_require_cubic_observables() {
        // A=(t²,t³,t+t⁸), inverse t=Z-X Y², with a constant denominator.
        let a = (0..=8)
            .map(|i| {
                let power = |k| {
                    if i >= k {
                        choose(i, k) / choose(8, k)
                    } else {
                        Q::zero()
                    }
                };
                [power(2), power(3), power(1) + power(8), Q::one()]
            })
            .collect::<Vec<_>>();
        let clock = vec![
            [integer(0), integer(1)],
            [Q::new(1.into(), 4.into()), integer(1)],
            [integer(1), integer(1)],
        ];
        let b = compose(&a, &clock, &mut Work::default()).unwrap();
        let p = inverse_proposal(&a, &b, &mut Work::default()).unwrap();
        assert_eq!(p.len(), 3);
        assert!(sb::same(
            &compose(&a, &p, &mut Work::default()).unwrap(),
            &b
        ));
    }
    #[test]
    fn spatial_seam_inverse_can_require_quartic_observables() {
        // A=(t²,t³,t+t¹¹), t=Z-X Y³ is a quartic spatial observable.
        let a = (0..=11)
            .map(|i| {
                let power = |k| {
                    if i >= k {
                        choose(i, k) / choose(11, k)
                    } else {
                        Q::zero()
                    }
                };
                [power(2), power(3), power(1) + power(11), Q::one()]
            })
            .collect::<Vec<_>>();
        let clock = vec![
            [integer(0), integer(1)],
            [Q::new(1.into(), 4.into()), integer(1)],
            [integer(1), integer(1)],
        ];
        let b = compose(&a, &clock, &mut Work::default()).unwrap();
        let p = inverse_proposal(&a, &b, &mut Work::default()).unwrap();
        assert!(sb::same(
            &compose(&a, &p, &mut Work::default()).unwrap(),
            &b
        ));
    }
    #[test]
    fn collapsed_carrier_cannot_forge_an_inverse() {
        let a = vec![[integer(0), integer(0), integer(0), integer(1)]; 3];
        assert!(inverse_proposal(&a, &a, &mut Work::default()).is_err());
    }
    #[test]
    fn curved_seams_allow_nonprojective_rational_clocks_and_bind_all_coordinates() {
        let a = vec![
            [Q::zero(), Q::zero(), Q::zero(), Q::one()],
            [
                Q::new(1.into(), 2.into()),
                Q::zero(),
                Q::new(1.into(), 4.into()),
                Q::one(),
            ],
            [Q::one(), Q::one(), Q::one(), Q::one()],
        ];
        let clock = vec![
            [Q::zero(), Q::one()],
            [Q::new(1.into(), 4.into()), Q::one()],
            [Q::one(), Q::one()],
        ];
        let b = compose(&a, &clock, &mut Work::default()).unwrap();
        let recovered = proposal(&a, &b, &mut Work::default()).unwrap();
        assert!(sb::same(
            &compose(&a, &recovered, &mut Work::default()).unwrap(),
            &b
        ));
        let mut bad = b;
        bad[2][2] += Q::new(1.into(), 64.into());
        assert!(proposal(&a, &bad, &mut Work::default()).is_err());
    }
}

#[cfg(test)]
mod complex_tests {
    use super::*;
    #[test]
    fn nonlinear_curved_transport_extends_inside_a_closed_curved_complex() {
        closed_complex(false);
    }
    #[test]
    fn spatial_projective_carrier_transport_extends_over_a_closed_complex() {
        closed_complex(true);
    }
    fn closed_complex(projective: bool) {
        let c = [[1, 1, 1], [-1, -1, 1], [-1, 1, -1], [1, -1, -1]].map(|p| p.map(integer));
        let mut positions = c.clone();
        for p in &mut positions {
            p[2] += Q::new(1.into(), 64.into());
        }
        let mut patches = Vec::new();
        for ids in [[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]] {
            let a = ids.map(|i| c[i as usize].clone());
            let linear = (0..3)
                .map(|k| {
                    vec![
                        vec![a[0][k].clone(), a[2][k].clone()],
                        vec![a[1][k].clone()],
                    ]
                })
                .collect::<Vec<_>>();
            let one = vec![vec![Q::one(), Q::one()], vec![Q::one()]];
            let mut h = (0..=2)
                .map(|i| vec![std::array::from_fn(|_| Q::zero()); 3 - i])
                .collect::<Net>();
            let mut coords = linear
                .iter()
                .map(|p| bernstein::product(p, &one, &mut Work::default()).unwrap())
                .collect::<Vec<_>>();
            let yy = bernstein::product(&linear[1], &linear[1], &mut Work::default()).unwrap();
            for i in 0..=2 {
                for j in 0..=2 - i {
                    coords[2][i][j] += &yy[i][j] / integer(64);
                    for k in 0..4 {
                        h[i][j][k] = if k == 3 {
                            Q::one()
                        } else {
                            coords[k][i][j].clone()
                        };
                    }
                }
            }
            patches.push(Patch {
                ids,
                h: h.clone(),
                derivatives: bernstein::derivatives(&h, &mut Work::default()).unwrap(),
                motion: bernstein::homotopy(&h, &mut Work::default()).unwrap(),
            });
        }
        if projective {
            for p in &mut positions {
                let w = Q::one() + &p[2] / integer(64);
                for q in p {
                    *q /= &w;
                }
            }
            for p in &mut patches {
                for h in p.h.iter_mut().flatten() {
                    let z = &h[2] / integer(64);
                    h[3] += z;
                }
                p.derivatives = bernstein::derivatives(&p.h, &mut Work::default()).unwrap();
                p.motion = bernstein::homotopy(&p.h, &mut Work::default()).unwrap();
            }
        }
        let uv = trim::disk_uv(
            vec![
                vec![
                    [integer(0), integer(0), integer(0), integer(1)],
                    [integer(1), integer(0), integer(0), integer(1)],
                ],
                vec![
                    [integer(1), integer(0), integer(0), integer(1)],
                    [
                        Q::new(3.into(), 4.into()),
                        Q::new(1.into(), 4.into()),
                        Q::zero(),
                        Q::one(),
                    ],
                    [integer(0), integer(1), integer(0), integer(1)],
                ],
                vec![
                    [integer(0), integer(1), integer(0), integer(1)],
                    [integer(0), integer(0), integer(0), integer(1)],
                ],
            ],
            &mut Work::default(),
        )
        .unwrap();
        let h = bernstein::compose_triangle(&patches[0].h, &uv, &mut Work::default()).unwrap();
        patches[0] = Patch {
            ids: patches[0].ids,
            h: h.clone(),
            derivatives: bernstein::derivatives(&h, &mut Work::default()).unwrap(),
            motion: bernstein::homotopy(&h, &mut Work::default()).unwrap(),
        };
        assert!(transport::synchronize(&mut patches.clone(), &mut Work::default()).is_err());
        synchronize(&mut patches, &mut Work::default()).unwrap();
        let mut edges = BTreeMap::<(u32, u32), Vec<Vec<sb::H>>>::new();
        for p in &patches {
            for (k, mut h) in bernstein::edges(&p.h).into_iter().enumerate() {
                let (a, b) = (p.ids[k], p.ids[(k + 1) % 3]);
                if a > b {
                    h.reverse();
                }
                edges.entry((a.min(b), a.max(b))).or_default().push(h);
            }
        }
        for e in edges.values() {
            assert!(
                sb::same(&e[0], &e[1]) || segment_seam(&e[0], &e[1], &mut Work::default()).unwrap()
            );
        }
        for v in 0..4 {
            let incident = patches
                .iter()
                .enumerate()
                .filter_map(|(i, p)| p.ids.contains(&v).then_some(i))
                .collect::<Vec<_>>();
            star(v, &incident, &patches, &positions, &mut Work::default()).unwrap();
        }
    }
}

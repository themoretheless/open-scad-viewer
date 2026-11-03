//! Relative material transition on a periodic UV quotient. A reference material
//! value must come from a separately admitted body. This owner proves crossings,
//! not body validity, Jordan embedding or an absolute inside on a torus.
use super::*;
use num_traits::{Signed, ToPrimitive, Zero};
#[derive(Clone, Copy, Debug)]
pub struct OriginalSsPathLimits {
    pub max_segments: usize,
    pub max_periodic_copies: usize,
    pub geometry: OriginalSsCurveLimits,
}
impl Default for OriginalSsPathLimits {
    fn default() -> Self {
        Self {
            max_segments: 4096,
            max_periodic_copies: 16384,
            geometry: Default::default(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct QualifiedOriginalSsPathParity {
    wire: OriginalSsRootWire,
    support: usize,
    from: [f64; 2],
    to: [f64; 2],
    odd: bool,
    segments: usize,
    copies: usize,
    limits: OriginalSsPathLimits,
}
impl QualifiedOriginalSsPathParity {
    pub fn odd_crossings(&self) -> bool {
        self.odd
    }
    pub fn segments(&self) -> usize {
        self.segments
    }
    pub fn periodic_copies(&self) -> usize {
        self.copies
    }
    pub fn reverify(&self) -> Result<()> {
        self.wire.reverify()?;
        let f = qualify_original_ss_path_parity(
            &self.wire,
            self.support,
            self.from,
            self.to,
            self.limits,
        )?;
        check(
            self.odd == f.odd && self.segments == f.segments && self.copies == f.copies,
            "Original quotient path crossing proof changed",
        )
    }
}
fn q(x: f64) -> Q {
    Q::from_float(x).unwrap()
}
fn floor(x: &Q) -> num_bigint::BigInt {
    let d = x.numer() / x.denom();
    if x.is_negative() && x.numer() % x.denom() != num_bigint::BigInt::zero() {
        d - 1
    } else {
        d
    }
}
fn periods(s: &Surface) -> [Option<Q>; 2] {
    [
        if s.periodic_u {
            Some(q(s.knots_u[s.control_points.len()]) - q(s.knots_u[s.degree_u]))
        } else {
            None
        },
        if s.periodic_v {
            Some(q(s.knots_v[s.control_points[0].len()]) - q(s.knots_v[s.degree_v]))
        } else {
            None
        },
    ]
}
fn offset(
    b: &OriginalSsCurveBounds,
    support: usize,
    phase: [i32; 4],
    periods: &[Option<Q>; 2],
) -> [[Q; 2]; 2] {
    std::array::from_fn(|k| {
        let shift = periods[k]
            .as_ref()
            .map(|p| p * Q::from_integer(phase[2 * support + k].into()))
            .unwrap_or_else(Q::zero);
        [
            q(b.uv[2 * support + k][0]) + &shift,
            q(b.uv[2 * support + k][1]) + shift,
        ]
    })
}
fn midpoint(b: &[[Q; 2]; 2]) -> [Q; 2] {
    std::array::from_fn(|k| (&b[k][0] + &b[k][1]) / q(2.))
}
fn orient(a: &[Q; 2], b: &[Q; 2], p: &[Q; 2]) -> Q {
    (&b[0] - &a[0]) * (&p[1] - &a[1]) - (&b[1] - &a[1]) * (&p[0] - &a[0])
}
fn crossing(a: &[Q; 2], b: &[Q; 2], p: &[Q; 2]) -> i32 {
    let sign = orient(a, b, p);
    if a[1] <= p[1] && p[1] < b[1] && sign > Q::zero() {
        1
    } else if b[1] <= p[1] && p[1] < a[1] && sign < Q::zero() {
        -1
    } else {
        0
    }
}
fn transform(p: &[Q; 2], from: &[Q; 2], v: &[Q; 2]) -> [Q; 2] {
    [
        (&p[0] - &from[0]) * &v[0] + (&p[1] - &from[1]) * &v[1],
        &v[0] * (&p[1] - &from[1]) - &v[1] * (&p[0] - &from[0]),
    ]
}
fn outward_clock(x: &Q, lower: bool) -> Result<f64> {
    let f = x
        .to_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| crate::input("Quotient root clock overflow"))?;
    Ok(if lower && q(f) > *x {
        f.next_down()
    } else if !lower && q(f) < *x {
        f.next_up()
    } else {
        f
    })
}
/// A finite lifted path can cross a noncontractible boundary. Counting all
/// relevant exact deck translations handles periodic shores without forcing
/// each closed physical contour to have zero winding in one UV rectangle.
pub fn qualify_original_ss_path_parity(
    wire: &OriginalSsRootWire,
    support: usize,
    from: [f64; 2],
    to: [f64; 2],
    limits: OriginalSsPathLimits,
) -> Result<QualifiedOriginalSsPathParity> {
    check(
        support < 2
            && from.iter().chain(&to).all(|v| v.is_finite())
            && from != to
            && (1..=16384).contains(&limits.max_segments)
            && (1..=65536).contains(&limits.max_periodic_copies),
        "Invalid quotient path work or endpoints",
    )?;
    let source = wire.edges()[0].curve().source();
    for e in wire.edges() {
        let s = e.curve().source();
        check(
            source.first() == s.first()
                && source.second() == s.second()
                && !matches!(
                    e.start().kind(),
                    OriginalSsPortKind::CollapsedPole | OriginalSsPortKind::OriginalSourceNode
                )
                && !matches!(
                    e.end().kind(),
                    OriginalSsPortKind::CollapsedPole | OriginalSsPortKind::OriginalSourceNode
                ),
            "Quotient path needs one original source pair; collapsed or nonlinear node joins require their own UV bridges",
        )?;
    }
    let s = if support == 0 {
        source.first()
    } else {
        source.second()
    };
    let period = periods(s);
    let from = from.map(q);
    let to = to.map(q);
    let path_box: [[Q; 2]; 2] = std::array::from_fn(|k| {
        [
            from[k].clone().min(to[k].clone()),
            from[k].clone().max(to[k].clone()),
        ]
    });
    let v = std::array::from_fn(|k| &to[k] - &from[k]);
    let end = [&v[0] * &v[0] + &v[1] * &v[1], Q::zero()];
    let origin = [Q::zero(), Q::zero()];
    let mut remaining = limits.geometry;
    let mut cache = Vec::new();
    let mut segments = 0;
    let mut copies = 0;
    let mut n = 1usize;
    loop {
        let count = n
            .checked_mul(wire.edges().len())
            .ok_or_else(|| crate::resource("Quotient subdivision overflow"))?;
        check(
            count <= limits.max_segments.saturating_sub(segments),
            "Quotient path subdivision exhausted",
        )?;
        segments += count;
        let mut separated = true;
        let mut winding = 0i32;
        for edge in wire.edges() {
            let start = super::winding::endpoint(edge.start().point(), &mut cache, &mut remaining)?;
            let finish = super::winding::endpoint(edge.end().point(), &mut cache, &mut remaining)?;
            let a = edge.start().parameter_bounds();
            let b = edge.end().parameter_bounds();
            let (lo, hi) = if edge.forward() {
                (&a[1], &b[0])
            } else {
                (&b[1], &a[0])
            };
            let mut clocks = vec![None];
            for i in 1..n {
                let x = lo + (hi - lo) * Q::from_integer(i.into()) / Q::from_integer(n.into());
                let f = x
                    .to_f64()
                    .filter(|v| v.is_finite())
                    .ok_or_else(|| crate::input("Quotient subdivision clock overflow"))?;
                check(
                    lo < &q(f) && &q(f) < hi,
                    "Quotient interior clock is unrepresentable",
                )?;
                clocks.push(Some(f));
            }
            clocks.push(None);
            if !edge.forward() {
                clocks.reverse();
            }
            let mut points = vec![offset(&start, support, edge.start().phase(), &period)];
            for clock in clocks.iter().take(n).skip(1) {
                let b = edge.curve().enclose([clock.unwrap(); 2], remaining)?;
                super::winding::charge(&mut remaining, &b)?;
                points.push(offset(&b, support, [0; 4], &period));
            }
            points.push(offset(&finish, support, edge.end().phase(), &period));
            let outer = edge.parameter_bounds();
            let natural = edge.curve().parameter_domain();
            let lower = outward_clock(&outer[0], true)?.max(natural[0]);
            let upper = outward_clock(&outer[1], false)?.min(natural[1]);
            for i in 0..n {
                let c0 = clocks[i].unwrap_or(if edge.forward() { lower } else { upper });
                let c1 = clocks[i + 1].unwrap_or(if edge.forward() { upper } else { lower });
                let arc = edge.curve().enclose([c0.min(c1), c0.max(c1)], remaining)?;
                super::winding::charge(&mut remaining, &arc)?;
                let arc = offset(&arc, support, [0; 4], &period);
                let bounds: [[Q; 2]; 2] = std::array::from_fn(|k| {
                    [
                        arc[k][0]
                            .clone()
                            .min(points[i][k][0].clone())
                            .min(points[i + 1][k][0].clone()),
                        arc[k][1]
                            .clone()
                            .max(points[i][k][1].clone())
                            .max(points[i + 1][k][1].clone()),
                    ]
                });
                let mut ranges = [[0i32; 2]; 2];
                let mut absent = false;
                for k in 0..2 {
                    if let Some(p) = &period[k] {
                        let low = (&path_box[k][0] - &bounds[k][1]) / p;
                        let high = (&path_box[k][1] - &bounds[k][0]) / p;
                        let ceil = -floor(&(-low));
                        let floor = floor(&high);
                        let lo = ceil
                            .to_i32()
                            .ok_or_else(|| crate::resource("Quotient deck count overflow"))?;
                        let hi = floor
                            .to_i32()
                            .ok_or_else(|| crate::resource("Quotient deck count overflow"))?;
                        check(
                            lo.unsigned_abs() <= 65536 && hi.unsigned_abs() <= 65536,
                            "Quotient deck index allowance exhausted",
                        )?;
                        ranges[k] = [lo, hi];
                        if lo > hi {
                            absent = true;
                        }
                    } else if path_box[k][1] < bounds[k][0] || bounds[k][1] < path_box[k][0] {
                        absent = true;
                    }
                }
                if absent {
                    continue;
                }
                for u in ranges[0][0]..=ranges[0][1] {
                    for w in ranges[1][0]..=ranges[1][1] {
                        check(
                            copies < limits.max_periodic_copies,
                            "Quotient path deck work exhausted",
                        )?;
                        copies += 1;
                        let deck = [u, w];
                        let shift: [Q; 2] = std::array::from_fn(|k| {
                            period[k]
                                .as_ref()
                                .map(|p| p * Q::from_integer(deck[k].into()))
                                .unwrap_or_else(Q::zero)
                        });
                        let box2: [[Q; 2]; 2] = std::array::from_fn(|k| {
                            [&bounds[k][0] + &shift[k], &bounds[k][1] + &shift[k]]
                        });
                        if [&from, &to]
                            .iter()
                            .any(|p| !(0..2).any(|k| p[k] < box2[k][0] || box2[k][1] < p[k]))
                        {
                            separated = false;
                            break;
                        }
                        let a = midpoint(&points[i]);
                        let b = midpoint(&points[i + 1]);
                        let a = std::array::from_fn(|k| &a[k] + &shift[k]);
                        let b = std::array::from_fn(|k| &b[k] + &shift[k]);
                        let a = transform(&a, &from, &v);
                        let b = transform(&b, &from, &v);
                        winding += crossing(&a, &b, &origin) - crossing(&a, &b, &end);
                    }
                    if !separated {
                        break;
                    }
                }
                if !separated {
                    break;
                }
            }
            if !separated {
                break;
            }
        }
        if separated {
            return Ok(QualifiedOriginalSsPathParity {
                wire: wire.clone(),
                support,
                from: from.map(|q| q.to_f64().unwrap()),
                to: to.map(|q| q.to_f64().unwrap()),
                odd: winding % 2 != 0,
                segments,
                copies,
                limits,
            });
        }
        n = n
            .checked_mul(2)
            .ok_or_else(|| crate::resource("Quotient path subdivision overflow"))?;
    }
}

//! Exact preprocessing of authored binary64 spline coefficients. In particular,
//! subtract a narrow chart origin BEFORE conversion back to binary64 intervals.
//! This module issues no geometric certificate by itself.
use crate::{Result, check, curve::Curve, distance_bounds::Interval as I};
#[path = "exact_trace/restriction.rs"]
mod restriction;
use num_rational::BigRational as Q;
use num_traits::{One, ToPrimitive, Zero};
pub(super) use restriction::Restriction;

pub(super) struct Work {
    pub used: usize,
    pub limit: usize,
}
impl Work {
    pub(super) fn operations(&mut self, n: usize) -> Result<()> {
        if n > self.limit.saturating_sub(self.used) {
            self.used = self.limit;
            return Err(crate::numeric_err(
                "Exact trace coefficient budget exhausted",
            ));
        }
        self.used += n;
        Ok(())
    }
    fn charge(&mut self, q: &Q) -> Result<()> {
        self.operations(1)?;
        check(
            q.numer().bits() <= 8192 && q.denom().bits() <= 8192,
            "Exact trace coefficient bit budget exhausted",
        )
    }
}
fn authored(x: f64) -> Result<Q> {
    Q::from_float(x).ok_or_else(|| crate::numeric_err("Non-finite exact trace input"))
}
/// Conversion is verified against the rational itself, rather than assuming
/// that a library's nearest-float conversion has a particular rounding mode.
fn enclosure(q: &Q) -> Result<I> {
    let x = q
        .to_f64()
        .filter(|x| x.is_finite())
        .ok_or_else(|| crate::numeric_err("Exact trace outside binary64 range"))?;
    if authored(x)? == *q {
        return Ok(I::point(x));
    }
    let mut lo = x;
    let mut hi = x;
    for _ in 0..4 {
        if authored(lo)? > *q {
            lo = lo.next_down();
        }
        if authored(hi)? < *q {
            hi = hi.next_up();
        }
        if lo.is_finite() && hi.is_finite() && authored(lo)? <= *q && authored(hi)? >= *q {
            return I::new(lo, hi);
        }
    }
    Err(crate::numeric_err(
        "Could not enclose exact trace coefficient",
    ))
}
type Grid = Vec<Vec<[Q; 4]>>;
struct Jet {
    degrees: [usize; 2],
    knots: [Vec<f64>; 2],
    poles: Grid,
}
// Exact separable tensor contraction. In particular, identity/triangular
// extraction matrices must not perform thousands of zero BigRational products.
// Reassociation changes neither rational coefficients nor final enclosures.
fn extract_tensor_net(
    jet: &Jet,
    span: [usize; 2],
    u: &[Vec<Q>],
    v: &[Vec<Q>],
    w: &mut Work,
) -> Result<Net> {
    let offset = [span[0] - jet.degrees[0], span[1] - jet.degrees[1]];
    let mut intermediate: Grid = vec![vec![std::array::from_fn(|_| Q::zero()); v.len()]; u.len()];
    for i in 0..u.len() {
        for t in 0..v.len() {
            for r in 0..u.len() {
                if u[i][r].is_zero() {
                    continue;
                }
                for k in 0..4 {
                    let pole = &jet.poles[offset[0] + r][offset[1] + t][k];
                    if pole.is_zero() {
                        continue;
                    }
                    intermediate[i][t][k] += &u[i][r] * pole;
                    w.charge(&intermediate[i][t][k])?;
                }
            }
        }
    }
    let mut net = vec![vec![[I::point(0.); 4]; v.len()]; u.len()];
    for i in 0..u.len() {
        for j in 0..v.len() {
            for k in 0..4 {
                let mut q = Q::zero();
                for t in 0..v.len() {
                    if v[j][t].is_zero() || intermediate[i][t][k].is_zero() {
                        continue;
                    }
                    q += &v[j][t] * &intermediate[i][t][k];
                    w.charge(&q)?;
                }
                net[i][j][k] = enclosure(&q)?;
            }
        }
    }
    Ok(net)
}
/// All homogeneous derivatives, projected onto the exact integer orthogonal
/// chart (x-y, x+y-2z, x+y+z). Projection precedes differencing.
type Net = Vec<Vec<[I; 4]>>;
pub(super) struct Support {
    jets: Vec<Jet>,
    degrees: [usize; 2],
    constant_jets: Vec<[Option<I>; 4]>,
    nets: std::cell::RefCell<std::collections::BTreeMap<(usize, usize, usize), Net>>,
}
/// Position numerators are absent only when original constant homogeneous
/// weights make them unnecessary for graph jets. Rational pcurves still
/// retain their common homogeneous denominator; variable surface weights
/// require all original position numerators.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct GraphTrace {
    pub(super) position: Option<[Vec<I>; 3]>,
    pub(super) weight: Vec<I>,
    pub(super) derivatives: [Vec<Vec<I>>; 5],
}
impl GraphTrace {
    pub(super) fn restrict(&self, map: &Restriction, w: &mut Work) -> Result<Self> {
        let position = match &self.position {
            Some(p) => Some(
                p.iter()
                    .map(|p| map.apply(p, w))
                    .collect::<Result<Vec<_>>>()?
                    .try_into()
                    .ok()
                    .unwrap(),
            ),
            None => None,
        };
        let weight = map.apply(&self.weight, w)?;
        let derivatives = self
            .derivatives
            .iter()
            .map(|j| {
                j.iter()
                    .map(|p| map.apply(p, w))
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?
            .try_into()
            .ok()
            .unwrap();
        Ok(Self {
            position,
            weight,
            derivatives,
        })
    }
    pub(super) fn polynomials(&self) -> impl Iterator<Item = &Vec<I>> {
        self.position
            .iter()
            .flat_map(|p| p.iter())
            .chain(std::iter::once(&self.weight))
            .chain(self.derivatives.iter().flatten())
    }
}
const ORDERS: [[usize; 2]; 6] = [[0, 0], [1, 0], [0, 1], [2, 0], [1, 1], [0, 2]];
impl Support {
    pub(super) fn hulls(&self) -> Result<Vec<[I; 4]>> {
        self.jets
            .iter()
            .map(|jet| {
                let mut out = [I::point(0.); 4];
                for k in 0..4 {
                    let mut lo = f64::INFINITY;
                    let mut hi = f64::NEG_INFINITY;
                    for row in &jet.poles {
                        for cp in row {
                            let i = enclosure(&cp[k])?;
                            lo = lo.min(i.lo);
                            hi = hi.max(i.hi);
                        }
                    }
                    out[k] = I::new(lo, hi)?;
                }
                Ok(out)
            })
            .collect()
    }
    pub(super) fn new(s: &crate::surface::Surface, w: &mut Work) -> Result<Self> {
        Self::new_in_frame(s, w, false)
    }
    /// Both choices are exact integer projections of original authored poles.
    /// Cartesian is an additional proof chart, never a rotated floating proxy.
    pub(super) fn new_in_frame(
        s: &crate::surface::Surface,
        w: &mut Work,
        cartesian: bool,
    ) -> Result<Self> {
        s.validate()?;
        check(
            !s.periodic_u && !s.periodic_v && s.degree_u <= 12 && s.degree_v <= 12,
            "Exact graph support degree or periodicity unsupported",
        )?;
        check(
            s.control_points
                .len()
                .saturating_mul(s.control_points[0].len())
                <= 4096,
            "Exact graph support pole budget exhausted",
        )?;
        let mut poles = Vec::new();
        for (i, row) in s.control_points.iter().enumerate() {
            let mut output = Vec::new();
            for (j, p) in row.iter().enumerate() {
                let weight = authored(s.weights[i][j])?;
                let x = authored(p[0])?;
                let y = authored(p[1])?;
                let z = authored(p[2])?;
                let cp = if cartesian {
                    [&x * &weight, &y * &weight, &z * &weight, weight]
                } else {
                    [
                        (&x - &y) * &weight,
                        (&x + &y - Q::from_integer(2.into()) * &z) * &weight,
                        (x + y + z) * &weight,
                        weight,
                    ]
                };
                for q in &cp {
                    w.charge(q)?;
                }
                output.push(cp);
            }
            poles.push(output);
        }
        let degrees = [s.degree_u, s.degree_v];
        let mut jets = Vec::new();
        for orders in ORDERS {
            let mut jet = Jet {
                degrees,
                knots: [s.knots_u.clone(), s.knots_v.clone()],
                poles: poles.clone(),
            };
            for axis in 0..2 {
                for _ in 0..orders[axis] {
                    jet = differentiate(jet, axis, w)?;
                }
            }
            jets.push(jet);
        }
        let mut constant_jets = Vec::new();
        for jet in &jets {
            w.operations(4 * jet.poles.len() * jet.poles[0].len())?;
            let mut constants = [None; 4];
            for k in 0..4 {
                let value = &jet.poles[0][0][k];
                if jet.poles.iter().flatten().all(|p| &p[k] == value) {
                    constants[k] = Some(enclosure(value)?);
                }
            }
            constant_jets.push(constants);
        }
        Ok(Self {
            jets,
            degrees,
            constant_jets,
            nets: std::cell::RefCell::new(Default::default()),
        })
    }
    /// Each derivative uses the same homogeneous pcurve denominator power.
    /// Thus rational source jet quotient formulas can cancel it algebraically.
    pub(super) fn compose(
        &self,
        pc: &Curve,
        range: [f64; 2],
        spans: [usize; 2],
        w: &mut Work,
    ) -> Result<Vec<Vec<Vec<I>>>> {
        self.compose_impl(pc, range, spans, w, false, false)
    }
    pub(super) fn compose_graph(
        &self,
        pc: &Curve,
        range: [f64; 2],
        spans: [usize; 2],
        w: &mut Work,
    ) -> Result<GraphTrace> {
        self.compose_graph_oriented(pc, range, spans, w, false)
    }
    pub(super) fn compose_graph_oriented(
        &self,
        pc: &Curve,
        range: [f64; 2],
        spans: [usize; 2],
        w: &mut Work,
        reversed: bool,
    ) -> Result<GraphTrace> {
        w.operations(pc.weights.len() + 6)?;
        let constant = self.constant_jets[0][3].filter(|v| v.lo == v.hi && v.lo > 0.);
        let polynomial_pc = pc.weights.iter().all(|&x| x == 1.);
        let omit = constant.is_some()
            && self.constant_jets[1..]
                .iter()
                .all(|jet| jet[3] == Some(I::point(0.)));
        let mut jets = self.compose_impl(pc, range, spans, w, omit, reversed)?;
        if omit {
            check(
                jets.iter().all(|j| j[3].iter().all(|&v| v == I::point(0.))),
                "Original constant-weight graph derivative invariant failed",
            )?;
            let scale = constant.unwrap();
            let weight = if polynomial_pc {
                vec![scale]
            } else {
                // Original pcurve weights are independent of UV chart origin.
                // Wp^(degree_u+degree_v) is the common factor of ALL composed
                // homogeneous jets, including this omitted position weight.
                let h = pcurve_oriented(pc, range, [[0., 1.]; 2], w, reversed)?;
                let mut power = vec![I::point(1.)];
                for _ in 0..self.degrees.iter().sum::<usize>() {
                    w.operations(12 * power.len() * h[2].len())?;
                    power = super::mul(&power, &h[2])?;
                }
                if scale == I::point(1.) {
                    power
                } else {
                    w.operations(power.len())?;
                    power
                        .into_iter()
                        .map(|v| v.mul(scale))
                        .collect::<Result<Vec<_>>>()?
                }
            };
            Ok(GraphTrace {
                position: None,
                weight,
                derivatives: jets.try_into().ok().unwrap(),
            })
        } else {
            let mut position = jets.remove(0);
            let weight = position.pop().unwrap();
            Ok(GraphTrace {
                position: Some(position.try_into().ok().unwrap()),
                weight,
                derivatives: jets.try_into().ok().unwrap(),
            })
        }
    }
    fn compose_impl(
        &self,
        pc: &Curve,
        range: [f64; 2],
        spans: [usize; 2],
        w: &mut Work,
        omit_position: bool,
        reversed: bool,
    ) -> Result<Vec<Vec<Vec<I>>>> {
        let root = &self.jets[0];
        let domains =
            std::array::from_fn(|a| [root.knots[a][spans[a]], root.knots[a][spans[a] + 1]]);
        let h = pcurve_oriented(pc, range, domains, w, reversed)?;
        w.operations(pc.weights.len())?;
        let polynomial_pc = pc.weights.iter().all(|&weight| weight == 1.);
        let mut weight_powers = vec![vec![I::point(1.)]];
        for n in 1..=4 {
            if polynomial_pc {
                weight_powers.push(vec![I::point(1.)]);
            } else {
                w.operations(12 * weight_powers[n - 1].len() * h[2].len())?;
                weight_powers.push(super::mul(&weight_powers[n - 1], &h[2])?);
            }
        }
        let mut basis = TraceBasis::new(&h, w)?;
        let mut output = Vec::new();
        for (index, jet) in self.jets.iter().enumerate() {
            if omit_position && index == 0 {
                continue;
            }
            if self.constant_jets[index]
                .iter()
                .all(|v| *v == Some(I::point(0.)))
            {
                // An original exact zero remains zero for every rational
                // pcurve denominator; do not expand it into uncertain noise.
                w.operations(4)?;
                output.push(vec![vec![I::point(0.)]; 4]);
                continue;
            }
            if polynomial_pc && self.constant_jets[index].iter().all(Option::is_some) {
                // Exact constants of the ORIGINAL homogeneous derivative net.
                // Unit pcurve weights make the common denominator exactly one.
                w.operations(4)?;
                output.push(
                    self.constant_jets[index]
                        .iter()
                        .map(|v| vec![v.unwrap()])
                        .collect(),
                );
                continue;
            }
            let [a, b] = ORDERS[index];
            let zero = a > self.degrees[0] || b > self.degrees[1];
            if zero {
                output.push(vec![
                    vec![
                        I::point(0.);
                        pc.degree * (self.degrees[0] + self.degrees[1]) + 1
                    ];
                    4
                ]);
                continue;
            }
            let span = [spans[0] - a, spans[1] - b];
            let key = (index, span[0], span[1]);
            let cached = self.nets.borrow().get(&key).cloned();
            let net = if let Some(net) = cached {
                net
            } else {
                let u = extraction(&jet.knots[0], jet.degrees[0], span[0], w)?;
                let v = extraction(&jet.knots[1], jet.degrees[1], span[1], w)?;
                let net = extract_tensor_net(jet, span, &u, &v, w)?;
                // The cache belongs to this immutable authored support, never to
                // a receipt or a rounded construction proxy. Eviction drops only
                // an optimization; the next visit recomputes original coefficients.
                let mut cache = self.nets.borrow_mut();
                if cache.len() >= 256 {
                    cache.clear();
                }
                cache.insert(key, net.clone());
                net
            };
            // A literal unit-weight boundary pcurve restricts the original
            // tensor jet to one boundary row/column exactly. No epsilon test
            // and no replacement of the authored coordinate is permitted.
            let (net, degrees) =
                restrict_polynomial_boundaries(net, jet.degrees, &h, polynomial_pc, w)?;
            let active = std::array::from_fn(|k| {
                self.constant_jets[index][k] != Some(I::point(0.))
                    && (!polynomial_pc || self.constant_jets[index][k].is_none())
            });
            let composed = basis.compose(&net, degrees, active, w)?;
            let mut composed = if polynomial_pc {
                composed
            } else {
                composed
                    .iter()
                    .map(|poly| {
                        w.operations(12 * poly.len() * weight_powers[a + b].len())?;
                        super::mul(poly, &weight_powers[a + b])
                    })
                    .collect::<Result<Vec<_>>>()?
            };
            for k in 0..4 {
                if let Some(value) = self.constant_jets[index][k] {
                    if polynomial_pc || value == I::point(0.) {
                        composed[k] = vec![value];
                    }
                }
            }
            output.push(composed);
        }
        Ok(output)
    }
}
/// Per-composition cache: every entry belongs to this exact authored pcurve
/// interval. No basis or work receipt can leak across cells or source curves.
struct TraceBasis<'a> {
    h: &'a [Vec<I>],
    complement: [Vec<I>; 2],
    up: [Vec<Vec<I>>; 2],
    down: [Vec<Vec<I>>; 2],
    rows: std::collections::BTreeMap<(usize, usize), Vec<Vec<I>>>,
}
impl<'a> TraceBasis<'a> {
    fn new(h: &'a [Vec<I>], w: &mut Work) -> Result<Self> {
        let mut complement: [Vec<I>; 2] = std::array::from_fn(|_| Vec::new());
        for axis in 0..2 {
            w.operations(h[axis].len())?;
            complement[axis] = h[2]
                .iter()
                .zip(&h[axis])
                .map(|(&weight, &value)| weight.sub(value))
                .collect::<Result<Vec<_>>>()?;
        }
        Ok(Self {
            h,
            complement,
            up: std::array::from_fn(|_| vec![vec![I::point(1.)]]),
            down: std::array::from_fn(|_| vec![vec![I::point(1.)]]),
            rows: Default::default(),
        })
    }
    fn row(&mut self, axis: usize, degree: usize, w: &mut Work) -> Result<Vec<Vec<I>>> {
        if !self.rows.contains_key(&(axis, degree)) {
            while self.up[axis].len() <= degree {
                let previous = self.up[axis].last().unwrap();
                w.operations(12 * previous.len() * self.h[axis].len())?;
                let next = super::mul(previous, &self.h[axis])?;
                self.up[axis].push(next);
            }
            while self.down[axis].len() <= degree {
                let previous = self.down[axis].last().unwrap();
                w.operations(12 * previous.len() * self.complement[axis].len())?;
                let next = super::mul(previous, &self.complement[axis])?;
                self.down[axis].push(next);
            }
            let mut row = Vec::new();
            for i in 0..=degree {
                let (up, down) = (&self.up[axis][i], &self.down[axis][degree - i]);
                w.operations(12 * up.len() * down.len())?;
                let product = super::mul(up, down)?;
                w.operations(product.len())?;
                let factor = I::point(super::choose(degree, i));
                row.push(
                    product
                        .into_iter()
                        .map(|x| x.mul(factor))
                        .collect::<Result<Vec<_>>>()?,
                );
            }
            self.rows.insert((axis, degree), row);
        }
        let row = &self.rows[&(axis, degree)];
        w.operations(row.iter().map(Vec::len).sum())?;
        Ok(row.clone())
    }
    fn compose(
        &mut self,
        net: &Net,
        degrees: [usize; 2],
        active: [bool; 4],
        w: &mut Work,
    ) -> Result<Vec<Vec<I>>> {
        let [du, dv] = degrees;
        let u = self.row(0, du, w)?;
        let v = self.row(1, dv, w)?;
        let degree = (self.h[0].len() - 1) * (du + dv);
        let mut output = vec![vec![I::point(0.); degree + 1]; 4];
        let coordinates = active.iter().filter(|&&a| a).count();
        for i in 0..=du {
            for j in 0..=dv {
                w.operations(12 * u[i].len() * v[j].len())?;
                let coefficient = super::mul(&u[i], &v[j])?;
                w.operations(2 * coordinates * coefficient.len())?;
                for k in 0..4 {
                    if active[k] {
                        for (out, &value) in output[k].iter_mut().zip(&coefficient) {
                            *out = out.add(value.mul(net[i][j][k])?)?;
                        }
                    }
                }
            }
        }
        Ok(output)
    }
}

/// Exact boundary restriction retains the same homogeneous jets only for a
/// unit pcurve denominator. For rational pcurves the missing denominator power
/// must not be discarded, even if the geometric coordinate is constant.
fn restrict_polynomial_boundaries(
    mut net: Net,
    mut degrees: [usize; 2],
    h: &[Vec<I>],
    polynomial: bool,
    w: &mut Work,
) -> Result<(Net, [usize; 2])> {
    if !polynomial {
        return Ok((net, degrees));
    }
    w.operations(2 * h.iter().map(Vec::len).sum::<usize>())?;
    if !h[2].iter().all(|&v| v == I::point(1.)) {
        return Ok((net, degrees));
    }
    for axis in 0..2 {
        let boundary = if h[axis].iter().all(|&v| v == I::point(0.)) {
            Some(0)
        } else if h[axis].iter().all(|&v| v == I::point(1.)) {
            Some(degrees[axis])
        } else {
            None
        };
        if let Some(index) = boundary {
            w.operations(4 * net.len() * net[0].len())?;
            net = if axis == 0 {
                vec![net.swap_remove(index)]
            } else {
                net.into_iter()
                    .map(|mut row| vec![row.swap_remove(index)])
                    .collect()
            };
            degrees[axis] = 0;
        }
    }
    Ok((net, degrees))
}
fn differentiate(jet: Jet, axis: usize, w: &mut Work) -> Result<Jet> {
    let p = jet.degrees[axis];
    if p == 0 {
        let mut jet = jet;
        for row in &mut jet.poles {
            for cp in row {
                *cp = std::array::from_fn(|_| Q::zero());
            }
        }
        return Ok(jet);
    }
    let n = jet.poles.len() - usize::from(axis == 0);
    let m = jet.poles[0].len() - usize::from(axis == 1);
    let mut poles = vec![vec![std::array::from_fn(|_| Q::zero()); m]; n];
    for i in 0..n {
        for j in 0..m {
            let index = if axis == 0 { i } else { j };
            let den =
                authored(jet.knots[axis][index + p + 1])? - authored(jet.knots[axis][index + 1])?;
            // Disconnected supports are rejected by the caller's continuity gate.
            check(!den.is_zero(), "Disconnected exact derivative support")?;
            for k in 0..4 {
                poles[i][j][k] = (&jet.poles[i + usize::from(axis == 0)]
                    [j + usize::from(axis == 1)][k]
                    - &jet.poles[i][j][k])
                    * Q::from_integer(p.into())
                    / &den;
                w.charge(&poles[i][j][k])?;
            }
        }
    }
    let mut degrees = jet.degrees;
    degrees[axis] -= 1;
    let mut knots = jet.knots;
    knots[axis] = knots[axis][1..knots[axis].len() - 1].to_vec();
    Ok(Jet {
        degrees,
        knots,
        poles,
    })
}
use spline_extraction::{mix,blossom};

/// Restrict an original clamped pcurve traversal to one original knot span,
/// then express it in the surface knot cell. Spline extraction and traversal
/// conversion use exact rational arithmetic, not rounded knot insertion.
/// A range crossing knots refuses; callers must cover all original spans.
pub(super) fn pcurve(
    c: &Curve,
    range: [f64; 2],
    cell: [[f64; 2]; 2],
    w: &mut Work,
) -> Result<Vec<Vec<I>>> {
    pcurve_oriented(c, range, cell, w, false)
}
pub(super) fn pcurve_oriented(
    c: &Curve,
    range: [f64; 2],
    cell: [[f64; 2]; 2],
    w: &mut Work,
    reversed: bool,
) -> Result<Vec<Vec<I>>> {
    c.validate()?;
    let p = c.degree;
    let n = c.control_points.len();
    check(
        n <= 4096
            && p <= 16
            && !c.periodic
            && c.control_points[0].len() == 2
            && c.knots[..=p].iter().all(|x| *x == c.knots[p])
            && c.knots[n..].iter().all(|x| *x == c.knots[n]),
        "Exact trace requires a bounded clamped nonperiodic pcurve",
    )?;
    check(
        range[0] >= 0. && range[1] <= 1. && range[0] < range[1],
        "Invalid exact trace traversal range",
    )?;
    let mut ends = [authored(range[0])?, authored(range[1])?];
    if reversed {
        for end in &mut ends {
            *end = Q::one() - &*end;
            w.charge(end)?;
        }
    }
    let span = if n == p + 1 {
        p
    } else {
        w.operations(n)?;
        let mut index = p + 1;
        while index < n {
            let mut end = index + 1;
            while end < n && c.knots[end] == c.knots[index] {
                end += 1;
            }
            check(
                end - index <= p,
                "Exact pcurve trace cannot skip an internal discontinuity",
            )?;
            index = end;
        }
        let origin = authored(c.knots[p])?;
        let width = authored(c.knots[n])? - &origin;
        let t = ends.clone().map(|t| &origin + &width * t);
        let span = (p..n)
            .find(|&i| {
                c.knots[i] < c.knots[i + 1]
                    && authored(c.knots[i]).is_ok_and(|a| a <= *t.iter().min().unwrap())
                    && authored(c.knots[i + 1]).is_ok_and(|b| *t.iter().max().unwrap() <= b)
            })
            .ok_or_else(|| crate::numeric_err("Exact pcurve range crosses original knots"))?;
        let a = authored(c.knots[span])?;
        let width = authored(c.knots[span + 1])? - &a;
        ends = t.map(|t| (t - &a) / &width);
        span
    };
    let mut h: Vec<Vec<Q>> = vec![Vec::new(); 3];
    for i in span - p..=span {
        let cp = &c.control_points[i];
        let weight = authored(c.weights[i])?;
        for k in 0..2 {
            h[k].push(authored(cp[k])? * &weight);
        }
        h[2].push(weight);
    }
    if n != p + 1 {
        let matrix = extraction(&c.knots, p, span, w)?;
        let mut extracted = vec![Vec::new(); 3];
        for row in matrix {
            for axis in 0..3 {
                let mut value = Q::zero();
                for i in 0..=p {
                    if row[i].is_zero() {
                        continue;
                    }
                    let term = &row[i] * &h[axis][i];
                    w.charge(&term)?;
                    value += term;
                    w.charge(&value)?;
                }
                extracted[axis].push(value);
            }
        }
        h = extracted;
    }
    let mut restricted = vec![Vec::new(); 3];
    for j in 0..=p {
        let args: Vec<_> = (0..p)
            .map(|i| ends[usize::from(i >= p - j)].clone())
            .collect();
        for k in 0..3 {
            restricted[k].push(blossom(h[k].clone(), &args, w)?);
        }
    }
    for k in 0..2 {
        let a = authored(cell[k][0])?;
        let width = authored(cell[k][1])? - &a;
        check(width > Q::zero(), "Empty exact trace chart cell")?;
        for j in 0..=p {
            restricted[k][j] = (&restricted[k][j] - &a * &restricted[2][j]) / &width;
            w.charge(&restricted[k][j])?;
        }
    }
    restricted
        .iter()
        .map(|row| row.iter().map(enclosure).collect())
        .collect()
}

/// Exact Bezier extraction matrix for one original B-spline span. Rows are
/// output Bezier poles, columns are the p+1 original active coefficients.
#[path = "exact_trace/extraction.rs"]
mod spline_extraction;
pub(super) use spline_extraction::extraction;

#[cfg(test)]
#[path = "exact_trace/tests/mod.rs"]
mod tests;

#[cfg(test)]
#[path = "exact_trace/tests/tensor_contraction.rs"]
mod tensor_contraction_tests;

//! Continuous plane-root coverage of the original positive-weight curve.
use crate::{Result, check, curve::Curve, curve_jets, distance_bounds::Interval, resource};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Classification {
    Excluded,
    UniqueRoot,
    RootAtStart,
    RootAtEnd,
    TangencyAtMidpoint,
    HigherOrderContactAtMidpoint,
    TangencyAtStart,
    TangencyAtEnd,
    HigherOrderContactAtStart,
    HigherOrderContactAtEnd,
    Coincident,
    Unresolved,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    ExactCoincidence,
    ExactEndpoint,
    ExactTangency,
    ExactHigherOrderContact,
    ParameterTolerance,
    RootClassificationNotProven,
    WorkLimit,
    PrecisionLimit,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Proof {
    ResidualBounds,
    CoplanarControls,
    EndpointCoefficients,
    QuadraticSquare,
    BernsteinMidpointPower,
    EndpointWithMonotonicity,
    MonotonicityWithSignChange,
    BernsteinOneVariation,
    BernsteinZeroVariation,
    DirectLinearFormula,
}
#[derive(Clone, Debug)]
pub struct Cell {
    pub domain: [f64; 2],
    pub span: usize,
    pub residual_bounds: [f64; 2],
    pub classification: Classification,
    pub stop_reason: StopReason,
    /// Proved scalar root multiplicity; None without one isolated root.
    pub root_multiplicity: Option<usize>,
    /// Optional direct parameter enclosure independent of the coverage cell.
    pub isolated_root_bounds: Option<[f64; 2]>,
    /// Proof used for a resolved classification. None for Unresolved.
    pub proof: Option<Proof>,
}
impl Cell {
    /// Parameter enclosure of the single proved root. The coverage cell may
    /// be much wider than an exact endpoint root. Coincident and unresolved
    /// cells do not describe one isolated root and therefore return None.
    pub fn root_parameter_bounds(&self) -> Option<[f64; 2]> {
        match self.classification {
            Classification::UniqueRoot => Some(self.isolated_root_bounds.unwrap_or(self.domain)),
            Classification::RootAtStart
            | Classification::TangencyAtStart
            | Classification::HigherOrderContactAtStart => Some([self.domain[0]; 2]),
            Classification::RootAtEnd
            | Classification::TangencyAtEnd
            | Classification::HigherOrderContactAtEnd => Some([self.domain[1]; 2]),
            Classification::TangencyAtMidpoint | Classification::HigherOrderContactAtMidpoint => {
                Some([self.domain[0] * 0.5 + self.domain[1] * 0.5; 2])
            }
            Classification::Excluded | Classification::Coincident | Classification::Unresolved => {
                None
            }
        }
    }
}
#[derive(Clone, Debug)]
pub struct Report {
    /// Closed leaf intervals cover every active span, including excluded cells.
    pub cells: Vec<Cell>,
    /// Counts all computed cells, including replaced parents.
    pub work: usize,
    pub all_roots_isolated: bool,
    pub coverage_resolved: bool,
}
#[derive(Clone, Debug)]
pub struct RootEvent {
    pub parameter_bounds: [f64; 2],
    /// Indices of all contributing cells. Their proofs and one-sided
    /// multiplicities remain separate; a knot need not have one common order.
    pub cells: Vec<usize>,
}
impl Report {
    /// Collect proved roots, merging only identical exact parameter points.
    /// Overlapping nondegenerate intervals are never assumed to be aliases.
    /// Unresolved and coincident cells remain available in the original report.
    /// Domain start/end are not equated, even for periodic source storage.
    pub fn root_events(&self) -> Vec<RootEvent> {
        let mut events: Vec<_> = self
            .cells
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                c.root_parameter_bounds().map(|parameter_bounds| RootEvent {
                    parameter_bounds,
                    cells: vec![i],
                })
            })
            .collect();
        events.sort_by(|a, b| {
            a.parameter_bounds[0]
                .total_cmp(&b.parameter_bounds[0])
                .then(a.parameter_bounds[1].total_cmp(&b.parameter_bounds[1]))
        });
        let mut roots: Vec<RootEvent> = Vec::new();
        for event in events {
            if event.parameter_bounds[0] == event.parameter_bounds[1] {
                if let Some(previous) = roots.last_mut() {
                    if previous.parameter_bounds == event.parameter_bounds {
                        previous.cells.extend(event.cells);
                        continue;
                    }
                }
            }
            roots.push(event);
        }
        roots
    }
}
struct Pending {
    cell: Cell,
    unique: bool,
    root_proof: Option<Proof>,
}
#[derive(Clone, Debug)]
pub struct ClosedReport {
    pub report: Report,
    /// Exact end-of-domain root aliases use the domain-start parameter.
    /// Original parameters, proofs and side orders remain in report.cells.
    pub events: Vec<RootEvent>,
}
/// Inspect an exactly closed clamped or exactly authored periodic curve.
/// This proves C0 closure, not periodic derivatives or manifold embedding.
/// The report is constructed here to bind event aliases to the same source.
pub fn inspect_closed(
    c: &Curve,
    normal: [f64; 3],
    offset: f64,
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<ClosedReport> {
    c.validate()?;
    let [lo, hi] = c.domain();
    if c.periodic {
        let period_controls = c.control_points.len() - c.degree;
        // Validate exact translations of the original binary64 knot values.
        // Curve::validate permits a rounding tolerance, which is insufficient
        // for identifying domain endpoints as the same geometric point.
        check(
            (0..c.knots.len() - period_controls).all(|i| {
                exact_sum_zero(&[
                    (1., c.knots[i + period_controls]),
                    (-1., c.knots[i]),
                    (-1., hi),
                    (1., lo),
                ])
            }),
            "Closed plane events require exact periodic knot translations",
        )?;
        check(
            c.knots.iter().filter(|&&t| t == lo).count() <= c.degree,
            "Closed plane events require a continuous periodic seam",
        )?;
        // Original controls and weights repeat exactly, as checked by validate.
        // Translated bases thus have equal one-sided endpoint limits. Seam
        // multiplicity <= degree makes this a C0 identification.
    } else {
        check(
            c.knots[..=c.degree].iter().all(|&t| t == lo)
                && c.knots[c.knots.len() - c.degree - 1..]
                    .iter()
                    .all(|&t| t == hi),
            "Closed plane events require clamped source endpoints",
        )?;
        check(
            c.control_points.first() == c.control_points.last(),
            "Closed plane events require exactly equal original endpoint controls",
        )?;
    }
    let report = inspect(c, normal, offset, parameter_tolerance, max_cells)?;
    let mut events = report.root_events();
    if let Some(index) = events.iter().position(|e| e.parameter_bounds == [hi; 2]) {
        let mut end = events.remove(index);
        if let Some(start) = events.iter_mut().find(|e| e.parameter_bounds == [lo; 2]) {
            start.cells.extend(end.cells);
        } else {
            end.parameter_bounds = [lo; 2];
            events.insert(0, end);
        }
    }
    Ok(ClosedReport { report, events })
}
// Exact binary64 products accumulated as integer magnitudes. The smallest
// product exponent is -2148; up to eight finite products fit in 68 u64 limbs.
fn exact_sum_zero(terms: &[(f64, f64)]) -> bool {
    crate::exact_products::sum_sign(terms) == std::cmp::Ordering::Equal
}
fn exact_plane_zero(p: &[f64], n: [f64; 3], o: f64) -> bool {
    exact_sum_zero(&[(n[0], p[0]), (n[1], p[1]), (n[2], p[2]), (-o, 1.)])
}
// Four binary64 factors have at most 212 mantissa bits. Their exact sum
// occupies at most 8396 bits, including carry for the sixteen terms below.
fn exact_four_products_zero(terms: &[[f64; 4]]) -> bool {
    let mut positive = [0_u64; 136];
    let mut negative = [0_u64; 136];
    for term in terms {
        let mut product = [0_u64; 4];
        product[0] = 1;
        let mut exponent = 0_i32;
        let mut sign = false;
        for x in term {
            let bits = x.to_bits();
            sign ^= bits >> 63 != 0;
            let e = ((bits >> 52) & 2047) as i32;
            let m = (bits & ((1_u64 << 52) - 1)) | if e == 0 { 0 } else { 1_u64 << 52 };
            exponent += if e == 0 { -1074 } else { e - 1075 };
            let mut carry = 0_u128;
            for limb in &mut product {
                let value = u128::from(*limb) * u128::from(m) + carry;
                *limb = value as u64;
                carry = value >> 64;
            }
            debug_assert_eq!(carry, 0);
        }
        let target = if sign { &mut negative } else { &mut positive };
        let shift = (exponent + 4296) as usize;
        for (index, limb) in product.iter().enumerate() {
            for bit in 0..64 {
                if limb & (1_u64 << bit) == 0 {
                    continue;
                }
                let position = shift + index * 64 + bit;
                let mut index = position / 64;
                let mut carry = 1_u64 << (position % 64);
                loop {
                    let (sum, overflow) = target[index].overflowing_add(carry);
                    target[index] = sum;
                    if !overflow {
                        break;
                    }
                    index += 1;
                    carry = 1;
                }
            }
        }
    }
    positive == negative
}
fn exact_linear_parameter(c: &Curve, span: usize, t: f64, n: [f64; 3], o: f64) -> bool {
    if c.degree != 1 {
        return false;
    }
    let [lo, hi] = [c.knots[span], c.knots[span + 1]];
    let [wa, wb] = [c.weights[span - 1], c.weights[span]];
    let a = &c.control_points[span - 1];
    let b = &c.control_points[span];
    exact_weighted_pair(a, b, wa, wb, lo, hi, t, n, o)
}
fn exact_quadratic_knot(c: &Curve, t: f64, n: [f64; 3], o: f64) -> bool {
    if c.degree != 2 {
        return false;
    }
    let Some(k) = (2..=c.control_points.len()).find(|&k| c.knots[k] == t) else {
        return false;
    };
    if c.knots[k - 1] >= t || c.knots[k + 1] <= t {
        return false;
    }
    // At a simple quadratic knot only these two basis functions survive.
    // Their common denominator U[k+1]-U[k-1] is strictly positive.
    exact_weighted_pair(
        &c.control_points[k - 2],
        &c.control_points[k - 1],
        c.weights[k - 2],
        c.weights[k - 1],
        c.knots[k - 1],
        c.knots[k + 1],
        t,
        n,
        o,
    )
}
#[allow(clippy::too_many_arguments)]
fn exact_weighted_pair(
    a: &[f64],
    b: &[f64],
    wa: f64,
    wb: f64,
    lo: f64,
    hi: f64,
    t: f64,
    n: [f64; 3],
    o: f64,
) -> bool {
    // Exact homogeneous residual: wa(hi-t)(n·a-o)+wb(t-lo)(n·b-o).
    // Expanding products avoids rounded differences and constructed points.
    let mut terms = [[0.; 4]; 16];
    for axis in 0..3 {
        terms[axis * 4] = [wa, hi, n[axis], a[axis]];
        terms[axis * 4 + 1] = [-wa, t, n[axis], a[axis]];
        terms[axis * 4 + 2] = [wb, t, n[axis], b[axis]];
        terms[axis * 4 + 3] = [-wb, lo, n[axis], b[axis]];
    }
    terms[12] = [-wa, hi, o, 1.];
    terms[13] = [wa, t, o, 1.];
    terms[14] = [-wb, t, o, 1.];
    terms[15] = [wb, lo, o, 1.];
    exact_four_products_zero(&terms)
}
fn project(jet: &[Interval], normal: [f64; 3], offset: f64) -> Result<Interval> {
    let mut value = Interval::point(-offset);
    for axis in 0..3 {
        // An exact zero coefficient contributes exactly zero.
        if normal[axis] != 0. {
            value = value.add(jet[axis].mul(Interval::point(normal[axis]))?)?;
        }
    }
    Ok(value)
}
fn binomial(n: usize, k: usize) -> i64 {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    (1..=k).fold(1_i64, |v, j| v * (n - j + 1) as i64 / j as i64)
}
fn exact_midpoint_power(c: &Curve, span: usize, d: [f64; 2], n: [f64; 3], o: f64) -> Option<usize> {
    if c.degree < 2
        || d != [c.knots[span], c.knots[span + 1]]
        || c.knots[span - c.degree + 1] != d[0]
        || c.knots[span + c.degree] != d[1]
    {
        return None;
    }
    let mid = d[0] * 0.5 + d[1] * 0.5;
    if !exact_sum_zero(&[(2., mid), (-1., d[0]), (-1., d[1])]) {
        return None;
    }
    let first = span - c.degree;
    let a = &c.control_points[first];
    if exact_plane_zero(a, n, o) {
        return None;
    }
    let wa = c.weights[first];
    // Degree elevation of A(1-2q)^k to degree p has coefficients
    // A*S[i]/choose(p,i), S[i]=sum_j (-1)^j choose(k,j) choose(p-k,i-j).
    // All integer coefficients for p<=25 are exactly representable in f64.
    (2..=c.degree).rev().find(|&order| {
        (1..=c.degree).all(|i| {
            let coefficient: i64 = (0..=order.min(i))
                .map(|j| {
                    let v = binomial(order, j) * binomial(c.degree - order, i - j);
                    if j % 2 == 0 { v } else { -v }
                })
                .sum();
            let denominator = binomial(c.degree, i) as f64;
            let factor = -(coefficient as f64);
            let other = &c.control_points[first + i];
            let w = c.weights[first + i];
            exact_four_products_zero(&[
                [wa, n[0], a[0], factor],
                [wa, n[1], a[1], factor],
                [wa, n[2], a[2], factor],
                [-wa, o, 1., factor],
                [w, n[0], other[0], denominator],
                [w, n[1], other[1], denominator],
                [w, n[2], other[2], denominator],
                [-w, o, 1., denominator],
            ])
        })
    })
}
fn bernstein_endpoint_order(
    c: &Curve,
    span: usize,
    d: [f64; 2],
    n: [f64; 3],
    o: f64,
) -> Result<Option<(bool, usize)>> {
    if c.degree < 2
        || d != [c.knots[span], c.knots[span + 1]]
        || c.knots[span - c.degree + 1] != d[0]
        || c.knots[span + c.degree] != d[1]
    {
        return Ok(None);
    }
    let controls = &c.control_points[span - c.degree..=span];
    let zero: Vec<_> = controls.iter().map(|p| exact_plane_zero(p, n, o)).collect();
    let prefix = zero.iter().take_while(|&&v| v).count();
    let suffix = zero.iter().rev().take_while(|&&v| v).count();
    let result = if prefix >= 1 && suffix == 0 {
        (true, prefix)
    } else if suffix >= 1 && prefix == 0 {
        (false, suffix)
    } else {
        return Ok(None);
    };
    // Bernstein basis functions are strictly positive inside the span.
    // One nonzero coefficient is sufficient; otherwise prove that all
    // nonzero residuals have the same sign. Positive weights preserve signs.
    if zero.iter().filter(|&&v| !v).count() > 1 {
        let mut sign = None;
        for (p, &is_zero) in controls.iter().zip(&zero) {
            if is_zero {
                continue;
            }
            let point: Vec<_> = p.iter().copied().map(Interval::point).collect();
            let r = project(&point, n, o)?;
            let next = if r.lo > 0. {
                true
            } else if r.hi < 0. {
                false
            } else {
                return Ok(None);
            };
            if sign.is_some_and(|old| old != next) {
                return Ok(None);
            }
            sign = Some(next);
        }
    }
    // The first/last nonzero Bernstein coefficient fixes the exact vanishing
    // order, while the shared sign excludes any other root on the open span.
    Ok(Some(result))
}
fn bernstein_variation_upper(
    c: &Curve,
    span: usize,
    d: [f64; 2],
    n: [f64; 3],
    o: f64,
) -> Result<usize> {
    let net = crate::curve_distance::restricted_controls(c, span, Interval::new(d[0], d[1])?)?;
    let origin: Vec<_> = c.control_points[span - c.degree]
        .iter()
        .copied()
        .map(Interval::point)
        .collect();
    let origin_residual = project(&origin, n, o)?;
    let mut dp: [Option<usize>; 2] = [None, None];
    for h in net {
        let r = project(&h[..3], n, 0.)?.add(origin_residual.mul(h[3])?)?;
        let possible = [r.lo < 0., r.hi > 0.];
        if !possible[0] && !possible[1] {
            continue;
        }
        let mut next = [None, None];
        for sign in 0..2 {
            if !possible[sign] {
                continue;
            }
            next[sign] = Some(if dp == [None, None] {
                0
            } else {
                (0..2)
                    .filter_map(|old| dp[old].map(|v| v + usize::from(old != sign)))
                    .max()
                    .unwrap()
            });
        }
        dp = next;
    }
    Ok(dp.into_iter().flatten().max().unwrap_or(0))
}
fn linear_parameter_bounds(
    c: &Curve,
    span: usize,
    d: [f64; 2],
    n: [f64; 3],
    o: f64,
) -> Result<Option<[f64; 2]>> {
    let a: Vec<_> = c.control_points[span - 1]
        .iter()
        .copied()
        .map(Interval::point)
        .collect();
    let b: Vec<_> = c.control_points[span]
        .iter()
        .copied()
        .map(Interval::point)
        .collect();
    // A common positive weight scale cancels in q. Bound its division before
    // multiplying plane residuals, avoiding overflow from irrelevant scale.
    let scale = c.weights[span - 1].max(c.weights[span]);
    let weight = |w: f64| -> Result<Interval> {
        if w == scale {
            Ok(Interval::point(1.))
        } else {
            Interval::point(w).div(Interval::point(scale))
        }
    };
    let ra = project(&a, n, o)?.mul(weight(c.weights[span - 1])?)?;
    let rb = project(&b, n, o)?.mul(weight(c.weights[span])?)?;
    let denominator = ra.sub(rb)?;
    if denominator.lo <= 0. && denominator.hi >= 0. {
        return Ok(None);
    }
    let q = ra.div_signed(denominator)?.intersect(0., 1.)?;
    let lo = Interval::point(c.knots[span]);
    let width = Interval::point(c.knots[span + 1]).sub(lo)?;
    let t = lo.add(width.mul(q)?)?.intersect(d[0], d[1])?;
    Ok(Some([t.lo, t.hi]))
}
fn make(c: &Curve, span: usize, d: [f64; 2], n: [f64; 3], o: f64) -> Result<Pending> {
    if c.control_points[span - c.degree..=span]
        .iter()
        .all(|p| exact_plane_zero(p, n, o))
    {
        return Ok(Pending {
            cell: Cell {
                domain: d,
                span,
                residual_bounds: [0., 0.],
                classification: Classification::Coincident,
                stop_reason: StopReason::ExactCoincidence,
                root_multiplicity: None,
                isolated_root_bounds: None,
                proof: Some(Proof::CoplanarControls),
            },
            unique: false,
            root_proof: None,
        });
    }
    let j = curve_jets::enclose(c, span, d)?;
    let mut f = project(&j[0], n, o)?;
    let endpoint_residuals = if c.degree == 1 {
        let a = project(&curve_jets::endpoint(c, span, d, false)?[0], n, o)?;
        let b = project(&curve_jets::endpoint(c, span, d, true)?[0], n, o)?;
        // Every scalar projection of a positive-weight rational line is
        // monotone (possibly constant). Endpoint bounds cover the whole cell,
        // independently of whether its derivative sign can be certified.
        f = f.intersect(a.lo.min(b.lo), a.hi.max(b.hi))?;
        Some((a, b))
    } else {
        None
    };
    let linear_root_bounds = if let Some((a, b)) = endpoint_residuals {
        if (a.hi < 0. && b.lo > 0.) || (b.hi < 0. && a.lo > 0.) {
            linear_parameter_bounds(c, span, d, n, o)?
        } else {
            None
        }
    } else {
        None
    };
    let mut excluded = f.lo > 0. || f.hi < 0.;
    let mut exclusion_proof = Proof::ResidualBounds;
    if let Some((at_start, order)) = bernstein_endpoint_order(c, span, d, n, o)? {
        return Ok(Pending {
            cell: Cell {
                domain: d,
                span,
                residual_bounds: [f.lo, f.hi],
                classification: match (at_start, order) {
                    (true, 1) => Classification::RootAtStart,
                    (false, 1) => Classification::RootAtEnd,
                    (true, 2) => Classification::TangencyAtStart,
                    (false, 2) => Classification::TangencyAtEnd,
                    (true, _) => Classification::HigherOrderContactAtStart,
                    (false, _) => Classification::HigherOrderContactAtEnd,
                },
                stop_reason: if order == 1 {
                    StopReason::ExactEndpoint
                } else if order == 2 {
                    StopReason::ExactTangency
                } else {
                    StopReason::ExactHigherOrderContact
                },
                root_multiplicity: Some(order),
                isolated_root_bounds: None,
                proof: Some(Proof::EndpointCoefficients),
            },
            unique: false,
            root_proof: None,
        });
    }
    if let Some(order) = exact_midpoint_power(c, span, d, n, o) {
        return Ok(Pending {
            cell: Cell {
                domain: d,
                span,
                residual_bounds: [f.lo, f.hi],
                classification: if order == 2 {
                    Classification::TangencyAtMidpoint
                } else {
                    Classification::HigherOrderContactAtMidpoint
                },
                stop_reason: if order == 2 {
                    StopReason::ExactTangency
                } else {
                    StopReason::ExactHigherOrderContact
                },
                root_multiplicity: Some(order),
                isolated_root_bounds: None,
                proof: Some(if c.degree == 2 {
                    Proof::QuadraticSquare
                } else {
                    Proof::BernsteinMidpointPower
                }),
            },
            unique: false,
            root_proof: None,
        });
    }
    let mut unique = false;
    let mut root_proof = None;
    let mut endpoint = None;
    if !excluded {
        let derivative = if c.degree == 1 {
            // For positive weights, the projected rational line derivative
            // has the sign of n·(b-a) everywhere. Its scalar factor is
            // wa*wb*(hi-lo)/[wa(hi-t)+wb(t-lo)]² > 0. No quotient dependency.
            let a = &c.control_points[span - 1];
            let b = &c.control_points[span];
            let direction = (0..3)
                .map(|axis| Interval::point(b[axis]).sub(Interval::point(a[axis])))
                .collect::<Result<Vec<_>>>()?;
            project(&direction, n, 0.)?
        } else {
            project(&j[1], n, 0.)?
        };
        if derivative.lo > 0. || derivative.hi < 0. {
            let lo = c.knots[span];
            let hi = c.knots[span + 1];
            // Multiplicity >= degree makes the corresponding one-sided
            // endpoint equal to an original control, without knot insertion.
            if d[0] == lo
                && c.knots[span - c.degree + 1] == lo
                && exact_plane_zero(&c.control_points[span - c.degree], n, o)
            {
                endpoint = Some(Classification::RootAtStart);
            } else if d[1] == hi
                && c.knots[span + c.degree] == hi
                && (hi == c.domain()[1] || c.knots.iter().filter(|&&k| k == hi).count() <= c.degree)
                && exact_plane_zero(&c.control_points[span], n, o)
            {
                endpoint = Some(Classification::RootAtEnd);
            }
            if endpoint.is_none() {
                if exact_linear_parameter(c, span, d[0], n, o)
                    || exact_quadratic_knot(c, d[0], n, o)
                {
                    endpoint = Some(Classification::RootAtStart);
                } else if exact_linear_parameter(c, span, d[1], n, o)
                    || exact_quadratic_knot(c, d[1], n, o)
                {
                    endpoint = Some(Classification::RootAtEnd);
                }
            }
            let (a, b) = if let Some(pair) = endpoint_residuals {
                pair
            } else {
                (
                    project(&curve_jets::endpoint(c, span, d, false)?[0], n, o)?,
                    project(&curve_jets::endpoint(c, span, d, true)?[0], n, o)?,
                )
            };
            unique = (a.hi < 0. && b.lo > 0.) || (b.hi < 0. && a.lo > 0.);
            if unique {
                root_proof = Some(Proof::MonotonicityWithSignChange);
            }
        }
    }
    if !excluded && !unique && endpoint.is_none() {
        let a = project(&curve_jets::endpoint(c, span, d, false)?[0], n, o)?;
        let b = project(&curve_jets::endpoint(c, span, d, true)?[0], n, o)?;
        let opposite = (a.hi < 0. && b.lo > 0.) || (b.hi < 0. && a.lo > 0.);
        let same = (a.lo > 0. && b.lo > 0.) || (a.hi < 0. && b.hi < 0.);
        if opposite || same {
            let variations = bernstein_variation_upper(c, span, d, n, o)?;
            if same && variations == 0 {
                excluded = true;
                exclusion_proof = Proof::BernsteinZeroVariation;
            }
            if opposite && variations <= 1 {
                // Bernstein Descartes bounds positive interior roots (counting
                // multiplicity). Strict endpoint signs prove existence; <=1
                // therefore proves exactly one simple root without monotonicity.
                unique = true;
                root_proof = Some(Proof::BernsteinOneVariation);
            }
        }
    }
    Ok(Pending {
        cell: Cell {
            domain: d,
            span,
            residual_bounds: [f.lo, f.hi],
            classification: if let Some(endpoint) = endpoint {
                endpoint
            } else if excluded {
                Classification::Excluded
            } else {
                Classification::Unresolved
            },
            stop_reason: if endpoint.is_some() {
                StopReason::ExactEndpoint
            } else {
                StopReason::ParameterTolerance
            },
            root_multiplicity: endpoint.map(|_| 1),
            isolated_root_bounds: linear_root_bounds,
            proof: if excluded {
                Some(exclusion_proof)
            } else {
                endpoint.map(|_| Proof::EndpointWithMonotonicity)
            },
        },
        unique,
        root_proof,
    })
}
/// Isolate intersections with `normal · C(t) = offset` without normalizing
/// or rounding a new plane. Tolerance is in source parameter units.
/// Strict endpoint sign change proves existence; a continuous derivative
/// interval separated from zero proves uniqueness. Exact coplanar active
/// controls prove Coincident. Monotone interpolated span endpoints are exact
/// roots. Weighted quadratic square residuals prove midpoint tangencies.
/// General tangencies and non-interpolated endpoint roots may remain unresolved.
pub fn inspect(
    c: &Curve,
    normal: [f64; 3],
    offset: f64,
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<Report> {
    c.validate()?;
    check(
        c.control_points[0].len() == 3,
        "Plane intersection requires a 3D curve",
    )?;
    check(
        (1..=25).contains(&c.degree) && c.control_points.len() <= 256,
        "Plane intersection admits degree 1..25 and at most 256 controls",
    )?;
    check(
        normal.iter().all(|x| x.is_finite())
            && normal.iter().any(|x| *x != 0.)
            && offset.is_finite(),
        "Plane normal must be finite and nonzero; offset must be finite",
    )?;
    check(
        parameter_tolerance.is_finite() && parameter_tolerance > 0.,
        "Parameter tolerance must be positive and finite",
    )?;
    check(
        (1..=100000).contains(&max_cells),
        "Plane intersection needs 1..100000 cells",
    )?;
    let spans: Vec<_> = (c.degree..c.control_points.len())
        .filter(|&i| c.knots[i] < c.knots[i + 1])
        .collect();
    if spans.len() > max_cells {
        return Err(resource("Initial plane-root coverage exceeds budget"));
    }
    let mut pending = Vec::new();
    for span in spans {
        pending.push(make(
            c,
            span,
            [c.knots[span], c.knots[span + 1]],
            normal,
            offset,
        )?);
    }
    let mut work = pending.len();
    let mut cells = Vec::new();
    while let Some(mut p) = pending.pop() {
        let [lo, hi] = p.cell.domain;
        if let Some([a, b]) = p.cell.isolated_root_bounds {
            if (b - a).next_up() <= parameter_tolerance {
                p.cell.classification = Classification::UniqueRoot;
                p.cell.root_multiplicity = Some(1);
                p.cell.proof = Some(Proof::DirectLinearFormula);
                cells.push(p.cell);
                continue;
            }
        }
        if matches!(
            p.cell.classification,
            Classification::Excluded
                | Classification::Coincident
                | Classification::RootAtStart
                | Classification::RootAtEnd
                | Classification::TangencyAtMidpoint
                | Classification::HigherOrderContactAtMidpoint
                | Classification::TangencyAtStart
                | Classification::TangencyAtEnd
                | Classification::HigherOrderContactAtStart
                | Classification::HigherOrderContactAtEnd
        ) {
            cells.push(p.cell);
            continue;
        }
        // Outward width avoids accepting a rounded-down parameter extent.
        if (hi - lo).next_up() <= parameter_tolerance {
            if p.unique {
                p.cell.classification = Classification::UniqueRoot;
                p.cell.root_multiplicity = Some(1);
                p.cell.proof = p.root_proof;
            } else {
                p.cell.stop_reason = StopReason::RootClassificationNotProven;
            }
            cells.push(p.cell);
            continue;
        }
        let mid = lo * 0.5 + hi * 0.5;
        if work + 2 > max_cells || mid <= lo || mid >= hi {
            p.cell.stop_reason = if work + 2 > max_cells {
                StopReason::WorkLimit
            } else {
                StopReason::PrecisionLimit
            };
            cells.push(p.cell);
            continue;
        }
        pending.push(make(c, p.cell.span, [lo, mid], normal, offset)?);
        pending.push(make(c, p.cell.span, [mid, hi], normal, offset)?);
        work += 2;
    }
    cells.sort_by(|a, b| {
        a.domain[0]
            .total_cmp(&b.domain[0])
            .then(a.span.cmp(&b.span))
    });
    let coverage_resolved = cells
        .iter()
        .all(|x| x.classification != Classification::Unresolved);
    let all_roots_isolated = cells.iter().all(|x| {
        matches!(
            x.classification,
            Classification::Excluded
                | Classification::UniqueRoot
                | Classification::RootAtStart
                | Classification::RootAtEnd
                | Classification::TangencyAtMidpoint
                | Classification::HigherOrderContactAtMidpoint
                | Classification::TangencyAtStart
                | Classification::TangencyAtEnd
                | Classification::HigherOrderContactAtStart
                | Classification::HigherOrderContactAtEnd
        )
    });
    Ok(Report {
        cells,
        work,
        all_roots_isolated,
        coverage_resolved,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn small_bernstein_coefficients_cannot_be_discarded_as_exact_zero() {
        let tiny = 2_f64.powi(-100);
        let c = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![
                vec![0., 0., -1.],
                vec![1., 0., tiny],
                vec![2., 0., -tiny],
                vec![3., 0., 1.],
            ],
            weights: vec![1.; 4],
            periodic: false,
        };
        // The exact source signs are -,+,-,+. Extraction cancellation can
        // widen the tiny coefficients, but must retain the upper bound 3.
        assert_eq!(
            bernstein_variation_upper(&c, 3, [0., 1.], [0., 0., 1.], 0.).unwrap(),
            3
        );
    }
    #[test]
    fn four_factor_products_retain_extreme_exponents_and_low_residuals() {
        let tiny = f64::from_bits(1);
        assert!(!exact_four_products_zero(&[[tiny; 4]]));
        assert!(exact_four_products_zero(&[
            [tiny; 4],
            [-tiny, tiny, tiny, tiny]
        ]));
        let huge = f64::MAX;
        assert!(exact_four_products_zero(&[
            [huge; 4],
            [-huge, huge, huge, huge]
        ]));
        assert!(!exact_four_products_zero(&[
            [huge; 4],
            [-huge, huge, huge, huge],
            [tiny; 4]
        ]));
    }
    #[test]
    fn exact_products_keep_underflow_overflow_and_cancellation_residuals() {
        let tiny = f64::from_bits(1);
        assert!(!exact_plane_zero(&[tiny, 0., 0.], [tiny, 0., 0.], 0.));
        assert!(exact_plane_zero(&[tiny, tiny, 0.], [tiny, -tiny, 0.], 0.));
        assert!(exact_plane_zero(
            &[f64::MAX, f64::MAX, 0.],
            [f64::MAX, -f64::MAX, 0.],
            0.
        ));
        assert!(!exact_plane_zero(&[1e16, 1., 1e16], [1., 1., -1.], 0.));
        assert!(exact_plane_zero(&[1., 1., 1.], [1e308, -1e308, tiny], tiny));
    }
}

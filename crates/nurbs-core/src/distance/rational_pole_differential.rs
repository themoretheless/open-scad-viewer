//! Source-owned exact rational pole decomposition and weighted chord columns.
use crate::{check, interval_eval::Interval as I, normal_alignment, surface::Surface, Result};
use cad_predicates::{
    AuthoredScalar, LeafRef, Limits, PredicateContext, Sign, SourceArena, ToleranceContext,
};
type Net = Vec<Vec<[I; 4]>>;
pub struct Certificate<'a> {
    surface: &'a Surface,
    end: bool,
    anchor: usize,
    a: [I; 3],
    weight: Net,
    lambda: Net,
    alpha: Net,
    e: Net,
    b: Net,
    e_over_lambda: Net,
    fixed_x: Net,
    anchor_position: Net,
}
pub struct Report<'a> {
    pub certificate: Option<Certificate<'a>>,
    pub work: u64,
    pub reason: &'static str,
}
struct Builder<'a> {
    arena: &'a SourceArena,
    leaves: &'a [Vec<[LeafRef; 4]>],
    tolerance: &'a ToleranceContext,
    work: u64,
    limit: u64,
    reason: &'static str,
}
impl Builder<'_> {
    fn query(
        &mut self,
        coordinate: usize,
        axis: usize,
        anchor: Option<usize>,
        row: usize,
        column: Option<usize>,
    ) -> Option<cad_predicates::SurfaceDerivativeDecision> {
        if self.work == self.limit {
            self.reason = "rational-pole-work-limit";
            return None;
        }
        let mut ctx = PredicateContext::new(
            self.arena,
            self.tolerance,
            Limits {
                max_work: (self.limit - self.work).min(cad_predicates::MAX_WORK),
                ..Limits::default()
            },
            None,
        );
        let d = match (anchor, column) {
            (None, None) => cad_predicates::rational_surface_derivative_numerator_row_bounds(
                &mut ctx,
                self.leaves,
                coordinate,
                axis,
                row,
            ),
            (None, Some(c)) => {
                cad_predicates::rational_surface_derivative_numerator_coefficient_bounds(
                    &mut ctx,
                    self.leaves,
                    coordinate,
                    axis,
                    [row, c],
                )
            }
            (Some(a), None) => cad_predicates::rational_surface_derivative_transverse_row_bounds(
                &mut ctx,
                self.leaves,
                coordinate,
                a,
                axis,
                row,
            ),
            (Some(a), Some(c)) => {
                cad_predicates::rational_surface_derivative_transverse_coefficient_bounds(
                    &mut ctx,
                    self.leaves,
                    coordinate,
                    a,
                    axis,
                    [row, c],
                )
            }
        }
        .unwrap();
        self.work += d.work_used;
        Some(d)
    }
    fn grid(
        &mut self,
        coordinate: usize,
        axis: usize,
        anchor: Option<usize>,
        order: usize,
        divisor: I,
    ) -> Result<Option<Vec<Vec<I>>>> {
        let nu = 2 * (self.leaves.len() - 1) - usize::from(axis == 0);
        let nv = 2 * (self.leaves[0].len() - 1) - usize::from(axis == 1);
        let mut net = Vec::new();
        for u in 0..=nu {
            let Some(row) = self.query(coordinate, axis, anchor, u, None) else {
                return Ok(None);
            };
            let mut values = Vec::new();
            for v in 0..=nv {
                let (bounds, sign, scale) = if let (Some(b), Some(s), Some(scale)) =
                    (&row.coefficient_bounds, &row.signs, row.coefficient_scale)
                {
                    (b[0][v], s[0][v], scale)
                } else {
                    let Some(cell) = self.query(coordinate, axis, anchor, u, Some(v)) else {
                        return Ok(None);
                    };
                    let (Some(b), Some(s), Some(scale)) =
                        (cell.coefficient_bounds, cell.signs, cell.coefficient_scale)
                    else {
                        self.reason = "rational-pole-exact-work-unproven";
                        return Ok(None);
                    };
                    (b[0][0], s[0][0], scale)
                };
                if u < order {
                    if sign != Sign::Zero {
                        self.reason = "rational-pole-factor-unproven";
                        return Ok(None);
                    }
                    continue;
                }
                if scale[0] <= 0. {
                    self.reason = "rational-pole-scale-unproven";
                    return Ok(None);
                }
                let mut x = I::new(bounds[0], bounds[1])?
                    .div(I::new(scale[0], scale[1])?)?
                    .div_signed(divisor)?;
                for k in 0..order {
                    x = x.mul(I::point((nu - k) as f64).div(I::point((u - k) as f64))?)?;
                }
                values.push(x);
            }
            if u >= order {
                net.push(values);
            }
        }
        Ok(Some(net))
    }
}
fn empty(u: usize, v: usize) -> Net {
    vec![vec![[I::point(0.), I::point(0.), I::point(0.), I::point(1.)]; v]; u]
}
fn scalar(g: Vec<Vec<I>>) -> Net {
    g.into_iter()
        .map(|r| {
            r.into_iter()
                .map(|x| [x, I::point(0.), I::point(0.), I::point(1.)])
                .collect()
        })
        .collect()
}
fn choose(n: usize, k: usize) -> u64 {
    (0..k).fold(1, |x, i| x * (n - i) as u64 / (i + 1) as u64)
}
fn weight_square(weights: &[Vec<f64>]) -> Result<Net> {
    // Preparation admits degree <= 8. All binomial products below are exact
    // integers below 2^53; only their quotient needs outward rounding.
    let (p, q) = (weights.len() - 1, weights[0].len() - 1);
    let mut out = empty(2 * p + 1, 2 * q + 1);
    for u in 0..=2 * p {
        for v in 0..=2 * q {
            let mut sum = I::point(0.);
            for i in 0..=p {
                for j in 0..=q {
                    if u < i || u - i > p || v < j || v - j > q {
                        continue;
                    }
                    let numerator =
                        (choose(p, i) * choose(p, u - i) * choose(q, j) * choose(q, v - j)) as f64;
                    let denominator = (choose(2 * p, u) * choose(2 * q, v)) as f64;
                    let factor = I::point(numerator).div(I::point(denominator))?;
                    sum = sum.add(
                        I::point(weights[i][j])
                            .mul(I::point(weights[u - i][v - j]))?
                            .mul(factor)?,
                    )?;
                }
            }
            out[u][v][3] = sum;
        }
    }
    Ok(out)
}
fn rational_net(mut net: Net, denominator: &Net) -> Result<Net> {
    // Elevate the original polynomial numerator to W²'s tensor degrees.
    // Packing the common denominator allows restricted coefficient quotients
    // to retain correlations lost by dividing two independent range hulls.
    for axis in 0..2 {
        let target = if axis == 0 {
            denominator.len()
        } else {
            denominator[0].len()
        };
        while (if axis == 0 { net.len() } else { net[0].len() }) < target {
            let (nu, nv) = (net.len(), net[0].len());
            let mut elevated = empty(nu + usize::from(axis == 0), nv + usize::from(axis == 1));
            let degree = if axis == 0 { nu } else { nv };
            for u in 0..elevated.len() {
                for v in 0..elevated[0].len() {
                    let i = if axis == 0 { u } else { v };
                    if i == 0 {
                        elevated[u][v] = net[u][v];
                        continue;
                    }
                    if i == degree {
                        elevated[u][v] =
                            net[u - usize::from(axis == 0)][v - usize::from(axis == 1)];
                        continue;
                    }
                    let t = I::point(i as f64).div(I::point(degree as f64))?;
                    let one = I::point(1.).sub(t)?;
                    for k in 0..3 {
                        elevated[u][v][k] = net[u - usize::from(axis == 0)]
                            [v - usize::from(axis == 1)][k]
                            .mul(t)?
                            .add(net[u][v][k].mul(one)?)?;
                    }
                }
            }
            net = elevated;
        }
    }
    for u in 0..net.len() {
        for v in 0..net[0].len() {
            net[u][v][3] = denominator[u][v][3];
        }
    }
    Ok(net)
}
fn fixed_coordinate_net(b: &Net, e: &Net, lambda: &Net, alpha: &Net) -> Result<Net> {
    let (p, q) = (b.len() - 1, b[0].len() - 1);
    let mut out = empty(2 * p + 1, 2 * q + 1);
    for u in 0..=2 * p {
        for v in 0..=2 * q {
            let mut value = [I::point(0.); 4];
            for i in 0..=p {
                for j in 0..=q {
                    if u < i || u - i > p || v < j || v - j > q {
                        continue;
                    }
                    // Each integer <= choose(32,16) is exact in Binary64; products
                    // use outward interval operations rather than a rounded cast.
                    let mut factor = I::point(1.);
                    for (n, k) in [(p, i), (p, u - i), (q, j), (q, v - j)] {
                        factor = factor.mul(I::point(choose(n, k) as f64))?;
                    }
                    factor = factor
                        .div(I::point(choose(2 * p, u) as f64))?
                        .div(I::point(choose(2 * q, v) as f64))?;
                    let l = lambda[u - i][v - j][0];
                    for k in 0..3 {
                        let n = b[i][j][k]
                            .mul(l)?
                            .sub(e[i][j][k].mul(alpha[u - i][v - j][0])?)?;
                        value[k] = value[k].add(n.mul(factor)?)?;
                    }
                    value[3] = value[3].add(b[i][j][3].mul(l)?.mul(factor)?)?;
                }
            }
            out[u][v] = value;
        }
    }
    Ok(out)
}
/// No input geometry is fitted or changed. Every dropped coefficient has an
/// exact Zero sign; all other magnitudes retain authored coefficient units.
pub fn prepare(s: &Surface, end: bool, max_work: u64) -> Result<Report<'_>> {
    s.validate()?;
    check(
        (1..=100_000_000).contains(&max_work),
        "Bound exact rational pole preparation work",
    )?;
    let (p, q) = (s.degree_u, s.degree_v);
    let single = |d: usize, k: &[f64], n: usize| {
        n == d + 1
            && k.len() == 2 * (d + 1)
            && k[..=d].iter().all(|x| *x == k[0])
            && k[d + 1..].iter().all(|x| *x == k[d + 1])
    };
    check(
        (2..=8).contains(&p)
            && (1..=8).contains(&q)
            && !s.periodic_u
            && !s.periodic_v
            && single(p, &s.knots_u, s.control_points.len())
            && single(q, &s.knots_v, s.control_points[0].len()),
        "Rational pole needs a single clamped original Bezier chart",
    )?;
    let mut source = s.clone();
    if end {
        source.control_points.reverse();
        source.weights.reverse();
    }
    let anchor = (0..3)
        .max_by(|a, b| {
            (source.control_points[1][0][*a] - source.control_points[0][0][*a])
                .abs()
                .total_cmp(
                    &(source.control_points[1][0][*b] - source.control_points[0][0][*b]).abs(),
                )
        })
        .unwrap();
    let mut direction = [I::point(0.); 3];
    for k in 0..3 {
        direction[k] = I::point(source.control_points[1][0][k])
            .sub(I::point(source.control_points[0][0][k]))?;
    }
    if direction[anchor].lo <= 0. && direction[anchor].hi >= 0. {
        return Ok(Report {
            certificate: None,
            work: 0,
            reason: "rational-pole-direction-unproven",
        });
    }
    let mut a = [I::point(0.); 3];
    for k in 0..3 {
        a[k] = if k == anchor {
            I::point(1.)
        } else {
            direction[k].div_signed(direction[anchor])?
        };
    }
    let values = source
        .control_points
        .iter()
        .enumerate()
        .flat_map(|(u, row)| {
            let weights = &source.weights;
            row.iter()
                .enumerate()
                .flat_map(move |(v, p)| [p[0], p[1], p[2], weights[u][v]])
        })
        .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
        .collect();
    let arena = SourceArena::authored("owned-rational-pole-differential", 1, values).unwrap();
    let leaves = (0..=p)
        .map(|u| {
            (0..=q)
                .map(|v| std::array::from_fn(|k| arena.leaf((u * (q + 1) + v) * 4 + k).unwrap()))
                .collect()
        })
        .collect::<Vec<_>>();
    let tolerance = ToleranceContext::default_valid();
    let mut builder = Builder {
        arena: &arena,
        leaves: &leaves,
        tolerance: &tolerance,
        work: 0,
        limit: max_work,
        reason: "rational-pole-unproven",
    };
    let prepared = (|| -> Result<Option<Certificate<'_>>> {
        let Some(lambda) = builder.grid(anchor, 0, None, 0, I::point(1.))? else {
            return Ok(None);
        };
        let Some(alpha) = builder.grid(anchor, 1, None, 1, I::point(1.))? else {
            return Ok(None);
        };
        let mut e = empty(2 * p - 1, 2 * q + 1);
        let mut b = empty(2 * p - 1, 2 * q);
        for k in (0..3).filter(|k| *k != anchor) {
            let Some(eg) = builder.grid(k, 0, Some(anchor), 1, direction[anchor])? else {
                return Ok(None);
            };
            let Some(bg) = builder.grid(k, 1, Some(anchor), 2, direction[anchor])? else {
                return Ok(None);
            };
            for u in 0..eg.len() {
                for v in 0..eg[0].len() {
                    e[u][v][k] = eg[u][v];
                }
            }
            for u in 0..bg.len() {
                for v in 0..bg[0].len() {
                    b[u][v][k] = bg[u][v];
                }
            }
        }
        let mut weight = empty(p + 1, q + 1);
        for u in 0..=p {
            for v in 0..=q {
                weight[u][v][0] = I::point(source.weights[u][v]);
            }
        }
        let mut anchor_position = empty(p + 1, q + 1);
        let origin = source.control_points[0][0][anchor];
        for u in 0..=p {
            for v in 0..=q {
                let w = I::point(source.weights[u][v]);
                anchor_position[u][v][0] = I::point(source.control_points[u][v][anchor])
                    .sub(I::point(origin))?
                    .mul(w)?;
                anchor_position[u][v][3] = w;
            }
        }
        let denominator = weight_square(&source.weights)?;
        let lambda = rational_net(scalar(lambda), &denominator)?;
        let alpha = rational_net(scalar(alpha), &denominator)?;
        let e = rational_net(e, &denominator)?;
        let b = rational_net(b, &denominator)?;
        let fixed_x = fixed_coordinate_net(&b, &e, &lambda, &alpha)?;
        let mut e_over_lambda = e.clone();
        for u in 0..e.len() {
            for v in 0..e[0].len() {
                e_over_lambda[u][v][3] = lambda[u][v][0];
            }
        }
        Ok(Some(Certificate {
            surface: s,
            end,
            anchor,
            a,
            weight,
            lambda,
            alpha,
            e,
            b,
            e_over_lambda,
            fixed_x,
            anchor_position,
        }))
    })()?;
    let reason = if prepared.is_some() {
        "rational-pole-original-decomposition-qualified"
    } else {
        builder.reason
    };
    Ok(Report {
        certificate: prepared,
        work: builder.work,
        reason,
    })
}
impl<'a> Certificate<'a> {
    pub fn surface(&self) -> &'a Surface {
        self.surface
    }
    pub(crate) fn end(&self) -> bool {
        self.end
    }
    pub fn anchor(&self) -> usize {
        self.anchor
    }
    fn normalized_domain(&self, domain: [[f64; 2]; 2]) -> Result<[[f64; 2]; 2]> {
        let s = self.surface;
        let original = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        check(
            (0..2).all(|a| {
                domain[a][0].is_finite()
                    && domain[a][1].is_finite()
                    && original[a][0] <= domain[a][0]
                    && domain[a][0] <= domain[a][1]
                    && domain[a][1] <= original[a][1]
            }),
            "Rational pole rectangle must lie in its original chart",
        )?;
        let mut d = [[0.; 2]; 2];
        for a in 0..2 {
            let length = I::point(original[a][1]).sub(I::point(original[a][0]))?;
            for e in 0..2 {
                let x = I::point(domain[a][e])
                    .sub(I::point(original[a][0]))?
                    .div(length)?
                    .intersect(0., 1.)?;
                d[a][e] = if e == 0 { x.lo } else { x.hi };
            }
        }
        if self.end {
            let lo = I::point(1.).sub(I::point(d[0][1]))?;
            let hi = I::point(1.).sub(I::point(d[0][0]))?;
            d[0] = [lo.lo.max(0.), hi.hi.min(1.)];
        }
        Ok(d)
    }
    /// Two-leg path in (original anchor coordinate, V). The separated signed
    /// lambda proves unique U for every point of the entire path. This refuses
    /// when the enlarged original chart does not contain that path.
    pub fn coordinate_path_bounds(&self, domain: [[f64; 2]; 2]) -> Result<Option<[[I; 2]; 3]>> {
        Ok(self.coordinate_path_report(domain)?.0)
    }
    /// Return original path bounds together with charged net restrictions.
    /// A missing matrix supplies no path proof; preparation work is separate.
    pub fn coordinate_path_report(&self, domain: [[f64; 2]; 2])
        -> Result<(Option<[[I; 2]; 3]>, usize)> {
        let d = self.normalized_domain(domain)?;
        let mut extended = d;
        extended[0] = [0., I::point(d[0][1]).mul(I::point(4.))?.hi.min(1.)];
        let mut used = 1;
        let weight = normal_alignment::restrict_hull(
            &self.weight,
            [self.weight.len() - 1, self.weight[0].len() - 1],
            [[0., 1.], [0., 1.]],
            extended,
        )?;
        if weight[0].lo <= 0. {
            return Ok((None, used));
        }
        // Endpoint X is an integral from the pole, so its derivative bound
        // must cover that entire prefix, not only the endpoint rectangle.
        let mut prefix = d;
        prefix[0][0] = 0.;
        used += 1;
        let endpoint = normal_alignment::rational_quotient_hull(&self.lambda, prefix)?[0];
        used += 1;
        let lambda = normal_alignment::rational_quotient_hull(&self.lambda, extended)?[0];
        if lambda.lo <= 0. && lambda.hi >= 0. {
            return Ok((None, used));
        }
        let (minimum, maximum) = if lambda.lo > 0. {
            (lambda.lo, endpoint.hi)
        } else {
            (-lambda.hi, -endpoint.lo)
        };
        let available = I::point(extended[0][1]).mul(I::point(minimum))?;
        let needed = I::point(d[0][1]).mul(I::point(maximum))?;
        if available.lo < needed.hi {
            return Ok((None, used));
        }
        used += 1;
        let Some(ratio) =
            normal_alignment::signed_rational_quotient_hull(&self.e_over_lambda, extended)?
        else {
            return Ok((None, used));
        };
        used += 1;
        let Some(second) =
            normal_alignment::signed_rational_quotient_hull(&self.fixed_x, extended)?
        else {
            return Ok((None, used));
        };
        let mut j = [[I::point(0.); 2]; 3];
        for k in 0..3 {
            j[k][0] = self.a[k].add(I::new(0., extended[0][1])?.mul(ratio[k])?)?;
            j[k][1] = second[k];
        }
        Ok((Some(j), used))
    }
    /// Confine the inverse-coordinate path between two local U boundaries.
    /// Six net restrictions are required, separate from the pole-prefix path.
    pub fn local_coordinate_path_bounds(
        &self,
        domain: [[f64; 2]; 2],
    ) -> Result<Option<[[I; 2]; 3]>> {
        self.local_coordinate_path_bounds_with_margin(domain, 3.)
    }
    /// Every margin is independently confined; a smaller chart is never
    /// accepted merely because its endpoint derivatives are regular.
    pub(crate) fn local_coordinate_path_bounds_with_margin(
        &self, domain: [[f64; 2]; 2], factor: f64,
    ) -> Result<Option<[[I; 2]; 3]>> {
        Ok(self.local_coordinate_path_report(domain, factor)?.0)
    }
    /// Counts every executed original net restriction, including refusals.
    /// Callers reserve six before entry, so even a successful path stays bounded.
    pub(crate) fn local_coordinate_path_report(
        &self, domain: [[f64; 2]; 2], factor: f64,
    ) -> Result<(Option<[[I; 2]; 3]>, usize)> {
        check(factor.is_finite() && factor > 0., "Positive inverse path margin")?;
        let d = self.normalized_domain(domain)?;
        let width = I::point(d[0][1]).sub(I::point(d[0][0]))?;
        let margin = I::point(width.hi.max(2_f64.powi(-20))).mul(I::point(factor))?;
        let extended = [
            [
                I::point(d[0][0]).sub(margin)?.lo.max(0.),
                I::point(d[0][1]).add(margin)?.hi.min(1.),
            ],
            d[1],
        ];
        let mut used = 1;
        let lambda = normal_alignment::rational_quotient_hull(&self.lambda, extended)?[0];
        if lambda.lo <= 0. && lambda.hi >= 0. {
            return Ok((None, used));
        }
        let orient = |x: I| {
            if lambda.lo > 0. {
                Ok(x)
            } else {
                I::point(0.).sub(x)
            }
        };
        used += 1;
        let mut endpoint =
            orient(normal_alignment::rational_quotient_hull(&self.anchor_position, d)?[0])?;
        used += 1;
        let mut left = orient(
            normal_alignment::rational_quotient_hull(
                &self.anchor_position,
                [[extended[0][0]; 2], d[1]],
            )?[0],
        )?;
        used += 1;
        let right = orient(
            normal_alignment::rational_quotient_hull(
                &self.anchor_position,
                [[extended[0][1]; 2], d[1]],
            )?[0],
        )?;
        if extended[0][0] == 0. {
            // Exact Q_v/U factors prove the pole rail constant. Monotonicity
            // then gives signed X>=0; discard only outward rounding below zero.
            left = I::point(0.);
            endpoint = I::new(endpoint.lo.max(0.), endpoint.hi)?;
        }
        if left.hi > endpoint.lo || right.lo < endpoint.hi {
            return Ok((None, used));
        }
        used += 1;
        let Some(ratio) =
            normal_alignment::signed_rational_quotient_hull(&self.e_over_lambda, extended)?
        else {
            return Ok((None, used));
        };
        used += 1;
        let Some(second) =
            normal_alignment::signed_rational_quotient_hull(&self.fixed_x, extended)?
        else {
            return Ok((None, used));
        };
        let mut j = [[I::point(0.); 2]; 3];
        for k in 0..3 {
            j[k][0] = self.a[k].add(I::new(extended[0][0], extended[0][1])?.mul(ratio[k])?)?;
            j[k][1] = second[k];
        }
        Ok((Some(j), used))
    }
    /// Columns enclosing the weighted mean chord matrix and both endpoint tangent
    /// frames. A separated signed longitudinal derivative is required everywhere.
    pub fn bounds(&self, domain: [[f64; 2]; 2]) -> Result<Option<[[I; 2]; 3]>> {
        Ok(self.bounds_report(domain)?.0)
    }
    pub(crate) fn bounds_report(&self, domain: [[f64; 2]; 2])
        -> Result<(Option<[[I; 2]; 3]>, usize)> {
        let d = self.normalized_domain(domain)?;
        let hull = |net: &Net| {
            normal_alignment::restrict_hull(
                net,
                [net.len() - 1, net[0].len() - 1],
                [[0., 1.], [0., 1.]],
                d,
            )
        };
        let mut used = 1;
        let w = hull(&self.weight)?[0];
        check(w.lo > 0., "Positive original rational pole denominator")?;
        let quotient = |net: &Net| normal_alignment::rational_quotient_hull(net, d);
        used += 1;
        let lambda = quotient(&self.lambda)?[0];
        if lambda.lo <= 0. && lambda.hi >= 0. {
            return Ok((None, used));
        }
        used += 1;
        let alpha = quotient(&self.alpha)?[0];
        used += 1;
        let eh = quotient(&self.e)?;
        used += 1;
        let bh = quotient(&self.b)?;
        let first = I::new(0., d[0][1])?.div_signed(lambda)?;
        let second = alpha.div_signed(lambda)?.mul(I::new(0., 1.)?)?;
        let mut j = [[I::point(0.); 2]; 3];
        for k in 0..3 {
            let e = eh[k];
            let b = bh[k];
            j[k][0] = self.a[k].add(e.mul(first)?)?;
            j[k][1] = b.sub(e.mul(second)?)?;
        }
        Ok((Some(j), used))
    }
}
#[cfg(test)]
#[path = "rational_pole_differential/tests/mod.rs"]
mod tests;

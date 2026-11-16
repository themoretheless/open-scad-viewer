//! Isolate a stationary zero using curvature near the contact and strict
//! Bernstein gap bounds away from it.
use super::*;
use num_traits::Signed;

#[derive(Clone, Debug)]
pub struct QualifiedConvexGraphContact {
    source: FiberSources,
    center: [Q; 2],
    second_uv: UvMap,
    max_cells: usize,
}

impl QualifiedConvexGraphContact {
    /// Exact normalized U and V of the first original surface.
    pub fn center(&self) -> &[Q; 2] {
        &self.center
    }

    pub fn native_parameters(&self) -> [Q; 4] {
        native_parameters(&self.source, &self.center, self.second_uv)
    }

    pub fn reverify(&self) -> crate::Result<()> {
        qualify_convex_graph_contact(&self.source, self.center.clone(), self.max_cells).map(|_| ())
    }
}

type Net = Vec<Vec<Q>>;

/// Prove that the two whole supports meet at exactly the supplied rational UV
/// point of the first support. A zero value and gradient are checked exactly;
/// each cell must prove a strict oriented gap away from the center. Cells
/// meeting the center can use nonnegative Bernstein coefficients or a
/// semidefinite Hessian. For rank one the kernel line must carry a nonzero gap.
/// At rank zero, a strict gap on the boundary except at the contact rules
/// out any zero line crossing the domain.
pub fn qualify_convex_graph_contact(
    source: &FiberSources,
    center: [Q; 2],
    max_cells: usize,
) -> crate::Result<QualifiedConvexGraphContact> {
    crate::check(
        max_cells > 0 && center.iter().all(|q| !q.is_negative() && *q <= Q::one()),
        "Convex contact needs an in-domain center and positive work allowance",
    )?;
    qualify_gap(source, center, max_cells, common_gap(source)?)
}

fn qualify_gap(
    source: &FiberSources,
    center: [Q; 2],
    max_cells: usize,
    gap: CommonGap,
) -> crate::Result<QualifiedConvexGraphContact> {
    let CommonGap {
        coefficients,
        second_uv,
    } = gap;
    let u = derivative(&coefficients, 0);
    let v = derivative(&coefficients, 1);
    crate::check(
        [&coefficients, &u, &v]
            .into_iter()
            .all(|net| value(net, &center).is_zero()),
        "Original gap does not have a zero stationary value at this center",
    )?;
    let uu = derivative(&u, 0);
    let vv = derivative(&v, 1);
    let uv = derivative(&u, 1);
    let a = value(&uu, &center);
    let b = value(&vv, &center);
    let c = value(&uv, &center);
    let det = &a * &b - c.pow(2);
    crate::check(!det.is_negative(), "Stationary Hessian is indefinite")?;
    let curvature = if a.is_zero() { &b } else { &a };
    let rank_zero = curvature.is_zero();
    let negative = if rank_zero {
        let [i, j] = if center.iter().all(Q::is_zero) {
            [coefficients.len() - 1, 0]
        } else {
            [0, 0]
        };
        coefficients[i][j].is_negative()
    } else {
        curvature.is_negative()
    };
    let mut cells = if rank_zero {
        certify_boundary_gap(&coefficients, &center, negative, max_cells)?
    } else {
        0
    };
    if !rank_zero && det.is_zero() {
        let direction = if a.is_zero() {
            [Q::one(), Q::zero()]
        } else {
            [-c, a.clone()]
        };
        crate::check(
            line_is_nonzero(&coefficients, &center, &direction),
            "Original gap vanishes on the stationary Hessian kernel line",
        )?;
    }
    crate::check(
        coefficients.len() >= 3 && coefficients[0].len() >= 3,
        "Contact Hessian needs curvature in both original axes somewhere",
    )?;

    // Test the determinant as a single polynomial to retain correlations at
    // zero curvature, rather than bounding the products independently.
    let mut determinant = product(&uu, &vv);
    for (row, subtract) in determinant.iter_mut().zip(product(&uv, &uv)) {
        for (q, r) in row.iter_mut().zip(subtract) {
            *q -= r;
        }
    }
    let mut minors = [uu, vv, determinant];
    let mut gap = coefficients;
    if negative {
        for q in minors[..2]
            .iter_mut()
            .chain(std::iter::once(&mut gap))
            .flatten()
            .flatten()
        {
            *q = -&*q;
        }
    }
    let mut pending = vec![ContactCell {
        gap,
        minors,
        domain: std::array::from_fn(|_| [Q::zero(), Q::one()]),
        axis: 0,
    }];
    let half = Q::new(1.into(), 2.into());
    let mid = [half.clone(), half.clone()];
    while let Some(cell) = pending.pop() {
        if cells == max_cells {
            return Err(crate::resource(
                "Convex contact proof exhausted its cell budget",
            ));
        }
        cells += 1;
        if cell.proves_gap_except_center(&center) {
            continue;
        }
        let touches =
            (0..2).all(|k| cell.domain[k][0] <= center[k] && center[k] <= cell.domain[k][1]);
        if touches
            && cell
                .minors
                .iter()
                .flatten()
                .flatten()
                .all(|q| !q.is_negative())
        {
            continue;
        }
        let gap_at_mid = value(&cell.gap, &mid);
        let at_center =
            (0..2).all(|k| (&cell.domain[k][0] + &cell.domain[k][1]) * &half == center[k]);
        crate::check(
            gap_at_mid.is_positive() || at_center && gap_at_mid.is_zero(),
            "Original gap has a second zero or a sign change",
        )?;
        let [lo, hi] = &cell.domain[cell.axis];
        let cut = if det.is_zero() && *lo < center[cell.axis] && center[cell.axis] < *hi {
            (&center[cell.axis] - lo) / (hi - lo)
        } else {
            half.clone()
        };
        pending.extend(cell.split(&cut));
    }
    Ok(QualifiedConvexGraphContact {
        source: source.clone(),
        center,
        second_uv,
        max_cells,
    })
}

/// Search for a rational stationary point. Proposals never provide geometry
/// authority: the exact zero and whole-domain Hessian must both pass.
pub fn discover_convex_graph_contact(
    source: &FiberSources,
    max_cells: usize,
) -> crate::Result<Option<QualifiedConvexGraphContact>> {
    crate::check(
        max_cells > 0,
        "Convex contact needs a positive work allowance",
    )?;
    let gap = match common_gap(source) {
        Ok(gap) => gap,
        Err(error) if error.code == crate::RESOURCE_LIMIT => return Err(error),
        Err(_) => return Ok(None),
    };
    let gradient = [
        derivative(&gap.coefficients, 0),
        derivative(&gap.coefficients, 1),
    ];
    let hessian = [
        derivative(&gradient[0], 0),
        derivative(&gradient[0], 1),
        derivative(&gradient[1], 1),
    ];
    let mut at = [0.5_f64; 2];
    for _ in 0..16 {
        let exact = at.map(|x| Q::from_float(x).unwrap());
        let proposals =
            [16, 256, 65536].map(|limit| exact.each_ref().map(|q| rational_proposal(q, limit)));
        for center in proposals.into_iter().chain(std::iter::once(exact.clone())) {
            if center.iter().any(|q| q.is_negative() || *q > Q::one())
                || ![&gradient[0], &gradient[1], &gap.coefficients]
                    .into_iter()
                    .all(|net| value(net, &center).is_zero())
            {
                continue;
            }
            return match qualify_gap(source, center, max_cells, gap) {
                Ok(owner) => Ok(Some(owner)),
                Err(error) if error.code == crate::RESOURCE_LIMIT => Err(error),
                Err(_) => Ok(None),
            };
        }
        let [Some(du), Some(dv)] = gradient.each_ref().map(|net| value(net, &exact).to_f64())
        else {
            break;
        };
        let [Some(uu), Some(uv), Some(vv)] =
            hessian.each_ref().map(|net| value(net, &exact).to_f64())
        else {
            break;
        };
        let det = uu * vv - uv * uv;
        let step = if det > 0. {
            [(vv * du - uv * dv) / det, (uu * dv - uv * du) / det]
        } else if uv == 0. && vv == 0. && dv == 0. && uu != 0. {
            [du / uu, 0.]
        } else if uv == 0. && uu == 0. && du == 0. && vv != 0. {
            [0., dv / vv]
        } else {
            break;
        };
        if step.iter().any(|x| !x.is_finite()) {
            break;
        }
        let next = (0..32).find_map(|i| {
            let scale = 0.5_f64.powi(i);
            let candidate = [at[0] - scale * step[0], at[1] - scale * step[1]];
            candidate
                .iter()
                .all(|p| (0. ..=1.).contains(p))
                .then_some(candidate)
        });
        match next {
            Some(next) if next != at => at = next,
            _ => break,
        }
    }
    Ok(None)
}

// Continued-fraction reconstruction supplies compact nonbinary proposals.
fn rational_proposal(at: &Q, max_denominator: u64) -> Q {
    let mut remainder = at.clone();
    let [mut p0, mut p1, mut q0, mut q1] = [0u64, 1, 1, 0];
    loop {
        let Some(a) = remainder
            .to_integer()
            .to_u64()
            .filter(|a| *a <= max_denominator)
        else {
            break;
        };
        let q2 = q0 + a * q1;
        if q2 > max_denominator {
            break;
        }
        (p0, p1, q0, q1) = (p1, p0 + a * p1, q1, q2);
        remainder -= Q::from_integer(a.into());
        if remainder.is_zero() {
            break;
        }
        remainder = remainder.recip();
    }
    Q::new(p1.into(), q1.into())
}

// Convexity would force a second zero to lie on a zero segment from the
// center. A polynomial vanishing on a segment vanishes on its entire line,
// which would contradict a strict sign where that line meets the boundary.
fn certify_boundary_gap(
    net: &Net,
    center: &[Q; 2],
    negative: bool,
    max_cells: usize,
) -> crate::Result<usize> {
    let transposed = transpose(net);
    let mut pending: Vec<_> = [net, &transposed]
        .into_iter()
        .enumerate()
        .flat_map(|(fixed, net)| {
            [&net[0], net.last().unwrap()]
                .into_iter()
                .enumerate()
                .map(move |(end, edge)| {
                    let zero = (center[fixed] == Q::from_integer(end.into()))
                        .then(|| center[1 - fixed].clone());
                    let restricted: Net = edge
                        .iter()
                        .map(|q| vec![if negative { -q } else { q.clone() }])
                        .collect();
                    (restricted, zero)
                })
        })
        .collect();
    let mut cells = 0;
    let half = Q::new(1.into(), 2.into());
    while let Some((edge, zero)) = pending.pop() {
        if cells == max_cells {
            return Err(crate::resource(
                "Convex contact boundary proof exhausted its cell budget",
            ));
        }
        cells += 1;
        let zero_at_start = zero.as_ref().is_some_and(Q::is_zero);
        let zero_at_end = zero.as_ref().is_some_and(Q::is_one);
        crate::check(
            (zero_at_start || edge[0][0].is_positive())
                && (zero_at_end || edge.last().unwrap()[0].is_positive()),
            "Original gap is not strictly signed away from the boundary contact",
        )?;
        let interior_zero = zero.as_ref().filter(|_| !zero_at_start && !zero_at_end);
        if interior_zero.is_none() && edge.iter().flatten().all(|q| !q.is_negative()) {
            continue;
        }
        let [left, right] = split(&edge, 0, interior_zero.unwrap_or(&half));
        pending.extend([
            (
                right,
                if interior_zero.is_some() {
                    Some(Q::zero())
                } else {
                    zero_at_end.then(Q::one)
                },
            ),
            (
                left,
                if interior_zero.is_some() {
                    Some(Q::one())
                } else {
                    zero_at_start.then(Q::zero)
                },
            ),
        ]);
    }
    Ok(cells)
}

// Every direction outside the kernel has strict curvature at the center.
// On the kernel line, convexity and polynomial identity rule out a zero
// segment once one exact value on that line differs from zero.
fn line_is_nonzero(net: &Net, center: &[Q; 2], direction: &[Q; 2]) -> bool {
    let ray_limit = |direction: &[Q; 2]| {
        (0..2)
            .filter(|&k| !direction[k].is_zero())
            .map(|k| {
                if direction[k].is_positive() {
                    (Q::one() - &center[k]) / &direction[k]
                } else {
                    -&center[k] / &direction[k]
                }
            })
            .min()
            .unwrap()
    };
    let mut limit = ray_limit(direction);
    if limit.is_zero() {
        limit = -ray_limit(&direction.each_ref().map(|q| -q));
    }
    if limit.is_zero() {
        return true;
    }
    let degree = net.len() + net[0].len() - 2;
    (1..=degree).any(|i| {
        let step = &limit * Q::new(i.into(), degree.into());
        let on_line = std::array::from_fn(|k| &center[k] + &step * &direction[k]);
        !value(net, &on_line).is_zero()
    })
}

struct ContactCell {
    gap: Net,
    minors: [Net; 3],
    domain: [[Q; 2]; 2],
    axis: usize,
}

impl ContactCell {
    fn proves_gap_except_center(&self, center: &[Q; 2]) -> bool {
        let last = [self.gap.len() - 1, self.gap[0].len() - 1];
        // Every point other than the allowed zero corner has a positive
        // Bernstein basis weight for at least one of the other three corners.
        [(0, 0), (0, 1), (1, 0), (1, 1)].into_iter().all(|(i, j)| {
            self.gap[i * last[0]][j * last[1]].is_positive()
                || self.domain[0][i] == center[0] && self.domain[1][j] == center[1]
        }) && self.gap.iter().flatten().all(|q| !q.is_negative())
    }

    fn split(self, cut: &Q) -> [Self; 2] {
        let [left, right] = split(&self.gap, self.axis, cut);
        let [[a0, a1], [b0, b1], [d0, d1]] =
            self.minors.each_ref().map(|net| split(net, self.axis, cut));
        let [lo, hi] = &self.domain[self.axis];
        let at = lo + (hi - lo) * cut;
        let mut low_domain = self.domain.clone();
        let mut high_domain = self.domain;
        low_domain[self.axis][1] = at.clone();
        high_domain[self.axis][0] = at;
        let axis = 1 - self.axis;
        [
            Self {
                gap: right,
                minors: [a1, b1, d1],
                domain: high_domain,
                axis,
            },
            Self {
                gap: left,
                minors: [a0, b0, d0],
                domain: low_domain,
                axis,
            },
        ]
    }
}

fn product(a: &Net, b: &Net) -> Net {
    let p = [a.len() - 1, a[0].len() - 1];
    let q = [b.len() - 1, b[0].len() - 1];
    let degree = [p[0] + q[0], p[1] + q[1]];
    let weights = |axis: usize| -> Net {
        (0..=p[axis])
            .map(|i| {
                (0..=q[axis])
                    .map(|j| {
                        Q::new(
                            (choose(p[axis], i) * choose(q[axis], j)).into(),
                            choose(degree[axis], i + j).into(),
                        )
                    })
                    .collect()
            })
            .collect()
    };
    let (u, v) = (weights(0), weights(1));
    let mut result = vec![vec![Q::zero(); degree[1] + 1]; degree[0] + 1];
    for (i, a_row) in a.iter().enumerate() {
        for (j, a_value) in a_row.iter().enumerate() {
            if a_value.is_zero() {
                continue;
            }
            for (k, b_row) in b.iter().enumerate() {
                for (l, b_value) in b_row.iter().enumerate() {
                    if !b_value.is_zero() {
                        result[i + k][j + l] += a_value * b_value * &u[i][k] * &v[j][l];
                    }
                }
            }
        }
    }
    result
}

fn transpose(net: &Net) -> Net {
    (0..net[0].len())
        .map(|j| net.iter().map(|row| row[j].clone()).collect())
        .collect()
}

fn derivative(net: &Net, axis: usize) -> Net {
    if axis == 1 {
        return transpose(&derivative(&transpose(net), 0));
    }
    if net.len() == 1 {
        return vec![vec![Q::zero(); net[0].len()]];
    }
    let degree = Q::from_integer((net.len() - 1).into());
    net.windows(2)
        .map(|rows| {
            rows[0]
                .iter()
                .zip(&rows[1])
                .map(|(a, b)| (b - a) * &degree)
                .collect()
        })
        .collect()
}

fn line_value(mut line: Vec<Q>, at: &Q) -> Q {
    let one_minus = Q::one() - at;
    for length in (1..line.len()).rev() {
        for i in 0..length {
            line[i] = &line[i] * &one_minus + &line[i + 1] * at;
        }
    }
    line.remove(0)
}

fn value(net: &Net, at: &[Q; 2]) -> Q {
    line_value(
        net.iter()
            .map(|row| line_value(row.clone(), &at[1]))
            .collect(),
        &at[0],
    )
}

fn split(net: &Net, axis: usize, at: &Q) -> [Net; 2] {
    if axis == 1 {
        return split(&transpose(net), 0, at).map(|part| transpose(&part));
    }
    let one_minus = Q::one() - at;
    let mut net = net.clone();
    let mut left = Vec::new();
    let mut right = Vec::new();
    while net.len() > 1 {
        left.push(net[0].clone());
        right.push(net.last().unwrap().clone());
        net = net
            .windows(2)
            .map(|rows| {
                rows[0]
                    .iter()
                    .zip(&rows[1])
                    .map(|(a, b)| a * &one_minus + b * at)
                    .collect()
            })
            .collect();
    }
    left.push(net[0].clone());
    right.push(net.remove(0));
    right.reverse();
    [left, right]
}

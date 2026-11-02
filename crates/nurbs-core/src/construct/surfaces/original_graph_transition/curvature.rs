//! Outward Bernstein enclosure of the exact construction, never its preview.
use super::*;
use crate::{distance_bounds::Interval as I, surface_shape_operator};
type ExactNet = Vec<Vec<[Q; 3]>>;
type Net = Vec<Vec<[I; 3]>>;
type Region = [[f64; 2]; 2];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurvatureDecision {
    Within,
    Exceeds,
    Unresolved,
}
#[derive(Debug)]
pub struct CurvatureReport {
    pub decision: CurvatureDecision,
    pub absolute_curvature_upper: Option<f64>,
    pub cells: usize,
    pub covered_cells: usize,
    pub coefficient_work: usize,
    pub unresolved_regions: Vec<Region>,
    pub exceeds_region: Option<Region>,
    pub reason: &'static str,
}
pub(super) fn derivative(net: &ExactNet, axis: usize, b: &mut Budget) -> Result<ExactNet> {
    let nu = net.len() - usize::from(axis == 0);
    let nv = net[0].len() - usize::from(axis == 1);
    let degree = if axis == 0 {
        net.len() - 1
    } else {
        net[0].len() - 1
    };
    let factor = Q::from_integer(degree.into());
    (0..nu)
        .map(|u| {
            (0..nv)
                .map(|v| {
                    let next = &net[u + usize::from(axis == 0)][v + usize::from(axis == 1)];
                    let mut row = std::array::from_fn(|_| Q::zero());
                    for k in 0..3 {
                        row[k] = b.keep((&next[k] - &net[u][v][k]) * &factor)?;
                    }
                    Ok(row)
                })
                .collect()
        })
        .collect()
}
pub(super) fn enclose(q: &Q, b: &mut Budget) -> Result<I> {
    b.charge(1)?;
    let x = q
        .to_f64()
        .filter(|x| x.is_finite())
        .ok_or_else(|| crate::numeric_err("Exact graph curvature overflow"))?;
    let mut lo = x;
    let mut hi = x;
    for _ in 0..4 {
        if Q::from_float(lo).unwrap() > *q {
            lo = lo.next_down();
        }
        if Q::from_float(hi).unwrap() < *q {
            hi = hi.next_up();
        }
        if lo.is_finite()
            && hi.is_finite()
            && Q::from_float(lo).unwrap() <= *q
            && Q::from_float(hi).unwrap() >= *q
        {
            return I::new(lo, hi);
        }
    }
    Err(crate::numeric_err("Cannot enclose exact graph coefficient"))
}
fn enclosed(net: &ExactNet, b: &mut Budget) -> Result<Net> {
    net.iter()
        .map(|row| {
            row.iter()
                .map(|p| Ok([enclose(&p[0], b)?, enclose(&p[1], b)?, enclose(&p[2], b)?]))
                .collect()
        })
        .collect()
}
fn root(p: &Patch, b: &mut Budget) -> Result<Vec<Net>> {
    let u = derivative(&p.coefficients, 0, b)?;
    let v = derivative(&p.coefficients, 1, b)?;
    let uu = derivative(&u, 0, b)?;
    let uv = derivative(&u, 1, b)?;
    let vv = derivative(&v, 1, b)?;
    [&u, &v, &uu, &uv, &vv]
        .into_iter()
        .map(|n| enclosed(n, b))
        .collect()
}
fn split_line(a: &[[I; 3]], b: &mut Budget) -> Result<(Vec<[I; 3]>, Vec<[I; 3]>)> {
    let mut d = a.to_vec();
    let mut left = vec![d[0]];
    let mut right = vec![*d.last().unwrap()];
    for n in (1..d.len()).rev() {
        for i in 0..n {
            b.charge(3)?;
            for k in 0..3 {
                let (x, y) = (d[i][k], d[i + 1][k]);
                d[i][k] = if x.lo == y.lo && x.hi == y.hi {
                    x
                } else {
                    x.add(y)?.mul(I::point(0.5))?
                };
            }
        }
        left.push(d[0]);
        right.push(d[n - 1]);
    }
    right.reverse();
    Ok((left, right))
}
fn split(net: &Net, axis: usize, b: &mut Budget) -> Result<(Net, Net)> {
    if axis == 1 {
        let mut left = Vec::new();
        let mut right = Vec::new();
        for row in net {
            let (l, r) = split_line(row, b)?;
            left.push(l);
            right.push(r);
        }
        return Ok((left, right));
    }
    let (nu, nv) = (net.len(), net[0].len());
    let mut left = vec![vec![[I::point(0.); 3]; nv]; nu];
    let mut right = left.clone();
    for v in 0..nv {
        let line = net.iter().map(|r| r[v]).collect::<Vec<_>>();
        let (l, r) = split_line(&line, b)?;
        for u in 0..nu {
            left[u][v] = l[u];
            right[u][v] = r[u];
        }
    }
    Ok((left, right))
}
fn bound(nets: &[Net]) -> Result<Option<[f64; 2]>> {
    let mut jets = [[[I::point(0.); 3]; 4]; 4];
    for (net, (u, v)) in nets.iter().zip([(1, 0), (0, 1), (2, 0), (1, 1), (0, 2)]) {
        for k in 0..3 {
            let lo = net
                .iter()
                .flatten()
                .map(|p| p[k].lo)
                .fold(f64::INFINITY, f64::min);
            let hi = net
                .iter()
                .flatten()
                .map(|p| p[k].hi)
                .fold(f64::NEG_INFINITY, f64::max);
            jets[u][v][k] = I::new(lo, hi)?;
        }
    }
    let Some(shape) = surface_shape_operator::from_graph_jets(&jets, ([0, 1, 2], [0., 0.]))? else {
        return Ok(None);
    };
    surface_shape_operator::absolute_spectral_bounds(shape.tensor.map(|r| r.map(|x| [x.lo, x.hi])))
}
impl Patch {
    /// Fresh complete-field proof over normalized patch parameters. Construction
    /// and proof share `budget`; partial coverage never produces an upper bound.
    pub fn inspect_curvature(
        &self,
        limit: f64,
        max_cells: usize,
        budget: &mut Budget,
    ) -> Result<CurvatureReport> {
        check(
            limit.is_finite() && limit > 0. && max_cells <= 1_000_000,
            "Invalid exact graph curvature limits",
        )?;
        let start = budget.used();
        let full = [[0., 1.]; 2];
        let mut report = CurvatureReport {
            decision: CurvatureDecision::Unresolved,
            absolute_curvature_upper: None,
            cells: 0,
            covered_cells: 0,
            coefficient_work: 0,
            unresolved_regions: vec![full],
            exceeds_region: None,
            reason: "coefficient-budget",
        };
        let fields = match root(self, budget) {
            Ok(f) => f,
            Err(_) => {
                report.coefficient_work = budget.used() - start;
                return Ok(report);
            }
        };
        let mut stack = vec![(full, fields)];
        let mut upper = 0f64;
        while let Some((region, fields)) = stack.pop() {
            if report.cells >= max_cells {
                report.reason = "cell-budget";
                report.unresolved_regions = stack
                    .into_iter()
                    .map(|(r, _)| r)
                    .chain(std::iter::once(region))
                    .collect();
                report.coefficient_work = budget.used() - start;
                return Ok(report);
            }
            report.cells += 1;
            if let Ok(Some(range)) = bound(&fields) {
                if range[1] <= limit {
                    report.covered_cells += 1;
                    upper = upper.max(range[1]);
                    continue;
                }
                if range[0] > limit {
                    report.decision = CurvatureDecision::Exceeds;
                    report.reason = "whole-cell-exceeds-limit";
                    report.exceeds_region = Some(region);
                    report.unresolved_regions = stack.into_iter().map(|(r, _)| r).collect();
                    report.coefficient_work = budget.used() - start;
                    return Ok(report);
                }
            }
            let axis = usize::from(region[1][1] - region[1][0] > region[0][1] - region[0][0]);
            let mid = region[axis][0] * 0.5 + region[axis][1] * 0.5;
            if mid <= region[axis][0] || mid >= region[axis][1] {
                report.reason = "parameter-resolution";
                report.unresolved_regions = stack
                    .into_iter()
                    .map(|(r, _)| r)
                    .chain(std::iter::once(region))
                    .collect();
                report.coefficient_work = budget.used() - start;
                return Ok(report);
            }
            let children = (|| -> Result<(Vec<Net>, Vec<Net>)> {
                let mut left = Vec::new();
                let mut right = Vec::new();
                for n in &fields {
                    let (l, r) = split(n, axis, budget)?;
                    left.push(l);
                    right.push(r);
                }
                Ok((left, right))
            })();
            let (left, right) = match children {
                Ok(pair) => pair,
                Err(_) => {
                    report.reason = "coefficient-budget-or-arithmetic";
                    report.unresolved_regions = stack
                        .into_iter()
                        .map(|(r, _)| r)
                        .chain(std::iter::once(region))
                        .collect();
                    report.coefficient_work = budget.used() - start;
                    return Ok(report);
                }
            };
            let mut l = region;
            let mut r = region;
            l[axis][1] = mid;
            r[axis][0] = mid;
            stack.push((r, right));
            stack.push((l, left));
        }
        report.decision = CurvatureDecision::Within;
        report.absolute_curvature_upper = Some(upper);
        report.unresolved_regions.clear();
        report.reason = "complete-exact-coefficient-bernstein-cover";
        report.coefficient_work = budget.used() - start;
        Ok(report)
    }
}

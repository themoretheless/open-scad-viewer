//! Normal-curvature radius in the declared Sv direction of the exact transition.
//! This is not a moving-center/circular-section or full fillet certificate.
use super::*;
use crate::distance_bounds::Interval as I;
type Net = Vec<Vec<[Q; 3]>>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalRadiusDecision {
    Within,
    Exceeds,
    Unresolved,
}
pub struct NormalRadiusReport {
    pub decision: NormalRadiusDecision,
    pub domain: [[f64; 2]; 2],
    pub cells: usize,
    pub covered_cells: usize,
    pub coefficient_work: usize,
    pub witness_uv: Option<[[f64; 2]; 2]>,
    pub witness_radius_mm: Option<[f64; 2]>,
    pub unresolved_regions: Vec<[[f64; 2]; 2]>,
    pub reason: &'static str,
}
fn square(x: I) -> Result<I> {
    let low = if x.lo <= 0. && x.hi >= 0. {
        0.
    } else {
        (x.lo * x.lo).min(x.hi * x.hi).next_down().max(0.)
    };
    I::new(low, (x.lo * x.lo).max(x.hi * x.hi).next_up())
}
fn dot(a: [I; 3], z: [I; 3]) -> Result<I> {
    let mut out = I::point(0.);
    for k in 0..3 {
        out = out.add(a[k].mul(z[k])?)?;
    }
    Ok(out)
}
fn norm(a: [I; 3]) -> Result<I> {
    let mut out = I::point(0.);
    for x in a {
        out = out.add(square(x)?)?;
    }
    I::new(out.lo.max(0.), out.hi)
}
fn hull(n: &Net, b: &mut Budget) -> Result<[I; 3]> {
    let mut result = [I::point(0.); 3];
    for k in 0..3 {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for p in n.iter().flatten() {
            let x = curvature::enclose(&p[k], b)?;
            lo = lo.min(x.lo);
            hi = hi.max(x.hi);
        }
        result[k] = I::new(lo, hi)?;
    }
    Ok(result)
}
fn bound(jets: [[I; 3]; 3], low: I, high: I) -> Result<(NormalRadiusDecision, Option<[f64; 2]>)> {
    let [u, v, vv] = jets;
    let cross = std::array::from_fn::<_, 3, _>(|k| (k, (k + 1) % 3, (k + 2) % 3))
        .map(|(_, i, j)| u[i].mul(v[j]).and_then(|a| a.sub(u[j].mul(v[i])?)))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let cross: [I; 3] = cross.try_into().unwrap();
    let c = norm(cross)?;
    let e = norm(v)?;
    if c.lo <= 0. || e.lo <= 0. {
        return Ok((NormalRadiusDecision::Unresolved, None));
    }
    let t = square(dot(cross, vv)?)?;
    let d = c.mul(square(e)?)?;
    let above = square(high)?.mul(t)?.sub(d)?;
    let below = d.sub(square(low)?.mul(t)?)?;
    let radius = if t.lo > 0. {
        let r = d.div(t)?;
        Some([
            r.lo.max(0.).sqrt().next_down().max(0.),
            r.hi.sqrt().next_up(),
        ])
    } else {
        None
    };
    let decision = if above.hi < 0. || below.hi < 0. {
        NormalRadiusDecision::Exceeds
    } else if above.lo >= 0. && below.lo >= 0. {
        NormalRadiusDecision::Within
    } else {
        NormalRadiusDecision::Unresolved
    };
    Ok((decision, radius))
}
fn restrict_net(n: &Net, axis: usize, range: [Q; 2], b: &mut Budget) -> Result<Net> {
    let mut out = n.clone();
    for j in 0..if axis == 0 { n[0].len() } else { n.len() } {
        for k in 0..3 {
            let line = if axis == 0 {
                n.iter().map(|r| r[j][k].clone()).collect::<Vec<_>>()
            } else {
                n[j].iter().map(|p| p[k].clone()).collect()
            };
            let line = restrict(&line, &range[0], &range[1], b)?;
            for (i, x) in line.into_iter().enumerate() {
                if axis == 0 {
                    out[i][j][k] = x;
                } else {
                    out[j][i][k] = x;
                }
            }
        }
    }
    Ok(out)
}
fn at(n: &Net, parameter: &[Q; 2], b: &mut Budget) -> Result<[I; 3]> {
    let mut out = [I::point(0.); 3];
    for k in 0..3 {
        let values = n
            .iter()
            .map(|r| {
                split(
                    &r.iter().map(|p| p[k].clone()).collect::<Vec<_>>(),
                    &parameter[1],
                    b,
                )
                .map(|(a, _)| a.last().unwrap().clone())
            })
            .collect::<Result<Vec<_>>>()?;
        let value = split(&values, &parameter[0], b)?.0.last().unwrap().clone();
        out[k] = curvature::enclose(&value, b)?;
    }
    Ok(out)
}
fn midpoint(n: &Net, b: &mut Budget) -> Result<[I; 3]> {
    at(n, &[ratio(1, 2), ratio(1, 2)], b)
}
fn display(region: &[[Q; 2]; 2]) -> [[f64; 2]; 2] {
    fn enclosure(q: &Q) -> [f64; 2] {
        let x = q.to_f64().unwrap();
        let (mut lo, mut hi) = (x, x);
        for _ in 0..4 {
            if Q::from_float(lo).unwrap() > *q {
                lo = lo.next_down();
            }
            if Q::from_float(hi).unwrap() < *q {
                hi = hi.next_up();
            }
        }
        if Q::from_float(lo).unwrap() <= *q && *q <= Q::from_float(hi).unwrap() {
            [lo, hi]
        } else {
            [0., 1.]
        }
    }
    std::array::from_fn(|axis| {
        [
            enclosure(&region[axis][0])[0],
            enclosure(&region[axis][1])[1],
        ]
    })
}
impl Patch {
    /// Tests normal radius in the Sv tangent direction over an EXPLICIT
    /// normalized UV domain. G2 runouts are never silently removed. A witnessed
    /// violation rejects the whole requested domain; only full coverage passes.
    pub fn inspect_normal_radius(
        &self,
        radius_mm: f64,
        tolerance_mm: f64,
        domain: [[f64; 2]; 2],
        max_cells: usize,
        b: &mut Budget,
    ) -> Result<NormalRadiusReport> {
        check(
            radius_mm.is_finite()
                && tolerance_mm.is_finite()
                && tolerance_mm >= 0.
                && radius_mm > tolerance_mm
                && max_cells <= 1_000_000
                && domain.iter().all(|r| {
                    r[0].is_finite() && r[1].is_finite() && 0. <= r[0] && r[0] < r[1] && r[1] <= 1.
                }),
            "Invalid normal-radius law, domain or budget",
        )?;
        let start = b.used();
        let mut report = NormalRadiusReport {
            decision: NormalRadiusDecision::Unresolved,
            domain,
            cells: 0,
            covered_cells: 0,
            coefficient_work: 0,
            witness_uv: None,
            witness_radius_mm: None,
            unresolved_regions: vec![domain],
            reason: "exact-work-bit-or-numeric-limit",
        };
        if max_cells == 0 {
            report.reason = "cell-budget";
            return Ok(report);
        }
        let fast = (|| -> Result<bool> {
            let r = b.authored(radius_mm)?;
            let t = b.authored(tolerance_mm)?;
            let low = curvature::enclose(&b.keep(&r - &t)?, b)?;
            let high = curvature::enclose(&b.keep(r + t)?, b)?;
            self.correlated_normal_radius_within(domain, low, high, b)
        })();
        match fast {
            Ok(true) => {
                report.decision = NormalRadiusDecision::Within;
                report.reason = "whole-domain-correlated-normal-radius";
                report.cells = 1;
                report.covered_cells = 1;
                report.unresolved_regions.clear();
                report.coefficient_work = b.used() - start;
                return Ok(report);
            }
            Err(_) => {
                report.coefficient_work = b.used() - start;
                return Ok(report);
            }
            Ok(false) => (),
        }
        let setup = (|| -> Result<_> {
            let r = b.authored(radius_mm)?;
            let tolerance = b.authored(tolerance_mm)?;
            let low = curvature::enclose(&b.keep(&r - &tolerance)?, b)?;
            let high = curvature::enclose(&b.keep(r + tolerance)?, b)?;
            let u = curvature::derivative(&self.coefficients, 0, b)?;
            let v = curvature::derivative(&self.coefficients, 1, b)?;
            let vv = curvature::derivative(&v, 1, b)?;
            let mut jets = vec![u, v, vv];
            // A certified counterexample needs only exact jets at its point,
            // not an expensive prior restriction of every derivative net.
            let mut parameter = [Q::zero(), Q::zero()];
            for axis in 0..2 {
                parameter[axis] = b.keep(
                    (Q::from_float(domain[axis][0]).unwrap()
                        + Q::from_float(domain[axis][1]).unwrap())
                        / Q::from_integer(2.into()),
                )?;
            }
            let point = jets
                .iter()
                .map(|n| at(n, &parameter, b))
                .collect::<Result<Vec<_>>>()?;
            let witness = bound(point.try_into().unwrap(), low, high)?;
            if witness.0 == NormalRadiusDecision::Exceeds {
                return Ok((low, high, jets, Some((parameter, witness.1))));
            }
            for axis in 0..2 {
                if domain[axis] != [0., 1.] {
                    let range = [b.authored(domain[axis][0])?, b.authored(domain[axis][1])?];
                    for n in &mut jets {
                        *n = restrict_net(n, axis, range.clone(), b)?;
                    }
                }
            }
            Ok((low, high, jets, None))
        })();
        let (low, high, jets, witness) = match setup {
            Ok(x) => x,
            Err(_) => {
                report.coefficient_work = b.used() - start;
                return Ok(report);
            }
        };
        if let Some((parameter, radius)) = witness {
            report.decision = NormalRadiusDecision::Exceeds;
            report.reason = "certified-point-outside-normal-radius";
            report.cells = 1;
            report.witness_uv = Some(display(&parameter.map(|x| [x.clone(), x])));
            report.witness_radius_mm = radius;
            report.coefficient_work = b.used() - start;
            return Ok(report);
        }
        let exact_domain = domain.map(|r| r.map(|x| Q::from_float(x).unwrap()));
        let mut stack = vec![(exact_domain, jets)];
        while let Some((region, jets)) = stack.pop() {
            if report.cells >= max_cells {
                report.reason = "cell-budget";
                report.unresolved_regions = stack
                    .into_iter()
                    .map(|(r, _)| display(&r))
                    .chain([display(&region)])
                    .collect();
                report.coefficient_work = b.used() - start;
                return Ok(report);
            }
            report.cells += 1;
            let checked = (|| -> Result<_> {
                let point = jets
                    .iter()
                    .map(|n| midpoint(n, b))
                    .collect::<Result<Vec<_>>>()?;
                let witness = bound(point.try_into().unwrap(), low, high)?;
                if witness.0 == NormalRadiusDecision::Exceeds {
                    return Ok((witness, true));
                }
                let field = jets
                    .iter()
                    .map(|n| hull(n, b))
                    .collect::<Result<Vec<_>>>()?;
                Ok((bound(field.try_into().unwrap(), low, high)?, false))
            })();
            let ((decision, radius), point) = match checked {
                Ok(x) => x,
                Err(_) => {
                    report.unresolved_regions = stack
                        .into_iter()
                        .map(|(r, _)| display(&r))
                        .chain([display(&region)])
                        .collect();
                    report.coefficient_work = b.used() - start;
                    return Ok(report);
                }
            };
            if decision == NormalRadiusDecision::Exceeds {
                report.decision = decision;
                report.reason = if point {
                    "certified-point-outside-normal-radius"
                } else {
                    "whole-cell-outside-normal-radius"
                };
                report.witness_uv = Some(if point {
                    let point = region.clone().map(|r| {
                        let m = (&r[0] + &r[1]) / Q::from_integer(2.into());
                        [m.clone(), m]
                    });
                    display(&point)
                } else {
                    display(&region)
                });
                report.witness_radius_mm = radius;
                report.unresolved_regions = stack
                    .into_iter()
                    .map(|(r, _)| display(&r))
                    .chain(if point { Some(display(&region)) } else { None })
                    .collect();
                report.coefficient_work = b.used() - start;
                return Ok(report);
            }
            if decision == NormalRadiusDecision::Within {
                report.covered_cells += 1;
                continue;
            }
            let axis = usize::from(&region[1][1] - &region[1][0] > &region[0][1] - &region[0][0]);
            let children = (|| -> Result<_> {
                let mut left = Vec::new();
                let mut right = Vec::new();
                for n in &jets {
                    left.push(restrict_net(n, axis, [Q::zero(), ratio(1, 2)], b)?);
                    right.push(restrict_net(n, axis, [ratio(1, 2), Q::one()], b)?);
                }
                let mid =
                    b.keep((&region[axis][0] + &region[axis][1]) / Q::from_integer(2.into()))?;
                Ok((left, right, mid))
            })();
            let (left, right, mid) = match children {
                Ok(x) => x,
                Err(_) => {
                    report.unresolved_regions = stack
                        .into_iter()
                        .map(|(r, _)| display(&r))
                        .chain([display(&region)])
                        .collect();
                    report.coefficient_work = b.used() - start;
                    return Ok(report);
                }
            };
            let mut l = region.clone();
            let mut r = region;
            l[axis][1] = mid.clone();
            r[axis][0] = mid;
            stack.push((r, right));
            stack.push((l, left));
        }
        report.decision = NormalRadiusDecision::Within;
        report.reason = "complete-normal-radius-domain";
        report.unresolved_regions.clear();
        report.coefficient_work = b.used() - start;
        Ok(report)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn patch() -> Patch {
        let s: [Surface; 2] = std::array::from_fn(|side| Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..2)
                .map(|i| {
                    vec![
                        vec![i as f64, 0., 0.],
                        vec![i as f64, if side == 0 { -1. } else { 1. }, -0.25],
                    ]
                })
                .collect(),
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        });
        let w = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0.25, 0., 0.]; 2],
            weights: vec![1.; 2],
            periodic: false,
        };
        build([&s[0], &s[1]], &w, &mut Budget::new(1_000_000).unwrap()).unwrap()
    }
    #[test]
    fn analytic_quartic_midpoint_radius_rejects_r1_with_certified_witness() {
        // x=u, y=(2v-1)/4, z=-phi(v)/16. At v=1/2,
        // z_v=0, z_vv=-3/8, y_v=1/2, so normal radius=2/3 mm.
        let p = patch();
        let r = p
            .inspect_normal_radius(
                1.,
                0.01,
                [[0., 1.], [0.25, 0.75]],
                100,
                &mut Budget::new(1_000_000).unwrap(),
            )
            .unwrap();
        assert_eq!(r.decision, NormalRadiusDecision::Exceeds);
        let radius = r.witness_radius_mm.unwrap();
        assert!(radius[0] <= 2. / 3. && radius[1] >= 2. / 3. && radius[1] < 0.99);
        assert_eq!(r.witness_uv, Some([[0.5, 0.5]; 2]));
    }
    #[test]
    fn full_requested_band_is_required_and_runouts_are_not_silently_skipped() {
        let p = patch();
        let mut b = Budget::new(3_000_000).unwrap();
        let core = p
            .inspect_normal_radius(2. / 3., 0.01, [[0., 1.], [0.499, 0.501]], 200, &mut b)
            .unwrap();
        assert_eq!(core.decision, NormalRadiusDecision::Within);
        assert!(core.unresolved_regions.is_empty() && core.covered_cells > 0);
        let full = p
            .inspect_normal_radius(2. / 3., 0.01, [[0., 1.]; 2], 200, &mut b)
            .unwrap();
        assert_eq!(full.decision, NormalRadiusDecision::Exceeds);
    }
    #[test]
    fn radius_budget_exhaustion_never_looks_like_complete_coverage() {
        let p = patch();
        for (work, cells) in [(0, 100), (1_000_000, 0)] {
            let r = p
                .inspect_normal_radius(
                    1.,
                    0.01,
                    [[0., 1.]; 2],
                    cells,
                    &mut Budget::new(work).unwrap(),
                )
                .unwrap();
            assert_eq!(r.decision, NormalRadiusDecision::Unresolved);
            assert!(!r.unresolved_regions.is_empty());
        }
        assert!(
            p.inspect_normal_radius(0., 0.01, [[0., 1.]; 2], 1, &mut Budget::new(0).unwrap())
                .is_err()
        );
    }
    #[test]
    fn correlated_certificate_rechecks_actual_interior_coefficients() {
        let mut p = patch();
        let mut b = Budget::new(1_000_000).unwrap();
        p.coefficients[1][2][2] += ratio(1, 16);
        let r = p
            .inspect_normal_radius(2. / 3., 0.01, [[0., 1.], [0.499, 0.501]], 100, &mut b)
            .unwrap();
        assert_ne!(r.decision, NormalRadiusDecision::Within);
    }
    #[test]
    fn exact_witness_evaluation_preserves_both_asymmetric_parameter_clocks() {
        let net = (0..3)
            .map(|i| {
                (0..2)
                    .map(|j| {
                        [
                            ratio(i, 2),
                            ratio(j, 1),
                            ratio(if i == 2 { 1 } else { 0 }, 1) + ratio(3 * j, 1),
                        ]
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let point = at(
            &net,
            &[ratio(1, 4), ratio(3, 4)],
            &mut Budget::new(100_000).unwrap(),
        )
        .unwrap();
        for (k, value) in [0.25, 0.75, 37. / 16.].into_iter().enumerate() {
            assert!(point[k].lo <= value && value <= point[k].hi);
        }
    }
}

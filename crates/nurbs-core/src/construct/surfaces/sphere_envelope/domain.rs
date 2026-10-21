//! Continuous spanwise feasibility; no cross-knot smoothness or immersion claim.
use super::{V, validate};
use crate::{
    Result, check, curve::Curve, curve_jets, distance_bounds::Interval as I,
    numerics::interval_vec3 as iv,
};
#[derive(Clone, Debug)]
pub struct DomainReport {
    pub spanwise_characteristic_circles_proven: bool,
    pub angular_chart_proven: bool,
    /// All created cells, including replaced parents.
    pub cells: usize,
    pub unresolved: Vec<[f64; 2]>,
}
fn velocity(curve: &Curve, u: [f64; 2]) -> Result<[I; 3]> {
    let [a, b] = curve.domain();
    let width = I::point(b).sub(I::point(a))?;
    let source = I::point(a).add(width.mul(I::new(u[0], u[1])?)?)?;
    let mut result: Option<[I; 3]> = None;
    for span in curve.degree..curve.control_points.len() {
        let s = [curve.knots[span], curve.knots[span + 1]];
        if s[0] >= s[1] {
            continue;
        }
        let mut lo = source.lo.max(s[0]);
        let mut hi = source.hi.min(s[1]);
        if lo > hi {
            continue;
        }
        // Outward mapping can overlap a neighboring span by only a few ulps.
        // Enlarge this intersection (never shrink it) to keep derivative
        // extraction well conditioned; extra source coverage is conservative.
        let floor = (s[1] - s[0]) * 2_f64.powi(-20);
        if lo < hi && hi - lo < floor {
            lo = lo.min(hi - floor).max(s[0]);
            hi = hi.max(lo + floor).min(s[1]);
        }
        let (jets, local_width) = if lo < hi {
            (
                curve_jets::enclose(curve, span, [lo, hi])?,
                I::point(hi).sub(I::point(lo))?,
            )
        } else if lo == s[0] || lo == s[1] {
            // Include both one-sided endpoint jets without a zero-width cell.
            (
                curve_jets::endpoint(curve, span, s, lo == s[1])?,
                I::point(s[1]).sub(I::point(s[0]))?,
            )
        } else {
            (
                curve_jets::enclose(curve, span, s)?,
                I::point(s[1]).sub(I::point(s[0]))?,
            )
        };
        let factor = width.div(local_width)?;
        let v = [
            jets[1][0].mul(factor)?,
            jets[1][1].mul(factor)?,
            jets[1][2].mul(factor)?,
        ];
        result = Some(if let Some(old) = result {
            [
                I::new(old[0].lo.min(v[0].lo), old[0].hi.max(v[0].hi))?,
                I::new(old[1].lo.min(v[1].lo), old[1].hi.max(v[1].hi))?,
                I::new(old[2].lo.min(v[2].lo), old[2].hi.max(v[2].hi))?,
            ]
        } else {
            v
        });
    }
    result.ok_or_else(|| {
        crate::Error::new(
            crate::INVALID_INPUT,
            "Envelope feasibility has no source span",
        )
    })
}
/// Prove |dr/du| < |dC/du| and nonzero dC/du × normal over the entire normalized
/// domain using outward original-source jets. Positive radius follows from
/// positive controls and weights. Cross-knot derivative continuity is separate.
pub fn inspect_domain(
    path: &Curve,
    radius: &Curve,
    normal: V,
    max_cells: usize,
) -> Result<DomainReport> {
    validate(path, radius, normal)?;
    check(
        (1..=100000).contains(&max_cells),
        "Envelope feasibility needs 1..100000 cells",
    )?;
    let scale = normal.iter().map(|x| x.abs()).fold(0_f64, f64::max);
    let n = [
        I::point(normal[0]).div(I::point(scale))?,
        I::point(normal[1]).div(I::point(scale))?,
        I::point(normal[2]).div(I::point(scale))?,
    ];
    let mut pending = vec![[0., 1.]];
    let mut cells = 1;
    let mut unresolved = Vec::new();
    let mut circles = true;
    let mut chart = true;
    while let Some(u) = pending.pop() {
        let proof = (|| -> Result<(bool, bool)> {
            let v = velocity(path, u)?;
            let dr = velocity(radius, u)?[0];
            let speed = iv::norm(v)?;
            let radius_speed = dr.lo.abs().max(dr.hi.abs());
            Ok((speed.lo > radius_speed, iv::norm(iv::cross(v, n)?)?.lo > 0.))
        })();
        if matches!(proof, Ok((true, true))) {
            continue;
        }
        let mid = u[0] + (u[1] - u[0]) * 0.5;
        if cells + 2 <= max_cells && u[0] < mid && mid < u[1] {
            pending.push([mid, u[1]]);
            pending.push([u[0], mid]);
            cells += 2;
            continue;
        }
        let (a, b) = proof.unwrap_or((false, false));
        circles &= a;
        chart &= b;
        unresolved.push(u);
    }
    Ok(DomainReport {
        spanwise_characteristic_circles_proven: circles,
        angular_chart_proven: chart,
        cells,
        unresolved,
    })
}

#[derive(Clone, Debug)]
pub struct AuditedEnvelope {
    pub domain: DomainReport,
    /// Absent when a complete spanwise circle/frame cover was not proved.
    pub approximation: Option<super::Report>,
}
/// Keep missing feasibility distinct from sampled approximation acceptance.
/// A successful domain audit still does not certify cross-knot smoothness,
/// immersion, self-overlap, approximation error or solid topology.
pub fn approximate_with_domain_audit(
    path: &Curve,
    radius: &Curve,
    normal: V,
    limits: super::Limits,
    max_domain_cells: usize,
) -> Result<AuditedEnvelope> {
    super::validate_limits(limits)?;
    let domain = inspect_domain(path, radius, normal, max_domain_cells)?;
    let approximation =
        if domain.spanwise_characteristic_circles_proven && domain.angular_chart_proven {
            Some(super::approximate(path, radius, normal, limits)?)
        } else {
            None
        };
    Ok(AuditedEnvelope {
        domain,
        approximation,
    })
}

use super::*;
use crate::intersection::ss_intersection::fiber::{inverse, krawczyk};
pub(super) fn certify(
    equations: &[ExactSsPolynomial],
    d: [[f64; 2]; 4],
    axis: FiberAxis,
    limits: DeflatedSsFiberLimits,
) -> Result<DeflatedSsFiberReport> {
    validate_limits(limits.projection)?;
    check(
        (1..=1000000).contains(&limits.max_interval_operations)
            && limits
                .preconditioner
                .as_ref()
                .is_none_or(|r| r.iter().flatten().all(|x| x.is_finite())),
        "Invalid source-equivalent SS interval work or preconditioner",
    )?;
    check(
        equations.len() == 3,
        "A deflated square fiber requires three source-equivalent equations",
    )?;
    let free = axis.index();
    check(
        (0..4).all(|k| {
            d[k].iter().all(|x| x.is_finite())
                && 0. <= d[k][0]
                && d[k][1] <= 1.
                && if k == free {
                    d[k][0] <= d[k][1]
                } else {
                    d[k][0] < d[k][1]
                }
        }),
        "Deflated SS fiber leaves the original normalized knot cell",
    )?;
    let mut report = DeflatedSsFiberReport {
        status: FiberStatus::Unresolved,
        reason: "source-equivalent-operator-unproved",
        interval_operations: 0,
        projection_work: 0,
        root_image: None,
        contraction_upper: None,
        preconditioner: None,
        scales: None,
    };
    let mut iw = IntervalWork {
        used: 0,
        max: limits.max_interval_operations,
    };
    let mut algebra = Work {
        used: 0,
        limits: limits.projection,
    };
    let result = (|| -> Result<()> {
        let mut residual = [I::point(0.); 3];
        for k in 0..3 {
            residual[k] = interval::evaluate(&equations[k], d, &mut iw)?;
        }
        if residual.iter().any(|v| v.lo > 0. || v.hi < 0.) {
            report.status = FiberStatus::Excluded;
            report.reason = "source-equivalent-polynomial-separation";
            return Ok(());
        }
        let unknown: [usize; 3] = (0..4)
            .filter(|&k| k != free)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let mut center = d;
        let mut centers = [0.; 3];
        let mut scales = [0.; 3];
        for i in 0..3 {
            let k = unknown[i];
            let c = d[k][0] * 0.5 + d[k][1] * 0.5;
            if !(d[k][0] < c && c < d[k][1]) {
                report.reason = "source-equivalent-center-unrepresentable";
                return Ok(());
            }
            centers[i] = c;
            scales[i] = (c - d[k][0]).max(d[k][1] - c);
            center[k] = [c; 2];
        }
        let mut jacobian = [[I::point(0.); 3]; 3];
        for k in 0..3 {
            for i in 0..3 {
                let derivative = interval::derivative(&equations[k], unknown[i], &mut algebra)?;
                jacobian[k][i] = interval::evaluate(&derivative, d, &mut iw)?;
            }
        }
        let midpoint = jacobian.map(|row| row.map(|v| v.lo * 0.5 + v.hi * 0.5));
        let Some(r) = limits.preconditioner.or_else(|| inverse(midpoint)) else {
            report.reason = "source-equivalent-preconditioner-unavailable";
            return Ok(());
        };
        // Combine exact source polynomials BEFORE interval evaluation, retaining
        // correlation across the original equations under the fixed binary R.
        let projected = interval::project(equations, r, &mut algebra)?;
        let mut j = [[I::point(0.); 3]; 3];
        let mut f = [I::point(0.); 3];
        for k in 0..3 {
            f[k] = interval::centered(&projected[k], center, &mut iw, &mut algebra)?;
            for i in 0..3 {
                let derivative = interval::derivative(&projected[k], unknown[i], &mut algebra)?;
                j[k][i] = interval::evaluate(&derivative, d, &mut iw)?;
            }
        }
        let identity =
            std::array::from_fn(|i| std::array::from_fn(|j| if i == j { 1. } else { 0. }));
        // The fixed 3x3 map plus at most sixteen weighted-norm passes uses
        // fewer than 1024 interval arithmetic operations. Charge that upper
        // bound before entering the shared kernel; it cannot reset this ledger.
        if iw.max - iw.used < 1024 {
            return Err(crate::resource(
                "Source-equivalent SS map allowance exhausted",
            ));
        }
        iw.used += 1024;
        let map = krawczyk::map(d, unknown, centers, scales, identity, j, f)?;
        report.root_image = Some(map.image.map(|x| [x.lo, x.hi]));
        report.contraction_upper = Some(map.q);
        if (0..3).any(|i| map.image[i].hi < d[unknown[i]][0] || d[unknown[i]][1] < map.image[i].lo)
        {
            report.status = FiberStatus::Excluded;
            report.reason = "source-equivalent-krawczyk-exclusion";
            return Ok(());
        }
        if map.q >= 1. {
            report.reason = "source-equivalent-contraction-unproved";
            return Ok(());
        }
        if !(0..3).all(|i| d[unknown[i]][0] < map.image[i].lo && map.image[i].hi < d[unknown[i]][1])
        {
            report.reason = "source-equivalent-uniform-inclusion-unproved";
            return Ok(());
        }
        report.status = FiberStatus::UniformUniqueRoot;
        report.reason = "uniform-original-deflated-ss-fiber";
        report.preconditioner = Some(r);
        report.scales = Some(map.scales);
        Ok(())
    })();
    report.interval_operations = iw.used;
    report.projection_work = algebra.used;
    if result.is_err() {
        report.status = FiberStatus::Unresolved;
        report.reason = "source-equivalent-arithmetic-or-work-unproved";
        report.preconditioner = None;
        report.scales = None;
    }
    Ok(report)
}

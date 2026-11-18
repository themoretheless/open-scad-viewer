//! One common fixed-point map across the complete original periodic quilt.
use super::*;
pub(super) fn certify(
    source: &FiberSources,
    domain: [[f64; 2]; 4],
    axis: FiberAxis,
    limits: FiberLimits,
) -> Result<FiberReport> {
    check(
        (1..=100000).contains(&limits.max_rectangles)
            && limits
                .preconditioner
                .as_ref()
                .is_none_or(|r| r.iter().flatten().all(|x| x.is_finite())),
        "Quotient fibers need bounded work and finite preconditioning",
    )?;
    let free = axis.index();
    axes::validate(source, domain, free)?;
    let mut report = FiberReport {
        status: FiberStatus::Unresolved,
        reason: "quotient-source-unproved",
        rectangles: 0,
        residual_box: None,
        root_image: None,
        contraction_upper: None,
        operator: None,
        support_regularity_proven: [false; 2],
    };
    let mut work = Budget {
        max: limits.max_rectangles,
        used: 0,
    };
    let run = (|| -> Result<()> {
        let Some((a, ja)) = evaluate_lifted(source, 0, domain, true, &mut work)? else {
            report.reason = "quotient-rectangle-budget";
            return Ok(());
        };
        let Some((b, jb)) = evaluate_lifted(source, 1, domain, true, &mut work)? else {
            report.reason = "quotient-rectangle-budget";
            return Ok(());
        };
        let residual = [a[0].sub(b[0])?, a[1].sub(b[1])?, a[2].sub(b[2])?];
        report.residual_box = Some(residual.map(pair));
        if residual.iter().any(|x| x.lo > 0. || x.hi < 0.) {
            report.status = FiberStatus::Excluded;
            report.reason = "original-quotient-box-separation";
            return Ok(());
        }
        report.support_regularity_proven = [regular(ja), regular(jb)];
        let unknown: [usize; 3] = (0..4)
            .filter(|&k| k != free)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let mut centers = [0.; 3];
        let mut scales = [0.; 3];
        let mut center = domain;
        for i in 0..3 {
            let k = unknown[i];
            let c = domain[k][0] * 0.5 + domain[k][1] * 0.5;
            if !(domain[k][0] < c && c < domain[k][1]) {
                report.reason = "quotient-center-unrepresentable";
                return Ok(());
            }
            centers[i] = c;
            scales[i] = (c - domain[k][0]).max(domain[k][1] - c);
            center[k] = [c; 2];
        }
        let mut j = [[I::point(0.); 3]; 3];
        for k in 0..3 {
            for i in 0..3 {
                let a = unknown[i];
                j[k][i] = if a < 2 {
                    ja[k][a]
                } else {
                    I::point(0.).sub(jb[k][a - 2])?
                };
            }
        }
        let mid = j.map(|row| row.map(|x| x.lo * 0.5 + x.hi * 0.5));
        let Some(r) = limits.preconditioner.or_else(|| inverse(mid)) else {
            report.reason = "quotient-preconditioner-unavailable";
            return Ok(());
        };
        let Some((a, _)) = evaluate_lifted(source, 0, center, false, &mut work)? else {
            report.reason = "quotient-rectangle-budget";
            return Ok(());
        };
        let Some((b, _)) = evaluate_lifted(source, 1, center, false, &mut work)? else {
            report.reason = "quotient-rectangle-budget";
            return Ok(());
        };
        let f = [a[0].sub(b[0])?, a[1].sub(b[1])?, a[2].sub(b[2])?];
        let mut map = super::super::krawczyk::map(domain, unknown, centers, scales, r, j, f)?;
        let included = |image: &[I; 3]| {
            (0..3)
                .all(|i| domain[unknown[i]][0] < image[i].lo && image[i].hi < domain[unknown[i]][1])
        };
        if (map.q >= 1. || !included(&map.image)) && work.used < work.max {
            let origin = &source.first().control_points[0][0];
            let Some((_, pa)) =
                evaluate_lifted_projected(source, 0, domain, true, r, origin, &mut work)?
            else {
                report.reason = "quotient-rectangle-budget";
                return Ok(());
            };
            let Some((_, pb)) =
                evaluate_lifted_projected(source, 1, domain, true, r, origin, &mut work)?
            else {
                report.reason = "quotient-rectangle-budget";
                return Ok(());
            };
            let Some((a, _)) =
                evaluate_lifted_projected(source, 0, center, false, r, origin, &mut work)?
            else {
                report.reason = "quotient-rectangle-budget";
                return Ok(());
            };
            let Some((b, _)) =
                evaluate_lifted_projected(source, 1, center, false, r, origin, &mut work)?
            else {
                report.reason = "quotient-rectangle-budget";
                return Ok(());
            };
            let f = [a[0].sub(b[0])?, a[1].sub(b[1])?, a[2].sub(b[2])?];
            let mut projected = [[I::point(0.); 3]; 3];
            for k in 0..3 {
                for i in 0..3 {
                    let a = unknown[i];
                    projected[k][i] = if a < 2 {
                        pa[k][a]
                    } else {
                        I::point(0.).sub(pb[k][a - 2])?
                    };
                }
            }
            let identity =
                std::array::from_fn(|i| std::array::from_fn(|j| if i == j { 1. } else { 0. }));
            let other = super::super::krawczyk::map(
                domain, unknown, centers, scales, identity, projected, f,
            )?;
            let intersection = [0, 1, 2].map(|i| {
                [
                    map.image[i].lo.max(other.image[i].lo),
                    map.image[i].hi.min(other.image[i].hi),
                ]
            });
            if intersection.iter().all(|d| d[0] <= d[1]) {
                for i in 0..3 {
                    map.image[i] = I::new(intersection[i][0], intersection[i][1])?;
                }
            }
            if other.q < map.q {
                map.q = other.q;
                map.scales = other.scales;
            }
        }
        report.root_image = Some(map.image.map(pair));
        report.contraction_upper = Some(map.q);
        if (0..3).any(|i| {
            map.image[i].hi < domain[unknown[i]][0] || domain[unknown[i]][1] < map.image[i].lo
        }) {
            report.status = FiberStatus::Excluded;
            report.reason = "original-quotient-krawczyk-exclusion";
            return Ok(());
        }
        if map.q >= 1. {
            report.reason = "quotient-contraction-unproved";
            return Ok(());
        }
        if !(0..3).all(|i| {
            domain[unknown[i]][0] < map.image[i].lo && map.image[i].hi < domain[unknown[i]][1]
        }) {
            report.reason = "quotient-uniform-inclusion-unproved";
            return Ok(());
        }
        report.operator = Some(FiberOperator {
            preconditioner: r,
            scales: map.scales,
            first_jacobian: ja.map(|row| row.map(pair)),
            exact_graph_shear: false,
        });
        report.status = FiberStatus::UniformUniqueRoot;
        report.reason = "uniform-original-quotient-krawczyk-fiber";
        Ok(())
    })();
    report.rectangles = work.used;
    if run.is_err() {
        report.status = FiberStatus::Unresolved;
        report.reason = "quotient-original-operator-unavailable";
        report.operator = None;
    }
    Ok(report)
}

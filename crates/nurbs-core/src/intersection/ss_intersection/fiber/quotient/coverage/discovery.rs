use super::*;
fn hull(
    source: &FiberSources,
    domain: [[f64; 2]; 4],
    work: &mut Budget,
) -> Result<Option<FiberReport>> {
    // Charts already establish source-wide C0 continuity, original endpoint
    // semantics and exact periodic seam identity. Enclose each closed original
    // cell, including the one-sided seam limit which equals the actual alias.
    let start = work.used;
    let Some((a, _)) = evaluate(source.first(), [domain[0], domain[1]], false, work)? else {
        return Ok(None);
    };
    let Some((b, _)) = evaluate(source.second(), [domain[2], domain[3]], false, work)? else {
        return Ok(None);
    };
    let residual = [a[0].sub(b[0])?, a[1].sub(b[1])?, a[2].sub(b[2])?];
    let excluded = residual.iter().any(|v| v.lo > 0. || v.hi < 0.);
    Ok(Some(FiberReport {
        status: if excluded {
            FiberStatus::Excluded
        } else {
            FiberStatus::Unresolved
        },
        reason: if excluded {
            "original-quotient-cell-separation"
        } else {
            "original-quotient-cell-not-separated"
        },
        rectangles: work.used - start,
        residual_box: Some(residual.map(pair)),
        root_image: None,
        contraction_upper: None,
        operator: None,
        support_regularity_proven: [false; 2],
    }))
}
fn first_chart(fibers: &[QualifiedQuotientFiber], domain: [[f64; 2]; 4]) -> Result<Option<usize>> {
    for (i, f) in fibers.iter().enumerate() {
        if f.contains_original_box(domain)? {
            return Ok(Some(i));
        }
    }
    Ok(None)
}
fn root_chart(
    fibers: &[QualifiedQuotientFiber],
    domain: [[f64; 2]; 4],
    work: &mut Budget,
) -> Result<Option<QuotientCoverageKind>> {
    let source = fibers[0].source();
    let n = natural(source);
    // A complete periodic interval is not one injective lifted chart. The
    // forest first subdivides it; no chart scope is enlarged to admit it.
    for k in 0..4 {
        let s = if k < 2 {
            source.first()
        } else {
            source.second()
        };
        let periodic = if k % 2 == 0 {
            s.periodic_u
        } else {
            s.periodic_v
        };
        if periodic && domain[k] == n[k] {
            return Ok(None);
        }
    }
    for axis in [
        FiberAxis::FirstU,
        FiberAxis::FirstV,
        FiberAxis::SecondU,
        FiberAxis::SecondV,
    ] {
        if work.used == work.max {
            return Ok(None);
        }
        let r = source.certify_quotient(
            domain,
            axis,
            FiberLimits {
                max_rectangles: (work.max - work.used).min(100000),
                preconditioner: None,
            },
        )?;
        let report = r.report();
        work.used += report.rectangles;
        if report.status == FiberStatus::Excluded {
            return Ok(Some(QuotientCoverageKind::Excluded(report.clone())));
        }
        if let Some(image) = report.root_image {
            let mut root = domain;
            let mut i = 0;
            for k in 0..4 {
                if k != axis.index() {
                    root[k] = [domain[k][0].max(image[i][0]), domain[k][1].min(image[i][1])];
                    i += 1;
                }
            }
            if root.iter().any(|d| d[0] > d[1]) {
                return Err(crate::input(
                    "Inconsistent original quotient exclusion image",
                ));
            }
            if let Some(chart) = first_chart(fibers, root)? {
                return Ok(Some(QuotientCoverageKind::RootImage {
                    chart,
                    axis,
                    report: report.clone(),
                }));
            }
        }
    }
    Ok(None)
}
pub(super) fn construct(
    fibers: &[QualifiedQuotientFiber],
    domain: [[f64; 2]; 4],
    limits: CoverageLimits,
) -> Result<QuotientCoverageResult> {
    let source = fibers[0].source();
    let mut work = Budget {
        used: 0,
        max: limits.max_rectangles,
    };
    let mut report = FiberCoverageReport {
        status: FiberCoverageStatus::Unresolved,
        reason: "quotient-chart-replay-unproved",
        cells: 0,
        excluded_cells: 0,
        rectangles: 0,
        requested_entire_surfaces: domain == natural(source),
        unresolved_box: Some(domain),
    };
    for f in fibers {
        if work.used == work.max {
            report.rectangles = work.used;
            return Ok(QuotientCoverageResult {
                report,
                coverage: None,
            });
        }
        let r = source.certify_quotient(
            f.domain(),
            f.free_axis(),
            FiberLimits {
                max_rectangles: (work.max - work.used).min(100000),
                preconditioner: f.limits.preconditioner,
            },
        )?;
        work.used += r.report().rectangles;
        if r.report() != f.report() {
            report.rectangles = work.used;
            return Ok(QuotientCoverageResult {
                report,
                coverage: None,
            });
        }
    }
    let mut nodes = vec![QuotientCoverageNode { domain, kind: None }];
    let mut stack = vec![(0, 0)];
    let natural = natural(source);
    while let Some((id, depth)) = stack.pop() {
        let d = nodes[id].domain;
        report.unresolved_box = Some(d);
        report.reason = "quotient-original-coverage-unproved";
        if let Some(chart) = first_chart(fibers, d)? {
            nodes[id].kind = Some(QuotientCoverageKind::Chart(chart));
            continue;
        }
        let Some(attempt) = hull(source, d, &mut work)? else {
            report.reason = "quotient-rectangle-budget";
            break;
        };
        if attempt.status == FiberStatus::Excluded {
            report.excluded_cells += 1;
            nodes[id].kind = Some(QuotientCoverageKind::Excluded(attempt));
            continue;
        }
        if let Some(kind) = root_chart(fibers, d, &mut work)? {
            if matches!(kind, QuotientCoverageKind::Excluded(_)) {
                report.excluded_cells += 1;
            }
            nodes[id].kind = Some(kind);
            continue;
        }
        if work.used == work.max {
            report.reason = "quotient-rectangle-budget";
            break;
        }
        if depth == limits.max_depth || nodes.len() + 2 > limits.max_cells {
            report.reason = "quotient-cell-or-depth-limit";
            break;
        }
        let axis = (0..4)
            .max_by(|&a, &b| {
                ((d[a][1] - d[a][0]) / (natural[a][1] - natural[a][0]))
                    .total_cmp(&((d[b][1] - d[b][0]) / (natural[b][1] - natural[b][0])))
            })
            .unwrap();
        let parameter = d[axis][0] * 0.5 + d[axis][1] * 0.5;
        if parameter <= d[axis][0] || parameter >= d[axis][1] {
            report.reason = "quotient-subdivision-unrepresentable";
            break;
        }
        let mut a = d;
        let mut b = d;
        a[axis][1] = parameter;
        b[axis][0] = parameter;
        let children = [nodes.len(), nodes.len() + 1];
        nodes.push(QuotientCoverageNode {
            domain: a,
            kind: None,
        });
        nodes.push(QuotientCoverageNode {
            domain: b,
            kind: None,
        });
        nodes[id].kind = Some(QuotientCoverageKind::Split {
            axis,
            parameter,
            children,
        });
        stack.push((children[1], depth + 1));
        stack.push((children[0], depth + 1));
    }
    report.cells = nodes.len();
    report.rectangles = work.used;
    if nodes.iter().any(|n| n.kind.is_none()) {
        return Ok(QuotientCoverageResult {
            report,
            coverage: None,
        });
    }
    report.status = FiberCoverageStatus::Complete;
    report.reason = "complete-original-quotient-root-coverage";
    report.unresolved_box = None;
    let coverage = QualifiedQuotientCoverage {
        fibers: fibers.to_vec(),
        domain,
        nodes,
        limits,
        report: report.clone(),
    };
    Ok(QuotientCoverageResult {
        report,
        coverage: Some(coverage),
    })
}

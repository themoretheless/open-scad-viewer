use super::*;
#[test]
fn continuation_rejects_foreign_owners_and_insufficient_cumulative_limits() {
    let faces = [ShellFace {
        kind: ShellFaceKind::Original(0),
        loops: vec![],
    }];
    let foreign = [ShellFace {
        kind: ShellFaceKind::Original(0),
        loops: vec![],
    }];
    let audit = ShellContactAudit {
        owners: &faces,
        pairs: vec![],
        cells: 3,
        spans: 5,
        domain_cells: 7,
        exact_work: 11,
        driver_cells: 13,
        linear_cells: 17,
    };
    let limits = ContactAuditLimits {
        cells: 3,
        cells_per_pair: 0,
        controls_per_enclosure: 0,
        spans: 5,
        domain_cells: 7,
        domain_cells_per_pair: 0,
        exact_work: 11,
        driver_cells: 13,
        tolerance_uv: 1e-8,
    };
    let before = audit.diagnostics();
    assert!(validate_resume(&faces, &audit, limits, 0).is_ok());
    assert!(validate_resume(&foreign, &audit, limits, 0).is_err());
    for smaller in [
        ContactAuditLimits { cells: 2, ..limits },
        ContactAuditLimits { spans: 4, ..limits },
        ContactAuditLimits {
            domain_cells: 6,
            ..limits
        },
        ContactAuditLimits {
            exact_work: 10,
            ..limits
        },
        ContactAuditLimits {
            driver_cells: 12,
            ..limits
        },
    ] {
        assert!(validate_resume(&faces, &audit, smaller, 0).is_err());
    }
    for invalid in [
        ContactAuditLimits {
            cells: 100001,
            ..limits
        },
        ContactAuditLimits {
            controls_per_enclosure: 1000001,
            ..limits
        },
        ContactAuditLimits {
            tolerance_uv: f64::NAN,
            ..limits
        },
        ContactAuditLimits {
            tolerance_uv: 0.,
            ..limits
        },
    ] {
        assert!(validate_resume(&faces, &audit, invalid, 0).is_err());
    }
    assert!(validate_resume(&faces, &audit, limits, 100001).is_err());
    assert_eq!(audit.diagnostics(), before);
}
fn volume(p: SupportProduct) -> f64 {
    p.iter().flatten().map(|r| r[1] - r[0]).product()
}
#[test]
fn resumed_support_queries_keep_a_real_plane_intersection() {
    let inspect = |domain, cells, spans, domains| {
        super::super::super::contacts::inspect_regions(
            domain,
            cells,
            spans,
            domains,
            |side, r, budget| {
                if budget == 0 {
                    return Ok((None, 0));
                }
                Ok((
                    Some(if side == 0 {
                        [r[0], r[1], [0.; 2]]
                    } else {
                        [r[0], [0.5; 2], r[1]]
                    }),
                    1,
                ))
            },
            |_, _, _| Ok((false, 0)),
        )
    };
    let mut report = inspect([[[0., 1.]; 2]; 2], 7, 32, 0).unwrap();
    let witness = [[0.25, 0.5], [0.25, 0.]];
    for _ in 0..4 {
        assert!(!report.complete_excluded());
        assert!(report.pending.iter().any(|p| (0..2).all(|side| {
            (0..2).all(|axis| {
                p[side][axis][0] <= witness[side][axis]
                    && witness[side][axis] <= p[side][axis][1]
            })
        })));
        assert_eq!(
            report
                .pending
                .iter()
                .chain(&report.separated)
                .chain(&report.outside)
                .copied()
                .map(volume)
                .sum::<f64>(),
            1.
        );
        refine(&mut report, 64, 256, 0, inspect).unwrap();
    }
    assert!(!report.complete_excluded());
}
#[test]
fn pending_refinement_preserves_cover_and_failed_work_is_transactional() {
    let whole = [[[0., 1.]; 2]; 2];
    let mut left = whole;
    left[0][0][1] = 0.5;
    let mut right = whole;
    right[0][0][0] = 0.5;
    let mut report = SupportPairReport {
        separated: vec![left],
        outside: vec![],
        pending: vec![right],
        cells: 1,
        spans: 1,
        domain_cells: 0,
    };
    refine(&mut report, 0, 1, 1, |_, _, _, _| {
        panic!("no budget must not inspect")
    })
    .unwrap();
    assert_eq!(report.pending, vec![right]);
    let failed = refine(&mut report, 2, 2, 2, |_, _, _, _| {
        Err(Error::new("TEST", "cancelled"))
    });
    assert!(failed.is_err());
    assert_eq!(report.pending, vec![right]);
    assert_eq!(report.cells, 1);
    let failed = refine(&mut report, 1, 1, 1, |p, _, _, _| {
        Ok(SupportPairReport {
            separated: vec![p],
            outside: vec![],
            pending: vec![],
            cells: 2,
            spans: 1,
            domain_cells: 0,
        })
    });
    assert!(failed.is_err());
    assert_eq!(report.pending, vec![right]);
    assert_eq!(report.separated, vec![left]);
    assert_eq!((report.cells, report.spans), (1, 1));
    refine(&mut report, 3, 3, 3, |p, c, s, d| {
        assert_eq!(p, right);
        assert_eq!((c, s, d), (3, 3, 3));
        let mut a = p;
        let mut b = p;
        a[1][1][1] = 0.5;
        b[1][1][0] = 0.5;
        Ok(SupportPairReport {
            separated: vec![a],
            outside: vec![],
            pending: vec![b],
            cells: 1,
            spans: 1,
            domain_cells: 0,
        })
    })
    .unwrap();
    assert_eq!(report.cells, 2);
    assert_eq!(report.spans, 2);
    assert_eq!(
        report
            .pending
            .iter()
            .chain(&report.separated)
            .copied()
            .map(volume)
            .sum::<f64>(),
        1.
    );
    assert!(!report.complete_excluded());
    refine(&mut report, 1, 1, 1, |p, _, _, _| {
        Ok(SupportPairReport {
            separated: vec![p],
            outside: vec![],
            pending: vec![],
            cells: 1,
            spans: 1,
            domain_cells: 0,
        })
    })
    .unwrap();
    assert!(report.complete_excluded());
    assert_eq!(
        report.separated.iter().copied().map(volume).sum::<f64>(),
        1.
    );
}

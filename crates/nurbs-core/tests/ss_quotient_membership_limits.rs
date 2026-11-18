use nurbs_core::ss_intersection as ss;
#[allow(dead_code)]
#[path = "support/ss_closed_fiber.rs"]
mod fixture;
#[test]
fn binary_order_membership_preserves_exact_chart_shores() {
    let source = fixture::circle();
    let native = fixture::circle_family(&source).remove(0);
    let fiber = source
        .certify_quotient(
            native.domain(),
            native.free_axis(),
            ss::FiberLimits {
                preconditioner: native
                    .report()
                    .operator
                    .as_ref()
                    .map(|o| o.preconditioner()),
                ..Default::default()
            },
        )
        .unwrap()
        .into_fiber()
        .unwrap();
    let d = fiber.domain();
    assert!(fiber.contains_original_box(d).unwrap());
    for k in 0..4 {
        let mut before = d;
        before[k][0] = before[k][0].next_down();
        assert!(!fiber.contains_original_box(before).unwrap());
        let mut after = d;
        after[k][1] = after[k][1].next_up();
        assert!(!fiber.contains_original_box(after).unwrap());
    }
    let mut invalid = d;
    invalid[0][0] = f64::NAN;
    assert!(fiber.contains_original_box(invalid).is_err());
    let limited = ss::cover_quotient_fiber_surfaces(
        &[fiber.clone()],
        ss::CoverageLimits {
            max_cells: 65536,
            max_depth: 64,
            max_rectangles: 1,
        },
    )
    .unwrap();
    assert!(limited.coverage().is_none());
    assert!(limited.report().rectangles <= 1);
    for limits in [
        ss::CoverageLimits {
            max_depth: 65,
            ..Default::default()
        },
        ss::CoverageLimits {
            max_rectangles: 1000001,
            ..Default::default()
        },
    ] {
        assert!(ss::cover_quotient_fiber_surfaces(&[fiber.clone()], limits).is_err());
        assert!(
            ss::discover_quotient_fiber_surfaces(
                &source,
                ss::QuotientDiscoveryLimits {
                    coverage: limits,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}

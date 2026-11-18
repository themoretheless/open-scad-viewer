use nurbs_core::tessellation::lod::{self, Level, Reason, Request};

fn levels() -> Vec<Level> {
    vec![
        Level {
            triangle_count: 100,
            error_upper: 20.,
        },
        Level {
            triangle_count: 200,
            error_upper: 10.,
        },
        Level {
            triangle_count: 400,
            error_upper: 5.,
        },
    ]
}

fn request(scale: f64) -> Request {
    Request {
        pixels_per_unit: scale,
        pixel_budget: 10.,
        triangle_budget: 1000,
        hysteresis: 1.5,
    }
}

#[test]
fn selects_existing_levels_with_asymmetric_hysteresis() {
    let levels = levels();
    let initial = lod::select(&levels, None, request(1.)).unwrap();
    assert_eq!(initial.level, 2);
    assert_eq!(initial.reason, Reason::Initial);
    assert!(initial.within_pixel_budget);

    let held = lod::select(&levels, Some(1), request(1.2)).unwrap();
    assert_eq!(held.level, 1);
    assert_eq!(held.reason, Reason::HysteresisHold);
    assert!(!held.within_pixel_budget);

    let upgraded = lod::select(&levels, Some(1), request(1.6)).unwrap();
    assert_eq!(upgraded.level, 2);
    assert_eq!(upgraded.reason, Reason::PixelBudget);

    let held_fine = lod::select(&levels, Some(2), request(1.4)).unwrap();
    assert_eq!(held_fine.level, 2);
    assert_eq!(held_fine.reason, Reason::HysteresisHold);

    let downgraded = lod::select(&levels, Some(2), request(0.8)).unwrap();
    assert_eq!(downgraded.level, 1);
    assert_eq!(downgraded.reason, Reason::PixelBudget);
}

#[test]
fn triangle_budget_bypasses_hysteresis_and_never_synthesizes_lod() {
    let levels = levels();
    let selected = lod::select(
        &levels,
        Some(2),
        Request {
            triangle_budget: 150,
            ..request(100.)
        },
    )
    .unwrap();
    assert_eq!(selected.level, 0);
    assert_eq!(selected.reason, Reason::TriangleBudget);
    assert!(!selected.within_pixel_budget);

    let error = lod::select(
        &levels,
        None,
        Request {
            triangle_budget: 99,
            ..request(1.)
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "NURBS_LOD_TRIANGLE_BUDGET");
}

#[test]
fn invalid_or_unordered_levels_are_refused() {
    assert!(lod::select(&levels()[..1], None, request(1.)).is_err());
    let mut reversed = levels();
    reversed.swap(0, 2);
    assert!(lod::select(&reversed, None, request(1.)).is_err());
    let mut non_finite = levels();
    non_finite[1].error_upper = f64::NAN;
    assert!(lod::select(&non_finite, None, request(1.)).is_err());
    assert!(lod::select(
        &levels(),
        None,
        Request {
            hysteresis: 0.9,
            ..request(1.)
        },
    )
    .is_err());
}

#[test]
fn downgrade_never_leaves_the_pixel_budget_and_settles() {
    // Projected errors at scale 2: 40, 20, 2 px against a 10 px budget. The
    // fine level is far below budget / hysteresis, but the next coarser level
    // would exceed the budget, so the selector must hold instead of cycling.
    let levels = [
        Level {
            triangle_count: 100,
            error_upper: 20.,
        },
        Level {
            triangle_count: 200,
            error_upper: 10.,
        },
        Level {
            triangle_count: 400,
            error_upper: 1.,
        },
    ];
    let mut current = Some(0);
    let mut seen = Vec::new();
    for _ in 0..6 {
        let selection = lod::select(&levels, current, request(2.)).unwrap();
        seen.push(selection.level);
        current = Some(selection.level);
    }
    assert_eq!(seen, vec![2, 2, 2, 2, 2, 2]);
}

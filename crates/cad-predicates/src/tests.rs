use super::*;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

fn arena(values: &[f64]) -> SourceArena {
    SourceArena::authored(
        "authored-test-source",
        1,
        values
            .iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .unwrap()
}
fn pair(arena: &SourceArena, offset: usize) -> [LeafRef; 2] {
    [arena.leaf(offset).unwrap(), arena.leaf(offset + 1).unwrap()]
}
fn decision(source: &SourceArena, limits: Limits) -> Decision {
    let tolerance = ToleranceContext::default_valid();
    let mut context = PredicateContext::new(source, &tolerance, limits, None);
    orient2d(
        &mut context,
        pair(source, 0),
        pair(source, 2),
        pair(source, 4),
    )
    .unwrap()
}

#[test]
fn ordinary_sign_zero_and_handedness_use_authored_bits() {
    let source = arena(&[0., 0., 1., 0., 0., 1.]);
    let answer = decision(&source, Limits::default());
    assert_eq!(answer.outcome, Outcome::Sign(Sign::Positive));
    assert_eq!(answer.stage, Stage::Binary64);
    let source = arena(&[0., 0., 0., 1., 1., 0.]);
    assert_eq!(
        decision(&source, Limits::default()).outcome,
        Outcome::Sign(Sign::Negative)
    );
    let source = arena(&[0., -0., 1., 1., 2., 2.]);
    let answer = decision(&source, Limits::default());
    assert_eq!(answer.outcome, Outcome::Sign(Sign::Zero));
    assert_eq!(answer.stage, Stage::ExactExpansion);
}

#[test]
fn cancelling_products_keep_the_roundoff_component() {
    // (1+2^-52)(1-2^-52)-1 is exactly -2^-104.
    let source = arena(&[0., 0., 1. + f64::EPSILON, 1., 1., 1. - f64::EPSILON]);
    let answer = decision(&source, Limits::default());
    assert_eq!(answer.stage, Stage::ExactExpansion);
    assert_eq!(answer.outcome, Outcome::Sign(Sign::Negative));
    let source = arena(&[
        0.,
        0.,
        0.,
        1.,
        0.,
        0.,
        0.,
        1. + f64::EPSILON,
        1.,
        0.,
        1.,
        1. - f64::EPSILON,
    ]);
    let tolerance = ToleranceContext::default_valid();
    let mut context = PredicateContext::new(&source, &tolerance, Limits::default(), None);
    let p = |start| std::array::from_fn(|i| source.leaf(start + i).unwrap());
    assert_eq!(
        orient3d(&mut context, p(0), p(3), p(6), p(9))
            .unwrap()
            .outcome,
        Outcome::Sign(Sign::Negative)
    );
}

#[test]
fn rational_denominators_do_not_become_rounded_input_leaves() {
    let source = SourceArena::authored(
        "rational",
        0,
        vec![
            AuthoredScalar::Binary64Bits(0),
            AuthoredScalar::Binary64Bits(0),
            AuthoredScalar::Binary64Bits((1. / 3_f64).to_bits()),
            AuthoredScalar::Binary64Bits(0),
            AuthoredScalar::RationalConstant {
                numerator: 1,
                denominator: 3,
            },
        ],
    )
    .unwrap();
    let tolerance = ToleranceContext::default_valid();
    let mut context = PredicateContext::new(&source, &tolerance, Limits::default(), None);
    let answer = compare_squared_distance(
        &mut context,
        &pair(&source, 0),
        &pair(&source, 2),
        source.leaf(4).unwrap(),
    )
    .unwrap();
    assert_eq!(answer.outcome, Outcome::Sign(Sign::Negative));
    assert_eq!(answer.stage, Stage::ExactExpansion);
}

#[test]
fn uniform_extreme_exponents_can_normalize_but_unsafe_products_refuse() {
    for scale in [
        f64::from_bits(1),
        2_f64.powi(-600),
        2_f64.powi(600),
        f64::MAX,
    ] {
        let source = arena(&[0., 0., scale, 0., 0., scale]);
        assert_eq!(
            decision(&source, Limits::default()).outcome,
            Outcome::Sign(Sign::Positive)
        );
    }
    let source = arena(&[0., 0., f64::from_bits(1), 1., 0., f64::from_bits(1)]);
    assert_eq!(
        decision(&source, Limits::default()).outcome,
        Outcome::Indeterminate(Reason::PrecisionExhausted)
    );
}

#[test]
fn admission_preserves_source_and_context_identity() {
    for bits in [f64::NAN.to_bits(), f64::INFINITY.to_bits()] {
        assert!(
            SourceArena::authored("invalid", 0, vec![AuthoredScalar::Binary64Bits(bits)]).is_err()
        );
    }
    for (numerator, denominator) in [(1, 0), (2, 4), (0, 2)] {
        assert!(
            SourceArena::authored(
                "invalid",
                0,
                vec![AuthoredScalar::RationalConstant {
                    numerator,
                    denominator
                }]
            )
            .is_err()
        );
    }
    let a = arena(&[0., 0., 1., 0., 0., 1.]);
    let b = arena(&[0., 0., 1., 0., 0., 1.]);
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&a, &tolerance, Limits::default(), None);
    assert_eq!(
        orient2d(&mut ctx, pair(&b, 0), pair(&a, 2), pair(&a, 4)),
        Err(InputError::ProvenanceMismatch)
    );
    let other_tolerance = ToleranceContext::new(tolerance.specification().clone()).unwrap();
    assert_ne!(
        ctx.identity(),
        PredicateContext::new(&a, &other_tolerance, Limits::default(), None).identity()
    );
    assert_ne!(
        ctx.identity(),
        PredicateContext::new(&b, &tolerance, Limits::default(), None).identity()
    );
    assert_eq!(a.source_id(), "authored-test-source");
    assert_eq!(a.revision(), 1);
}

#[test]
fn work_cancellation_deadline_and_expansion_capacity_fail_explicitly() {
    let source = arena(&[0., 0., 1. + f64::EPSILON, 1., 1., 1. - f64::EPSILON]);
    assert_eq!(
        decision(
            &source,
            Limits {
                max_work: MAX_WORK + 1,
                ..Limits::default()
            }
        )
        .outcome,
        Outcome::Indeterminate(Reason::ResourceLimit)
    );
    assert_eq!(
        decision(
            &source,
            Limits {
                max_work: 0,
                ..Limits::default()
            }
        )
        .outcome,
        Outcome::Indeterminate(Reason::ResourceLimit)
    );
    assert_eq!(
        decision(
            &source,
            Limits {
                max_expansion_terms: 0,
                ..Limits::default()
            }
        )
        .outcome,
        Outcome::Indeterminate(Reason::ResourceLimit)
    );
    assert_eq!(
        decision(
            &source,
            Limits {
                deadline: Some(Instant::now() - Duration::from_millis(1)),
                ..Limits::default()
            }
        )
        .outcome,
        Outcome::Indeterminate(Reason::DeadlineExceeded)
    );
    let tolerance = ToleranceContext::default_valid();
    let cancelled = AtomicBool::new(true);
    let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), Some(&cancelled));
    assert_eq!(
        orient2d(
            &mut ctx,
            pair(&source, 0),
            pair(&source, 2),
            pair(&source, 4)
        )
        .unwrap()
        .outcome,
        Outcome::Indeterminate(Reason::Cancelled)
    );
    assert_eq!(ctx.work_used(), 0);
}

#[test]
fn residual_classification_requires_matching_enclosure_and_respects_gray_band() {
    let source = arena(&[0., 0., 1e-9, 0., 1e-6, 0., 1., 0.]);
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
    assert_eq!(
        classify_residual(&mut ctx, None).unwrap(),
        ModelClass::Indeterminate(Reason::MissingProof)
    );
    for (offset, expected) in [
        (2, ModelClass::Coincident),
        (4, ModelClass::Indeterminate(Reason::PrecisionExhausted)),
        (6, ModelClass::Separate),
    ] {
        let ResidualOutcome::Proven(proof) =
            distance_enclosure(&mut ctx, &pair(&source, 0), &pair(&source, offset)).unwrap()
        else {
            panic!("distance envelope")
        };
        assert_eq!(classify_residual(&mut ctx, Some(&proof)).unwrap(), expected);
        let other = ToleranceContext::default_valid();
        let mut other_ctx = PredicateContext::new(&source, &other, Limits::default(), None);
        assert_eq!(
            classify_residual(&mut other_ctx, Some(&proof)),
            Err(InputError::ContextMismatch)
        );
    }
    let mut invalid = tolerance.specification().clone();
    invalid.linear_abs = invalid.max_entity_error * 2.;
    assert!(ToleranceContext::new(invalid).is_err());
}

#[test]
fn portable_tolerance_identity_round_trips_and_detects_policy_changes() {
    let context = ToleranceContext::from_brep_tolerance_mm(1e-6).unwrap();
    let json = value_codec::to_string(&context).unwrap();
    let restored: ToleranceContext = value_codec::from_str_strict(&json).unwrap();
    assert!(context.is_compatible_with(&restored));
    assert_eq!(context.spec_identity(), restored.spec_identity());
    assert_eq!(context.spatial_bounds().on_mm, 1e-6);
    assert_eq!(context.angular_bounds().radians, 1e-9);
    assert_eq!(context.parametric_bounds().floor, 1e-12);
    assert_eq!(context.entity_error_bounds().maximum_mm, 1e-6 * 10.);

    let mut changed = context.specification().clone();
    changed.policy.push_str("-changed");
    let changed = ToleranceContext::new(changed).unwrap();
    assert!(!context.is_compatible_with(&changed));
    assert_ne!(context.spec_identity(), changed.spec_identity());
}

#[test]
fn serialized_tolerance_identity_cannot_be_substituted() {
    let context = ToleranceContext::from_brep_tolerance_mm(1e-6).unwrap();
    let mut value = value_codec::Serialize::to_value(&context);
    *value
        .get_mut("identity")
        .unwrap()
        .get_mut("canonical")
        .unwrap() = value_codec::Value::String("forged".into());
    assert!(<ToleranceContext as value_codec::Deserialize>::from_value(value).is_err());
}

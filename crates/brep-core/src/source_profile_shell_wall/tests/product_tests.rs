use super::*;
#[test]
fn intrinsic_exclusion_reuse_requires_exact_containment() {
    let outer = [[0., 1.], [0.25, 0.75]];
    assert!(rectangle_contains(outer, [[0.1, 0.9], [0.25, 0.5]]));
    assert!(rectangle_contains(outer, outer));
    assert!(!rectangle_contains(outer, [[0., 1.], [0.25, f64::from_bits(0.75f64.to_bits()+1)]]));
    assert!(!rectangle_contains(outer, [[0.9, 0.1], [0.25, 0.5]]));
    assert!(!rectangle_contains(outer, [[f64::NAN, 1.], [0.25, 0.5]]));
    assert!(!rectangle_contains(outer, [[0., f64::INFINITY], [0.25, 0.5]]));
}
#[cfg(feature="codec")]
#[test]
fn diagnostic_wire_preserves_domains_reasons_and_support_scope() {
    let report=ExpressionWallReport {
        minimum_mm:1.,max_sine_squared:1e-4,tolerance_uv:0.05,
        complete:false,
        pending:vec![ExpressionWallCell {faces:[2,3],domains:[[[0.,0.5],[0.2,0.4]];2],
            pending_reason:Some(ExpressionWallPendingReason::Budget)}],
        cells:0,spans:122,span_stages:ExpressionWallSpanStages {position:2,normal:3,shared_corner:5,original_pair:7,retained_pair:11,corrected_pair:13},distance_excluded:0,angle_excluded:0,planar_excluded:0,
        profile_transition_checked:2,profile_transition_excluded:1,profile_transition_spans:64,
        profile_transition_inconclusive:1,profile_transition_unresolved:0,
        intrinsic_checked:3,intrinsic_spans:17,intrinsic_unresolved:2,
        intrinsic_excluded:0,plane_controls:0,cache_hits:0,total_pairs:1,
        initial_pairs_checked:0,symmetry_reused:0,trim_excluded:0,domain_cells:0,
        budget_pending:1,enclosure_pending:0,resolution_pending:0,
    };
    let value=value_codec::Serialize::to_value(&report);
    let encoded=value_codec::to_string(&report).unwrap();
    let decoded:value_codec::Value=value_codec::from_str(&encoded).unwrap();
    assert_eq!(decoded,value);
    assert_eq!(value["spans"],value_codec::json!(122));
    for (stage,spans) in [("position",2),("normal",3),("shared_corner",5),
        ("original_pair",7),("retained_pair",11),("corrected_pair",13)] {
        assert_eq!(value["span_stages"][stage],value_codec::json!(spans));
    }
    assert_eq!(value["intrinsic_checked"],value_codec::json!(3));
    assert_eq!(value["intrinsic_spans"],value_codec::json!(17));
    assert_eq!(value["intrinsic_unresolved"],value_codec::json!(2));
    assert_eq!(value["profile_transition_checked"],value_codec::json!(2));
    assert_eq!(value["profile_transition_excluded"],value_codec::json!(1));
    assert_eq!(value["profile_transition_spans"],value_codec::json!(64));
    assert_eq!(value["profile_transition_inconclusive"],value_codec::json!(1));
    assert_eq!(value["profile_transition_unresolved"],value_codec::json!(0));
    assert_eq!(value["scope"],value_codec::json!("original-retained-transition-support-products"));
    assert_eq!(value["minimum_mm"],value_codec::json!(1.0));
    assert_eq!(value["max_sine_squared"],value_codec::json!(0.0001));
    assert_eq!(value["tolerance_uv"],value_codec::json!(0.05));
    assert_eq!(value["pending"][0]["pending_reason"],value_codec::json!("budget"));
    assert_eq!(value["pending"][0]["domains"],value_codec::json!(report.pending[0].domains));
    assert_eq!(value["pending"][0]["faces"],value_codec::json!([2,3]));
    assert!(!value.as_object().unwrap().contains_key("certificate"));
}
#[test]
fn exchange_reuses_only_exact_same_face_products() {
    let a = [[0., 0.5], [0.2, 0.4]];
    let b = [[0.5, 1.], [0.6, 0.8]];
    let cell = |faces, domains| ExpressionWallCell { faces, domains, pending_reason: None };
    assert_eq!(
        product_key(&cell([2, 2], [a, b])),
        product_key(&cell([2, 2], [b, a]))
    );
    assert_ne!(
        product_key(&cell([2, 3], [a, b])),
        product_key(&cell([2, 3], [b, a]))
    );
    let mut other = b;
    other[0][1] = 0.9;
    assert_ne!(
        product_key(&cell([2, 2], [a, b])),
        product_key(&cell([2, 2], [a, other]))
    );
}

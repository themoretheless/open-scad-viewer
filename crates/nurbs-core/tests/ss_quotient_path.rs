use nurbs_core::ss_intersection as ss;
#[path = "common/periodic_ss_wire.rs"]
mod fixture;
use fixture::wire;

#[test]
fn noncontractible_periodic_boundary_has_relative_material_transitions() {
    let wire = wire();
    wire.reverify().unwrap();
    assert!(ss::qualify_original_ss_winding(&wire, 0, [0.25, 0.25], Default::default()).is_err());
    for (a, b, odd) in [
        ([0.25, 0.25], [0.25, 0.75], true),
        ([3.99, 0.25], [4.01, 0.75], true),
        ([0.1, 0.25], [3.9, 0.25], false),
        ([8.25, 0.25], [8.25, 0.75], true),
    ] {
        let proof =
            ss::qualify_original_ss_path_parity(&wire, 0, a, b, Default::default()).unwrap();
        assert_eq!(proof.odd_crossings(), odd, "{a:?} -> {b:?}");
        proof.reverify().unwrap();
        let reversed =
            ss::qualify_original_ss_path_parity(&wire, 0, b, a, Default::default()).unwrap();
        assert_eq!(reversed.odd_crossings(), odd);
    }
    assert!(
        ss::qualify_original_ss_path_parity(
            &wire,
            0,
            [0.25, 0.5],
            [0.25, 0.75],
            ss::OriginalSsPathLimits {
                max_segments: 4,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        ss::qualify_original_ss_path_parity(
            &wire,
            0,
            [0.25, 0.25],
            [8.25, 0.75],
            ss::OriginalSsPathLimits {
                max_periodic_copies: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
}

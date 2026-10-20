use brep_core::profile_certificate as p;
use nurbs_core::curve::Curve;
fn rectangle(lo: [f64; 2], hi: [f64; 2]) -> Vec<Curve> {
    let points = [lo, [hi[0], lo[1]], hi, [lo[0], hi[1]], lo];
    points
        .windows(2)
        .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
        .collect()
}
#[test]
fn automatic_exact_sources_assemble_union_hole_and_disjoint_boundaries() {
    let loops = vec![rectangle([0., 0.], [4., 4.]), rectangle([1., 1.], [3., 3.])];
    let union = p::assemble_original_filled_boundaries(
        &loops,
        &[1, 1],
        1e-12,
        p::OriginalBoundaryAssemblyLimits::default(),
    )
    .unwrap();
    union.reverify_source().unwrap();
    assert_eq!(union.boundaries().len(), 1);
    assert_eq!(union.retained().len(), 4);
    let hole = p::assemble_original_filled_boundaries(
        &loops,
        &[1, -1],
        1e-12,
        p::OriginalBoundaryAssemblyLimits::default(),
    )
    .unwrap();
    hole.reverify_source().unwrap();
    assert_eq!(hole.boundaries().len(), 2);
    assert_eq!(hole.retained().len(), 8);
    let disconnected = vec![rectangle([0., 0.], [1., 1.]), rectangle([2., 2.], [3., 3.])];
    let separate = p::assemble_original_filled_boundaries(
        &disconnected,
        &[1, 1],
        1e-12,
        p::OriginalBoundaryAssemblyLimits::default(),
    )
    .unwrap();
    separate.reverify_source().unwrap();
    assert_eq!(separate.boundaries().len(), 2);
}
#[test]
fn automatic_exact_radius_handles_a_strip_narrower_than_display_tolerance() {
    let loops = vec![rectangle([0., 0.], [1., 2f64.powi(-40)])];
    let limited = p::OriginalBoundaryAssemblyLimits {
        radius_halvings: 0,
        ..Default::default()
    };
    assert!(p::assemble_original_filled_boundaries(&loops, &[1], 1e-12, limited).is_err());
    let region = p::assemble_original_filled_boundaries(
        &loops,
        &[1],
        1e-12,
        p::OriginalBoundaryAssemblyLimits::default(),
    )
    .unwrap();
    region.reverify_source().unwrap();
    assert_eq!(region.boundaries().len(), 1);
    assert_eq!(region.retained().len(), 4);
}
#[test]
fn automatic_irrational_contact_cuts_preserve_original_curved_sources() {
    let arch = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        weights: vec![1.; 3],
        control_points: vec![vec![-3., 0.], vec![0., 6.], vec![3., 0.]],
        periodic: false,
    };
    let base = Curve::from_polyline(vec![vec![3., 0.], vec![-3., 0.]]).unwrap();
    let loops = vec![vec![arch, base], rectangle([-4., 1.5], [4., 4.])];
    let region = p::assemble_original_filled_boundaries(
        &loops,
        &[-1, 1],
        1e-12,
        p::OriginalBoundaryAssemblyLimits::default(),
    )
    .unwrap();
    region.reverify_source().unwrap();
    assert_eq!(region.boundaries().len(), 1);
    assert_eq!(
        region.cycles().graph().owner().atlas().sources(),
        loops.iter().flatten().cloned().collect::<Vec<_>>()
    );
    assert!(
        region
            .retained()
            .iter()
            .any(|h| region.cycles().graph().owner().carriers()[h / 2].source == 0)
    );
}
#[test]
fn invalid_open_wire_and_excessive_allowance_cannot_be_automatically_admitted() {
    let open = vec![vec![
        Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap(),
    ]];
    assert!(
        p::assemble_original_filled_boundaries(&open, &[1], 1e-12, Default::default()).is_err()
    );
    let loops = vec![rectangle([0., 0.], [1., 1.])];
    assert!(
        p::assemble_original_filled_boundaries(&loops, &[], 1e-12, Default::default()).is_err()
    );
    assert!(
        p::assemble_original_filled_boundaries(
            &loops,
            &[1],
            1e-12,
            p::OriginalBoundaryAssemblyLimits {
                parameter_refinements: 513,
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn overlapping_algebraic_cut_boxes_need_actual_refinement_before_interior_choice() {
    let loops = vec![
        rectangle([0., 0.], [1., 1.]),
        rectangle([0.5, -0.5], [0.5 + 2f64.powi(-44), 1.5]),
    ];
    let limited = p::OriginalBoundaryAssemblyLimits {
        parameter_refinements: 0,
        ..Default::default()
    };
    assert!(p::assemble_original_filled_boundaries(&loops, &[1, 1], 1e-12, limited).is_err());
    let region =
        p::assemble_original_filled_boundaries(&loops, &[1, 1], 1e-12, Default::default()).unwrap();
    region.reverify_source().unwrap();
    assert_eq!(region.boundaries().len(), 1);
}

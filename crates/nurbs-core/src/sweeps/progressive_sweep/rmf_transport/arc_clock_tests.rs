use super::*;
use crate as nurbs;
use crate::progressive_sweep::{Options, Orientation, Spacing, Sweep, constant_vector_law};
#[path = "../../../../tests/support/arc_bishop_source.rs"]
mod source_case;
use source_case::path as spatial;
#[test]
fn spatial_original_arc_clock_has_geometric_g2_without_periodic_raw_speed_jets() {
    let p = spatial(true);
    let before = p.clone();
    let raw =
        certify_original_rmf_transport(&p, [0., 0., 1.], 1024, 100000, 1000000, true).unwrap();
    assert_eq!(raw.status, Status::Unresolved);
    assert!(raw.cover.is_none());
    let r =
        certify_original_arc_rmf_transport(&p, [0., 0., 1.], 1024, 100000, 1000000, true).unwrap();
    assert_eq!(
        r.status,
        Status::Certified,
        "{:?} cells={} work={}",
        r.reason,
        r.cells,
        r.exact_work
    );
    eprintln!(
        "geometric arc source transport cells={} exact={}",
        r.cells, r.exact_work
    );
    eprintln!("geometric arc angle={:?} radius={:?}",r.closing_angle,r.cover.as_ref().and_then(|c|c.last()).map(|c|c.endpoint_radius_upper));
    assert!(r.closing_angle.is_some() && r.original_arc_length.is_some());
    assert_eq!(r.arc_corrected_normals.as_ref().unwrap().len(), 1024);
    let profile = crate::primitives::line([0.1, 0., 0.], [0.2, 0., 0.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    for spacing in [
        Spacing::Parameter,
        Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 100000,
        },
    ] {
        let sweep = Sweep::new(
            &profile,
            &p,
            &scale,
            &twist,
            Options {
                normal: [0., 0., 1.],
                orientation: Orientation::RotationMinimizing,
                spacing,
                initial_sections: 65,
                max_sections: 65,
                max_deviation: 1.,
            },
        )
        .unwrap()
        .with_spatial_rmf_error_limits(1024, 100000, 1000000)
        .unwrap();
        let proof = sweep
            .certify_closed_path_frame_smoothness(2, 100000, 1000000)
            .unwrap();
        assert_eq!(
            proof.closed_source_frame_smoothness_certified,
            spacing != Spacing::Parameter,
            "{spacing:?} {proof:?}"
        );
    }
    assert_eq!(p, before);
    assert!(!r.continuous_bound && !r.solid_certified);
}
fn straight_c0_knot() -> Curve {
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![0.5, 0., 0.],
            vec![1., 0., 0.],
            vec![1.5, 0., 0.],
            vec![2., 0., 0.],
        ],
        weights: vec![1., 2., 4., 0.7, 1.],
        periodic: false,
    }
}
#[test]
fn original_c0_knot_position_and_forward_tangent_are_independent_mandatory_premises() {
    let p = straight_c0_knot();
    let r =
        certify_original_arc_rmf_transport(&p, [0., 0., 1.], 256, 100000, 1000000, false).unwrap();
    assert_eq!(r.status, Status::Certified, "{r:?}");
    for c in r.cover.as_ref().unwrap() {
        for k in 0..3 {
            let n = if k == 2 { 1. } else { 0. };
            assert!(c.normal[k][0] <= n && n <= c.normal[k][1]);
        }
    }
    let mut disconnected = spatial(true);
    disconnected.control_points[9][0] = f64::from_bits(1);
    let r =
        certify_original_arc_rmf_transport(&disconnected, [0., 0., 1.], 256, 100000, 1000000, true)
            .unwrap();
    assert_eq!(r.reason, Some("rmf-original-position-continuity-unproved"));
    assert!(r.cover.is_none());
    let mut reversed = p.clone();
    reversed.control_points[3][0] = 0.5;
    reversed.control_points[4][0] = 0.;
    let r =
        certify_original_arc_rmf_transport(&reversed, [0., 0., 1.], 256, 100000, 1000000, false)
            .unwrap();
    assert_eq!(r.reason, Some("rmf-original-forward-tangent-unproved"));
    assert!(r.cover.is_none());
}
#[test]
fn geometric_arc_clock_preserves_one_exact_and_cell_ledger_and_discards_partial_correction() {
    let p = spatial(true);
    let r =
        certify_original_arc_rmf_transport(&p, [0., 0., 1.], 128, 100000, 1000000, true).unwrap();
    assert_eq!(r.status, Status::Certified, "{r:?}");
    for (cells, exact) in [
        (0, 1000000),
        (r.cells - 1, 1000000),
        (100000, 0),
        (100000, r.exact_work - 1),
    ] {
        let x =
            certify_original_arc_rmf_transport(&p, [0., 0., 1.], 128, cells, exact, true).unwrap();
        assert_eq!(x.status, Status::Unresolved);
        assert!(x.cells <= cells && x.exact_work <= exact);
        assert!(
            x.cover.is_none()
                && x.closing_angle.is_none()
                && x.arc_corrected_normals.is_none()
                && x.arc_corrected_binormals.is_none()
        );
    }
    let total = r.cells + r.exact_work as usize;
    let spacing = Spacing::ArcLength {
        tolerance: 0.001,
        max_cells: 100000,
    };
    let full = certify_original_rmf_transport_shared_for_spacing(
        &p,
        [0., 0., 1.],
        128,
        total,
        true,
        spacing,
    )
    .unwrap();
    assert_eq!(full.status, Status::Certified);
    for budget in [0, total - 1] {
        let x = certify_original_rmf_transport_shared_for_spacing(
            &p,
            [0., 0., 1.],
            128,
            budget,
            true,
            spacing,
        )
        .unwrap();
        assert_eq!(x.status, Status::Unresolved);
        assert!(x.cells + x.exact_work as usize <= budget);
        assert!(x.cover.is_none());
    }
    let mut invalid = p;
    invalid.weights[9] = -1.;
    assert!(certify_original_arc_rmf_transport(&invalid, [0., 0., 1.], 128, 0, 0, true).is_err());
}
#[test]
fn geometric_arc_original_global_filled_volume_atlas() {
    use crate::progressive_sweep::MultiSweep;
    let (profiles,material,_)=source_case::material();let path=spatial(true);
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,Options{normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,spacing:Spacing::ArcLength{tolerance:0.001,max_cells:100000},initial_sections:33,max_sections:129,max_deviation:0.05}).unwrap().with_spatial_rmf_error_limits(4096,100000,1000000).unwrap();
    let r=sweep.certify_ideal_volume_atlas([&material,&material],[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],1000000,1000000).unwrap();
    assert!(r.embedding_certified && r.regularity_certified && r.parameter_topology_preserved,"{r:?}");
    eprintln!("original geometric-clock tube atlas: {r:?}");
    for (cells,exact) in [(0,1000000),(r.cells-1,1000000),(1000000,0),(1000000,r.exact_work-1)] {
        let short=sweep.certify_ideal_volume_atlas([&material,&material],[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],cells,exact).unwrap();
        assert!(!short.embedding_certified && !short.regularity_certified && !short.parameter_topology_preserved,"{short:?}");
        assert!(short.cells<=cells && short.exact_work<=exact);
    }

}

#[test]
fn geometric_arc_repeated_original_loop_has_no_global_embedding_credit() {
    use crate::progressive_sweep::MultiSweep;
    let (profiles,material,_)=source_case::material();let mut path=spatial(true);
    let poles=path.control_points.clone();let weights=path.weights.clone();
    path.control_points.extend_from_slice(&poles[1..]);
    path.weights.extend(weights[1..].iter().map(|w|w*weights[9]));
    path.knots=[vec![-3.;10],vec![1.;9],vec![5.;10]].concat();
    path.validate().unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,Options{normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,spacing:Spacing::ArcLength{tolerance:0.001,max_cells:100000},initial_sections:33,max_sections:129,max_deviation:0.05}).unwrap().with_spatial_rmf_error_limits(1024,100000,1000000).unwrap();
    let frame=sweep.sweeps[0].certify_closed_path_frame_smoothness(1,100000,1000000).unwrap();
    assert!(frame.closed_source_frame_smoothness_certified,"{frame:?}");
    // Two exact traversals share all original center-line points. Even valid
    // connected regular frame charts cannot supply global material embedding.
    let r=sweep.certify_ideal_volume_atlas([&material,&material],[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],200000,1000000).unwrap();
    assert!(!r.embedding_certified && !r.regularity_certified && !r.parameter_topology_preserved,"{r:?}");
    assert!(r.cells<=200000 && r.exact_work<=1000000);
    eprintln!("repeated original loop refused: {r:?}");
}

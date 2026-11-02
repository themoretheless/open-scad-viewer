use brep_core::{Model, region_replacement, watertight};
use nurbs_core::curve::Curve;
fn proposals(source: &Model, face: usize) -> Vec<(usize, Curve)> {
    source.loops[source.faces[face].outer]
        .coedges
        .iter()
        .map(|c| {
            (
                c.edge,
                if c.reversed {
                    c.pcurve.reverse().unwrap()
                } else {
                    c.pcurve.clone()
                },
            )
        })
        .collect()
}
#[test]
fn explicit_original_rim_clocks_are_freshly_admitted_without_projection() {
    let source = brep_core::cuboid([0.; 3], [2., 3., 4.]).unwrap();
    let before = source.clone();
    for face in 0..source.faces.len() {
        let p = proposals(&source, face);
        let result = region_replacement::cap_region_with_pcurves(
            &source,
            &[face],
            &source.faces[face].surface,
            &p,
            1e-8,
            100000,
            1000000,
        )
        .unwrap();
        assert!(result.interior.certified && result.replacement.trims.all_valid);
        assert_eq!(
            result.replacement.closure.closure.status,
            watertight::Closure::WithinTolerance
        );
        assert_eq!(result.replacement.model.faces.len(), 6);
        assert!(
            brep_core::solid_audit::audit_solid(&result.replacement.model)
                .unwrap()
                .ok
        );
    }
    assert_eq!(source, before);
}
#[test]
fn complete_rim_ownership_and_whole_lifts_are_required_for_authored_pcurves() {
    let source = brep_core::cuboid([0.; 3], [2., 3., 4.]).unwrap();
    let before = source.clone();
    let p = proposals(&source, 0);
    let mut duplicate = p.clone();
    duplicate[1] = duplicate[0].clone();
    let mut wrong_edge = p.clone();
    wrong_edge[0].0 = source.edges.len();
    let mut shifted = p.clone();
    for q in &mut shifted[0].1.control_points {
        q[0] += 0.01;
    }
    // Endpoints alone agree: a wrong interior pcurve must still fail.
    let mut bowed = p.clone();
    let a = bowed[0].1.control_points[0].clone();
    let b = bowed[0].1.control_points[1].clone();
    bowed[0].1 = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        weights: vec![1.; 3],
        periodic: false,
        control_points: vec![
            a.clone(),
            vec![(a[0] + b[0]) / 2. + 0.03, (a[1] + b[1]) / 2. + 0.03],
            b,
        ],
    };
    for candidate in [p[..3].to_vec(), duplicate, wrong_edge, shifted, bowed] {
        assert!(
            region_replacement::cap_region_with_pcurves(
                &source,
                &[0],
                &source.faces[0].surface,
                &candidate,
                1e-8,
                100000,
                1000000
            )
            .is_err()
        );
    }
    assert!(
        region_replacement::cap_region_with_pcurves(
            &source,
            &[0],
            &source.faces[0].surface,
            &p,
            1e-8,
            8,
            1000000
        )
        .is_err()
    );
    assert_eq!(source, before);
}

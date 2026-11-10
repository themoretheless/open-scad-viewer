use super::*;
fn source(
    hole: bool,
) -> (
    Vec<Vec<Curve>>,
    LoftCap,
    Vec<Vec<nurbs_core::surface::Surface>>,
) {
    let mut xy = vec![vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]]];
    if hole {
        xy.push(vec![[0.25, 0.25], [0.25, 0.75], [0.75, 0.75], [0.75, 0.25]]);
    }
    let profiles = xy
        .iter()
        .map(|r| {
            (0..4)
                .map(|i| {
                    nurbs_core::primitives::line(
                        [r[i][0], r[i][1], 0.],
                        [r[(i + 1) % 4][0], r[(i + 1) % 4][1], 0.],
                    )
                    .unwrap()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let cap = LoftCap {
        surface: nurbs_core::surface::Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        },
        trims: profiles
            .iter()
            .map(|r| {
                r.iter()
                    .map(|c| {
                        let mut c = c.clone();
                        for p in &mut c.control_points {
                            p.pop();
                        }
                        c
                    })
                    .collect()
            })
            .collect(),
    };
    let old = profiles
        .iter()
        .map(|r| {
            r.iter()
                .map(|c| {
                    let mut end = c.clone();
                    for p in &mut end.control_points {
                        p[2] += 1.;
                    }
                    nurbs_core::surface::loft(&[c.clone(), end]).unwrap()
                })
                .collect()
        })
        .collect();
    (profiles, cap, old)
}
fn limits() -> crate::AffineStationLimits {
    crate::AffineStationLimits {
        quantum: 1. / 65536.,
        tolerance: 0.001,
        work: 1000000,
        matrix_cells: 10000,
    }
}
fn model(c: &affine_candidate::Candidate) -> Model {
    crate::capped_loft_with_caps(&c.start, &c.end, &c.sides, [&c.caps[0], &c.caps[1]]).unwrap()
}
fn check(
    model: &Model,
    c: &affine_candidate::Candidate,
    cap: &LoftCap,
    l: crate::AffineStationLimits,
    spent: u64,
) -> Result<AffineFilledVolumeReport> {
    qualify(
        model,
        c,
        [cap, cap],
        1e-8,
        [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        l,
        spent,
    )
}
#[test]
fn fresh_actual_filled_map_owns_hole_roles_and_never_uses_a_successful_report_as_input() {
    let (original, cap, old) = source(true);
    let (c, prefix) = affine_candidate::prepare(&original, &old, [&cap, &cap], limits()).unwrap();
    let c = c.unwrap();
    let m = model(&c);
    let proof = check(&m, &c, &cap, limits(), prefix.work).unwrap();
    assert!(proof.certified && proof.packet_bound && proof.material_roles_certified);
    assert!(proof.agreement.as_ref().unwrap().all_equal && proof.trim.as_ref().unwrap().all_valid);
    assert!(proof.volume.as_ref().unwrap().parameter_topology_preserved);
    assert!(proof.orientation.as_ref().unwrap().sign.is_some());
    assert_eq!(m.faces.iter().filter(|f| !f.holes.is_empty()).count(), 2);
    assert!(proof.cells <= limits().matrix_cells && proof.work <= limits().work);
    for l in [
        crate::AffineStationLimits {
            work: prefix.work,
            ..limits()
        },
        crate::AffineStationLimits {
            matrix_cells: 0,
            ..limits()
        },
    ] {
        let r = check(&m, &c, &cap, l, prefix.work).unwrap();
        assert!(!r.certified && r.cells <= l.matrix_cells && r.work <= l.work);
    }
    let mut wrong = cap.clone();
    wrong.surface.control_points[1][1][2] = 0.125;
    let r = qualify(
        &m,
        &c,
        [&cap, &wrong],
        1e-8,
        [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        limits(),
        prefix.work,
    )
    .unwrap();
    assert!(!r.certified && r.reason == "actual-affine-source-packet-rebound");
}
#[test]
fn modified_actual_surfaces_uv_domains_hole_uses_and_orientation_refuse_fresh_admission() {
    let (original, cap, old) = source(true);
    let (c, prefix) = affine_candidate::prepare(&original, &old, [&cap, &cap], limits()).unwrap();
    let c = c.unwrap();
    let m = model(&c);
    assert!(
        check(&m, &c, &cap, limits(), prefix.work)
            .unwrap()
            .certified
    );
    let mut surface = m.clone();
    surface.faces[0].surface.control_points[0][0][0] += 0.125;
    let mut uv = m.clone();
    let wire = uv.faces[0].outer;
    uv.0.loops[wire].coedges[0].pcurve.control_points[1][0] = 0.5;
    let mut hole = m.clone();
    let cap_face = c.sides.iter().map(Vec::len).sum::<usize>();
    hole.0.faces[cap_face].holes.clear();
    let mut orientation = m.clone();
    orientation.0.shells[0].faces[0].reversed ^= true;
    for damaged in [surface, uv, hole, orientation] {
        assert!(
            match check(&damaged, &c, &cap, limits(), prefix.work) {
                Ok(r) => !r.certified,
                Err(_) => true,
            },
            "modified model received actual filled-volume authority"
        );
    }
    assert!(
        check(&m, &c, &cap, limits(), prefix.work)
            .unwrap()
            .certified
    );
}

#[test]
fn independent_cap_degrees_weights_and_uv_domains_share_the_actual_filled_certificate() {
    let (original, cap, old) = source(true);
    let mut other = cap.clone();
    other.surface.degree_u = 2;
    other.surface.degree_v = 2;
    other.surface.knots_u = vec![17., 17., 17., 29., 29., 29.];
    other.surface.knots_v = vec![-7., -7., -7., 13., 13., 13.];
    other.surface.control_points = [0., 0.25, 1.]
        .iter()
        .map(|&x| [0., 0.25, 1.].iter().map(|&y| vec![x, y, 0.]).collect())
        .collect();
    other.surface.weights = [1., 2., 3.]
        .iter()
        .map(|a| [1., 2., 3.].iter().map(|b| 8. * a * b).collect())
        .collect();
    for p in other
        .trims
        .iter_mut()
        .flatten()
        .flat_map(|c| &mut c.control_points)
    {
        p[0] = 17. + 12. * p[0];
        p[1] = -7. + 20. * p[1];
    }
    let before = other.clone();
    let (c, prefix) = affine_candidate::prepare(&original, &old, [&cap, &other], limits()).unwrap();
    let c = c.unwrap();
    let m = model(&c);
    let proof = qualify(
        &m,
        &c,
        [&cap, &other],
        1e-8,
        [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        limits(),
        prefix.work,
    )
    .unwrap();
    assert!(
        proof.certified,
        "{} cells={} work={}",
        proof.reason, proof.cells, proof.work
    );
    let identity = proof.carrier_identity.as_ref().unwrap();
    assert!(identity.certified && identity.cells > 0 && identity.exact_work > 0);
    assert!(
        proof.packet_bound
            && proof.material_roles_certified
            && proof.trim.as_ref().unwrap().all_valid
    );
    assert!(proof.volume.as_ref().unwrap().parameter_topology_preserved);
    // Rebinding BOTH source arguments to an equal but different interior must
    // not use this packet's poses/cap controls to certify another filled map.
    let mut rebound = other.clone();
    rebound.surface.control_points[1][1][2] += 1. / 1024.;
    let refusal = qualify(
        &m,
        &c,
        [&rebound, &rebound],
        1e-8,
        [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        limits(),
        prefix.work,
    )
    .unwrap();
    assert!(!refusal.certified && refusal.reason == "actual-affine-source-packet-rebound");
    assert_eq!(other.surface, before.surface);
    assert_eq!(other.trims, before.trims);
}

#[test]
fn classified_paired_caps_refuse_an_interior_fold_despite_exact_hole_traces() {
    let (original, cap, old) = source(true);
    let mut end = cap.clone();
    let q = [0., -9. / 16., 5. / 4., -9. / 16., 0.];
    end.surface = nurbs_core::surface::Surface {
        degree_u: 4,
        degree_v: 4,
        knots_u: [vec![0.; 5], vec![1.; 5]].concat(),
        knots_v: [vec![0.; 5], vec![1.; 5]].concat(),
        control_points: (0..5)
            .map(|i| {
                (0..5)
                    .map(|j| vec![i as f64 / 4., j as f64 / 4., -256. * q[i] * q[j]])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 5]; 5],
        periodic_u: false,
        periodic_v: false,
    };
    let before = (end.surface.clone(), end.trims.clone());
    let (candidate, prefix) =
        affine_candidate::prepare(&original, &old, [&cap, &end], limits()).unwrap();
    let candidate = candidate.expect("exact coherent boundary packet");
    let model = model(&candidate);
    let proof = qualify(
        &model,
        &candidate,
        [&cap, &end],
        1e-8,
        [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        limits(),
        prefix.work,
    )
    .unwrap();
    assert!(proof.packet_bound, "{}", proof.reason);
    assert!(!proof.certified && !proof.material_roles_certified);
    let volume = proof
        .volume
        .as_ref()
        .expect("must reach actual filled map, not only fail source binding");
    assert!(!volume.embedding_certified && !volume.parameter_topology_preserved);
    assert_eq!(proof.reason, "actual-affine-filled-volume-unproved");
    assert!(proof.cells <= limits().matrix_cells && proof.work <= limits().work);
    assert_eq!(end.surface, before.0);
    assert_eq!(end.trims, before.1);
}

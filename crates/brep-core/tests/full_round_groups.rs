use brep_core::{Model, feature_ops::full_round_groups};
use nurbs_core::curve::Curve;

fn source() -> Model {
    let points: [[f64; 2]; 10] = [
        [0., 0.],
        [1., -1.],
        [3., -1.],
        [4., 0.],
        [4., 4.],
        [4., 8.],
        [2., 8.],
        [0., 8.],
        [0., 4.],
        [0., 2.],
    ];
    let wire = (0..points.len())
        .map(|i| {
            Curve::from_polyline(vec![
                points[i].to_vec(),
                points[(i + 1) % points.len()].to_vec(),
            ])
            .unwrap()
        })
        .collect::<Vec<_>>();
    brep_core::prism::extrude(&[wire], 0., 6.).unwrap()
}
fn groups(m: &Model) -> [Vec<usize>; 3] {
    [(0, 0.), (1, 8.), (0, 4.)].map(|(axis, value)| {
        m.faces
            .iter()
            .enumerate()
            .filter_map(|(i, f)| {
                let controls = f
                    .surface
                    .control_points
                    .iter()
                    .flatten()
                    .collect::<Vec<_>>();
                (controls.iter().all(|p| p[axis] == value)
                    && controls.iter().any(|p| p[2] == 0.)
                    && controls.iter().any(|p| p[2] == 6.))
                .then_some(i)
            })
            .collect()
    })
}

#[test]
fn full_round_groups_collapse_selected_seams_and_keep_original_selection_ids() {
    let m = source();
    let before = m.clone();
    let mut g = groups(&m);
    assert_eq!(g.each_ref().map(Vec::len), [3, 2, 2]);
    // Collection order is not chain order; ownership determines that order.
    g[0].reverse();
    g[1].reverse();
    let r = full_round_groups(&m, [&g[0], &g[1], &g[2]]).unwrap();
    assert_eq!(m, before);
    assert_eq!(r.replaced_faces, g[1]);
    assert_eq!(r.support_faces, [g[0].clone(), g[2].clone()]);
    assert!((r.radius_mm - 2.).abs() < 1e-10);
    assert!(r.audit.ok);
    assert_eq!(r.tangency.model(), &r.model);
    assert_eq!(r.tangency.seams().len(), 3);
    assert_eq!(r.tangency.envelopes().len(), 2);
    assert!(
        r.tangency
            .envelopes()
            .iter()
            .all(|(_, p)| p.radius().error_upper() <= r.model.tolerance_mm)
    );
    assert!(
        r.tangency
            .seams()
            .iter()
            .all(|s| s.positions.proven && s.regularity_proven)
    );
    let volume = brep_core::analysis::mass_properties(&r.model, 1e-6, 2_000_000)
        .unwrap()
        .signed_volume_mm3;
    assert!((volume - (27. + 2. * std::f64::consts::PI) * 6.).abs() < 1e-4);
    let swapped = full_round_groups(&m, [&g[2], &g[1], &g[0]]).unwrap();
    assert!(swapped.audit.ok);
}

#[test]
fn full_round_groups_refuse_incomplete_disconnected_and_duplicate_selections() {
    let m = source();
    let g = groups(&m);
    let before = m.clone();
    let disconnected = [g[0][0], g[0][2]];
    assert!(full_round_groups(&m, [&disconnected, &g[1], &g[2]]).is_err());
    assert!(full_round_groups(&m, [&g[0], &g[1][..1], &g[2]]).is_err());
    assert!(full_round_groups(&m, [&g[0], &g[1], &g[0]]).is_err());
    assert!(full_round_groups(&m, [&[], &g[1], &g[2]]).is_err());
    assert!(full_round_groups(&m, [&[usize::MAX], &g[1], &g[2]]).is_err());
    assert_eq!(m, before);
}

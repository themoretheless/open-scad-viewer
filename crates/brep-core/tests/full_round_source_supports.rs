use brep_core::{
    feature_ops::full_round_curved::{build, source_supports},
    Model,
};
use nurbs_core::curve::Curve;
fn source(weight: f64) -> Model {
    let c = |p: [[f64; 2]; 3]| Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: p.map(|p| p.to_vec()).to_vec(),
        weights: vec![1., weight, 1.],
        periodic: false,
    };
    let line =
        |a: [f64; 2], b: [f64; 2]| Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap();
    brep_core::prism::extrude(
        &[vec![
            line([0., 0.], [6., 0.]),
            c([[6., 0.], [9., 6.], [6., 12.]]),
            line([6., 12.], [0., 12.]),
            c([[0., 12.], [-3., 6.], [0., 0.]]),
        ]],
        0.,
        6.,
    )
    .unwrap()
}
#[test]
fn all_native_side_images_are_covered_on_their_whole_uv_rectangles() {
    let r = build(&source(0.7), [3, 2, 1]).unwrap();
    let proof = r.qualify_source_supports().unwrap();
    assert_eq!(proof.side_count(), 4);
    assert_eq!(proof.bindings().len(), 4);
    proof.reverify().unwrap();
    proof.qualify_native_profiles().unwrap().reverify().unwrap();
    let lower = proof.qualify_lower_rims().unwrap();
    lower.reverify().unwrap();
    lower.qualify_vertical_seams().unwrap().reverify().unwrap();
}
#[test]
fn exact_degree_elevation_is_accepted_without_replacing_original_carrier() {
    let m = source(1.);
    let r = build(&m, [3, 2, 1]).unwrap();
    let mut changed = m.clone();
    let s = &mut changed.faces[1].surface;
    s.degree_u = 3;
    s.knots_u = vec![0., 0., 0., 0., 1., 1., 1., 1.];
    s.control_points = [[6., 0.], [8., 4.], [8., 8.], [6., 12.]]
        .iter()
        .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 6.]])
        .collect();
    s.weights = vec![vec![1., 1.]; 4];
    changed.validate().unwrap();
    let proof = source_supports::qualify(&changed, r.raw_profile_sources(), [0., 6.]).unwrap();
    assert_eq!(proof.source().faces[1].surface.degree_u, 3);
    assert_eq!(changed.faces[1].surface.degree_u, 3);
    let native = proof.qualify_native_profiles().unwrap();
    let original = native.regularity()[0].wire().sources()[1].source();
    assert_eq!(original.degree, 3);
    assert_eq!(
        original.control_points,
        vec![vec![6., 0.], vec![8., 4.], vec![8., 8.], vec![6., 12.]]
    );
    proof.reverify().unwrap();
    proof.qualify_native_profiles().unwrap().reverify().unwrap();
    let lower = proof.qualify_lower_rims().unwrap();
    lower.reverify().unwrap();
    lower.qualify_vertical_seams().unwrap().reverify().unwrap();
}
#[test]
fn projective_vertical_clock_keeps_the_entire_extruded_image() {
    let m = source(0.7);
    let r = build(&m, [3, 2, 1]).unwrap();
    let mut changed = m;
    for row in &mut changed.faces[1].surface.weights {
        row[1] *= 2.;
    }
    // z(v)=2v/(1+v); invert it on vertical pcurves so the original
    // affine world edges still agree pointwise with the surface composition.
    let w = changed.faces[1].outer;
    for c in &mut changed.loops[w].coedges {
        let a = &c.pcurve.control_points[0];
        let b = &c.pcurve.control_points[1];
        if a[0] == b[0] {
            c.pcurve.weights = if a[1] == 0. {
                vec![2., 1.]
            } else {
                vec![1., 2.]
            };
        }
    }
    changed.validate().unwrap();
    let proof = source_supports::qualify(&changed, r.raw_profile_sources(), [0., 6.]).unwrap();
    assert_eq!(
        proof.vertical_weight_ratio(1).unwrap(),
        &["2".to_string(), "1".to_string()]
    );
    proof.reverify().unwrap();
    proof.qualify_native_profiles().unwrap().reverify().unwrap();
    let lower = proof.qualify_lower_rims().unwrap();
    lower.reverify().unwrap();
    lower.qualify_vertical_seams().unwrap().reverify().unwrap();
}
#[test]
fn unchanged_top_rim_does_not_hide_interior_height_warp() {
    let m = source(0.7);
    let r = build(&m, [3, 2, 1]).unwrap();
    let mut changed = m;
    changed.faces[1].surface.control_points[1][0][0] += 1e-8;
    let error = source_supports::qualify(&changed, r.raw_profile_sources(), [0., 6.]).unwrap_err();
    assert!(error.to_string().contains("tensor factorization"));
}
#[test]
fn source_uv_gap_and_profile_mismatch_do_not_get_whole_support_proofs() {
    let m = source(0.7);
    let r = build(&m, [3, 2, 1]).unwrap();
    let mut changed = m.clone();
    let w = changed.faces[1].outer;
    changed.loops[w].coedges[0].pcurve.control_points[1][0] -= 2f64.powi(-48);
    assert!(source_supports::qualify(&changed, r.raw_profile_sources(), [0., 6.]).is_err());
    let mut changed = m;
    for p in &mut changed.faces[1].surface.control_points[1] {
        p[0] += 1e-8;
    }
    assert!(source_supports::qualify(&changed, r.raw_profile_sources(), [0., 6.]).is_err());
}

#[test]
fn transposed_original_side_axes_keep_whole_support_binding() {
    let m = source(0.7);
    let r = build(&m, [3, 2, 1]).unwrap();
    let mut changed = m;
    let old = changed.faces[1].surface.clone();
    let rows = old.control_points.len();
    let cols = old.control_points[0].len();
    let s = &mut changed.faces[1].surface;
    s.degree_u = old.degree_v;
    s.degree_v = old.degree_u;
    s.knots_u = old.knots_v;
    s.knots_v = old.knots_u;
    s.control_points = (0..cols)
        .map(|i| {
            (0..rows)
                .map(|j| old.control_points[j][i].clone())
                .collect()
        })
        .collect();
    s.weights = (0..cols)
        .map(|i| (0..rows).map(|j| old.weights[j][i]).collect())
        .collect();
    let w = changed.faces[1].outer;
    changed.loops[w].coedges.reverse();
    for c in &mut changed.loops[w].coedges {
        c.reversed = !c.reversed;
        c.pcurve = c.pcurve.reverse().unwrap();
        for p in &mut c.pcurve.control_points {
            p.swap(0, 1);
        }
    }
    for shell in &mut changed.shells {
        for use_ in &mut shell.faces {
            if use_.face == 1 {
                use_.reversed = !use_.reversed;
            }
        }
    }
    changed.validate().unwrap();
    let proof = source_supports::qualify(&changed, r.raw_profile_sources(), [0., 6.]).unwrap();
    assert_eq!(proof.extrusion_axis(1), Some(0));
    proof.reverify().unwrap();
    proof.qualify_native_profiles().unwrap().reverify().unwrap();
    let lower = proof.qualify_lower_rims().unwrap();
    lower.reverify().unwrap();
    lower.qualify_vertical_seams().unwrap().reverify().unwrap();
}

#[test]
fn lower_rim_interior_mismatch_is_not_hidden_by_valid_top_supports() {
    let m = source(0.7);
    let r = build(&m, [3, 2, 1]).unwrap();
    let mut changed = m.clone();
    let edge = changed
        .edges
        .iter_mut()
        .find(|e| e.curve.degree == 2 && e.curve.control_points.iter().all(|p| p[2] == 0.))
        .unwrap();
    edge.curve.control_points[1][0] += 1e-10;
    changed.validate().unwrap();
    let supports = source_supports::qualify(&changed, r.raw_profile_sources(), [0., 6.]).unwrap();
    assert!(supports.qualify_lower_rims().is_err());
    assert_eq!(m, *r.source());
}
#[test]
fn lower_rim_exact_binding_preserves_original_native_clocks() {
    let mut m = source(0.7);
    for (i, e) in m.edges.iter_mut().enumerate() {
        if e.curve.control_points.iter().all(|p| p[2] == 0.) {
            let offset = 0.1 + i as f64;
            for t in &mut e.curve.knots {
                *t = offset + *t * 1.8;
            }
        }
    }
    m.validate().unwrap();
    let r = build(&m, [3, 2, 1]).unwrap();
    let supports = r.qualify_source_supports().unwrap();
    let lower = supports.qualify_lower_rims().unwrap();
    assert_eq!(lower.bindings().len(), 4);
    assert_eq!(lower.supports().source(), &m);
    lower.reverify().unwrap();
}

#[test]
fn vertical_interior_bow_is_not_hidden_by_exact_horizontal_rims() {
    let m = source(0.7);
    let r = build(&m, [3, 2, 1]).unwrap();
    let mut changed = m.clone();
    let edge = changed
        .edges
        .iter_mut()
        .find(|e| {
            let p = &e.curve.control_points;
            p.len() == 2 && p[0][0] == p[1][0] && p[0][1] == p[1][1] && p[0][2] != p[1][2]
        })
        .unwrap();
    let mut middle = (0..3)
        .map(|k| (edge.curve.control_points[0][k] + edge.curve.control_points[1][k]) / 2.)
        .collect::<Vec<_>>();
    middle[0] += 1e-10;
    edge.curve.control_points.insert(1, middle);
    edge.curve.degree = 2;
    edge.curve.knots = vec![0., 0., 0., 1., 1., 1.];
    edge.curve.weights = vec![1.; 3];
    changed.validate().unwrap();
    let supports = source_supports::qualify(&changed, r.raw_profile_sources(), [0., 6.]).unwrap();
    let lower = supports.qualify_lower_rims().unwrap();
    assert!(lower.qualify_vertical_seams().is_err());
}

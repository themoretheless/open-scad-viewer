use nurbs_core::{
    curve::Curve,
    surface::{Axis, Surface},
};
use value_codec::{Value, json};
fn request() -> Value {
    let a = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![0.5, 0., 0.25], vec![0.5, 1., 0.25]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
    };
    let mut b = a.clone();
    for p in b.control_points.iter_mut().flatten() {
        p[2] += 1.;
    }
    let edges = |s: &Surface| {
        vec![vec![
            s.iso(Axis::V, 0.).unwrap(),
            s.iso(Axis::U, 1.).unwrap(),
            s.iso(Axis::V, 1.).unwrap().reverse().unwrap(),
            s.iso(Axis::U, 0.).unwrap().reverse().unwrap(),
        ]]
    };
    let (start, end): (Vec<Vec<Curve>>, Vec<Vec<Curve>>) = (edges(&a), edges(&b));
    let sides = vec![
        start[0]
            .iter()
            .zip(&end[0])
            .map(|(a, b)| nurbs_core::surface::loft(&[a.clone(), b.clone()]).unwrap())
            .collect::<Vec<_>>(),
    ];
    let p = [[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]];
    let trims = vec![
        p.windows(2)
            .map(|p| {
                nurbs_core::paths::bezier(p.iter().map(|p| p.to_vec()).collect(), None).unwrap()
            })
            .collect::<Vec<_>>(),
    ];
    json!({"op":"brep_nurbs_capped_loft_with_caps","start":start,"end":end,"sides":sides,
        "caps":[json!({"surface":a,"trims":trims.clone()}),json!({"surface":b,"trims":trims})],
        "toleranceUv":1e-9,"embeddingLimits":{"exactWork":1000000,"trimPairs":1000,"trimCells":10000,
            "trimDomainCells":100000,"spans":1000,"facePairs":100,"faceCells":100000,
            "faceDomainCells":100000,"faceCellsPerPair":1000,"faceDomainCellsPerPair":1000}})
}
#[test]
fn json_nonplanar_loft_requires_global_embedding_and_two_caps() {
    let request = request();
    let result = geometry_bridge::dispatch(request.clone()).unwrap();
    let model: brep_core::Model = value_codec::from_value(result).unwrap();
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    assert!((model.faces[4].surface.evaluate(0.5, 0.5).unwrap().point[2] - 0.125).abs() < 1e-12);
    let mut incomplete = request.clone();
    incomplete["embeddingLimits"]["facePairs"] = json!(1);
    assert!(geometry_bridge::dispatch(incomplete).is_err());
    let mut bad = request.clone();
    bad["caps"] = json!([]);
    assert!(geometry_bridge::dispatch(bad).is_err());
    let mut crossing = request;
    let mut side: Surface = value_codec::from_value(crossing["sides"][0][0].clone()).unwrap();
    side = side.edit_axis(Axis::V, |c| c.elevate(2)).unwrap();
    side.control_points[1][1][1] = 5.;
    crossing["sides"][0][0] = value_codec::to_value(side).unwrap();
    assert!(geometry_bridge::dispatch(crossing).is_err());
}

#[test]
fn json_multispan_section_mapping_retains_the_original_section() {
    let start = Curve {
        degree: 1,
        knots: vec![0., 0., 0.5, 1., 1.],
        control_points: vec![vec![0., 0., 0.], vec![0.5, 1., 0.], vec![1., 0., 0.]],
        weights: vec![1., 0.75, 1.],
        periodic: false,
    };
    let mut end = start.clone();
    for p in &mut end.control_points {
        p[2] += 2.;
    }
    let mapping = json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],"controlValues":[0.,0.25,1.],"weights":[1.,1.,1.]}]});
    let materialization = json!({"op":"curve_materialize_reparameterization_bounded","curve":start,
        "mapping":mapping,"errorBudget":1e-6,"maxCells":50000,"maxMapEvaluations":200000});
    let mapped = geometry_bridge::dispatch(materialization.clone()).unwrap();
    assert_eq!(mapped["certificate"]["exact"], false);
    assert_eq!(mapped["certificate"]["retention"]["accepted"], true);
    let mut depleted = materialization;
    depleted["maxCells"] = json!(1);
    assert!(geometry_bridge::dispatch(depleted).is_err());
    let loft = geometry_bridge::dispatch(
        json!({"op":"surface_natural_loft","curves":[start.clone(),end],
        "parameters":[0.,1.],"section_mappings":[mapping.clone(),mapping]}),
    )
    .unwrap();
    let surface: Surface = value_codec::from_value(loft).unwrap();
    for u in [0., 0.13, 0.37, 0.6180339887498949, 0.83, 1.] {
        let expected = start.evaluate((u + u * u) * 0.5).unwrap().point;
        for (v, z) in [(0., 0.), (1., 2.)] {
            let actual = surface.evaluate(u, v).unwrap().point;
            for axis in 0..3 {
                assert!(
                    (actual[axis] - expected[axis] - if axis == 2 { z } else { 0. }).abs() <= 1e-6
                );
            }
        }
    }
}

#[test]
fn json_loft_accepts_subdivision_certified_monotonic_section_map() {
    let start = Curve {
        degree: 1,
        knots: vec![0., 0., 0.5, 1., 1.],
        control_points: vec![vec![0., 0., 0.], vec![0.5, 1., 0.], vec![1., 0., 0.]],
        weights: vec![1., 0.75, 1.],
        periodic: false,
    };
    let mut end = start.clone();
    for point in &mut end.control_points {
        point[2] += 2.;
    }
    let mapping = json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],
        "controlValues":[0.,0.6,0.4,1.],"weights":[1.,1.,1.,1.]}]});
    let loft = geometry_bridge::dispatch(json!({"op":"surface_natural_loft",
        "curves":[start.clone(),end],"parameters":[0.,1.],
        "section_mappings":[mapping.clone(),mapping]}))
    .unwrap();
    let surface: Surface = value_codec::from_value(loft).unwrap();
    // Independently evaluate the authored cubic Bernstein polynomial.
    for sample in 0..=1000 {
        let u = sample as f64 / 1000.;
        let t = 1.8 * u * (1. - u).powi(2) + 1.2 * u * u * (1. - u) + u.powi(3);
        let expected = start.evaluate(t).unwrap().point;
        for (v, z) in [(0., 0.), (1., 2.)] {
            let actual = surface.evaluate(u, v).unwrap().point;
            let error = (0..3)
                .map(|axis| {
                    (actual[axis] - expected[axis] - if axis == 2 { z } else { 0. }).powi(2)
                })
                .sum::<f64>()
                .sqrt();
            assert!(error <= 1e-6, "u={u}, v={v}, error={error}");
        }
    }
}

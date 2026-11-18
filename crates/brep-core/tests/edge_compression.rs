use brep_core::cuboid;
use nurbs_core::curve::Curve;

fn refine_line(curve: &mut Curve) {
    assert_eq!(curve.degree, 1);
    assert_eq!(curve.control_points.len(), 2);
    assert_eq!(curve.weights, [1., 1.]);
    let middle = curve.control_points[0]
        .iter()
        .zip(&curve.control_points[1])
        .map(|(a, b)| (a + b) * 0.5)
        .collect();
    curve.control_points.insert(1, middle);
    curve.weights.insert(1, 1.);
    curve
        .knots
        .insert(2, (curve.knots[1] + curve.knots[2]) * 0.5);
}

#[test]
fn recovers_shared_edge_and_both_face_local_traces() {
    let mut source = cuboid([0.; 3], [8., 4., 2.]).unwrap();
    for usage in source
        .loops
        .iter_mut()
        .flat_map(|boundary| &mut boundary.coedges)
        .filter(|usage| usage.edge == 0)
    {
        for knot in &mut usage.pcurve.knots {
            *knot = 3. + 2. * *knot;
        }
        *usage = usage
            .clone()
            .with_trim(brep_topology::CoedgeTrim {
                curve_parameter: [0., 1.],
                pcurve_parameter: [3., 5.],
                periodic_lift: [[0; 2]; 2],
            })
            .unwrap();
    }
    let mut refined = source.clone();
    refine_line(&mut refined.edges[0].curve);
    let mut directions = Vec::new();
    for usage in refined
        .loops
        .iter_mut()
        .flat_map(|boundary| &mut boundary.coedges)
        .filter(|usage| usage.edge == 0)
    {
        refine_line(&mut usage.pcurve);
        directions.push(usage.reversed);
    }
    directions.sort();
    assert_eq!(directions, [false, true]);
    assert!(
        refined
            .compress_edge_with_trims(0, 1_000_000)
            .unwrap()
            .as_ref()
            == Some(&source)
    );
    assert!(refined.compress_edge_with_trims(0, 10).unwrap().is_none());
    assert!(refined.compress_edge_with_trims(usize::MAX, 100).is_err());
    assert!(refined.compress_edge_with_trims(0, 0).is_err());
}

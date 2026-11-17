use geometry_bridge::dispatch;
use value_codec::{Value, json};
#[allow(dead_code)]
#[path = "../../nurbs-core/tests/support/ss_closed_fiber.rs"]
mod intersections;
#[allow(dead_code)]
#[path = "../../brep-core/examples/support/fiber_cut_fixture.rs"]
mod sheets;
fn request() -> Value {
    let s = intersections::circle();
    let mut a = sheets::sheet(s.first().clone());
    let mut b = sheets::sheet(s.second().clone());
    a.tolerance_mm = 1e-6;
    b.tolerance_mm = 1e-6;
    json!({"op":"brep_partition_closed_ss","a":a,"b":b,"faceA":0,"faceB":0,"toleranceMm":1e-6,"toleranceUv":1e-6})
}
#[test]
fn native_dispatch_saves_both_closed_world_uv_partitions() {
    let input = request();
    let before = input.clone();
    let result = dispatch(input.clone()).unwrap();
    assert_eq!(input, before);
    assert_eq!(result["originalSourceClosedCoverage"], true);
    assert_eq!(
        result["coverageDomain"],
        json!([[0., 1.], [0., 1.], [0., 1.], [0., 1.]])
    );
    assert_eq!(result["materialClassified"], false);
    assert_eq!(result["a"]["faces"].as_array().unwrap().len(), 2);
    assert_eq!(result["b"]["faces"].as_array().unwrap().len(), 2);
    for key in ["a", "b"] {
        let m: brep_core::Model =
            value_codec::Deserialize::from_value(result[key].clone()).unwrap();
        m.validate().unwrap();
        assert!(m.persistent_naming_complete());
    }
}
#[test]
fn caller_flags_and_invalid_face_or_tolerance_cannot_admit_topology() {
    for (key, value) in [
        ("materialClassified", json!(true)),
        ("faceA", json!(99)),
        ("toleranceMm", json!(0.)),
        ("toleranceUv", json!(-1.)),
    ] {
        let mut input = request();
        input[key] = value;
        assert!(dispatch(input).is_err(), "accepted {key}");
    }
}
#[test]
fn all_closed_components_become_two_holes_and_three_finite_faces() {
    let s = intersections::two_ovals();
    let mut a = sheets::sheet(s.first().clone());
    let mut b = sheets::sheet(s.second().clone());
    a.tolerance_mm = 1e-4;
    b.tolerance_mm = 1e-4;
    let input = json!({"op":"brep_partition_all_closed_ss","a":a,"b":b,"faceA":0,"faceB":0,"toleranceMm":1e-4,"toleranceUv":1e-4,"maxRepairs":32});
    let before = input.clone();
    let result = dispatch(input.clone()).unwrap();
    assert_eq!(input, before);
    assert_eq!(result["branches"], 2);
    assert_eq!(result["materialClassified"], false);
    for key in ["a", "b"] {
        let m: brep_core::Model =
            value_codec::Deserialize::from_value(result[key].clone()).unwrap();
        m.validate().unwrap();
        assert_eq!(m.faces.len(), 3);
        assert_eq!(m.faces[0].holes.len(), 2);
        assert!(m.persistent_naming_complete());
    }
    let mut invalid = input.clone();
    invalid["maxRepairs"] = json!(129);
    assert!(dispatch(invalid).is_err());
    let mut invalid_attempts = input.clone();
    invalid_attempts["maxStartAttempts"] = json!(129);
    assert!(dispatch(invalid_attempts).is_err());
    let mut invalid = input;
    invalid["originalSourceClosedCoverage"] = json!(true);
    assert!(dispatch(invalid).is_err());
}

#[test]
fn finite_face_coverage_keeps_only_the_oval_inside_the_trim() {
    let source = intersections::two_ovals();
    let mut a = sheets::sheet(source.first().clone());
    let mut b = sheets::sheet(source.second().clone());
    a.tolerance_mm = 1e-4;
    b.tolerance_mm = 1e-4;
    for vertex in &mut a.vertices {
        vertex.point[0] *= 0.5;
    }
    for edge in &mut a.edges {
        for point in &mut edge.curve.control_points {
            point[0] *= 0.5;
        }
    }
    for coedge in &mut a.loops[0].coedges {
        for point in &mut coedge.pcurve.control_points {
            point[0] *= 0.5;
        }
    }
    let input = json!({
        "op":"brep_partition_all_closed_ss", "a":a, "b":b, "faceA":0, "faceB":0,
        "toleranceMm":1e-4, "toleranceUv":1e-4, "maxRepairs":32
    });
    let result = dispatch(input).unwrap();
    assert_eq!(result["branches"], 1);
    assert_eq!(result["originalSourceClosedCoverage"], false);
    assert_eq!(
        result["coverageDomain"],
        json!([[0., 0.5], [0., 1.], [0., 1.], [0., 1.]])
    );
    for key in ["a", "b"] {
        let model: brep_core::Model =
            value_codec::Deserialize::from_value(result[key].clone()).unwrap();
        model.validate().unwrap();
        assert_eq!(model.faces.len(), 2);
        assert_eq!(model.faces[0].holes.len(), 1);
    }
}

#[test]
fn periodic_carrier_is_partitioned_inside_one_native_chart() {
    let source = intersections::circle();
    let ring = [[0., 0.], [1., 1.], [2., 0.], [1., -1.], [0., 0.]];
    let mut tube = source.first().clone();
    tube.periodic_v = true;
    tube.knots_v = vec![-0.25, 0., 0.25, 0.5, 0.75, 1., 1.25];
    tube.control_points = [0., 1.]
        .map(|y| ring.map(|[x, z]| vec![x, y, z]).to_vec())
        .to_vec();
    tube.weights = vec![vec![1.; 5]; 2];
    let mut bowl = source.second().clone();
    for point in bowl.control_points.iter_mut().flatten() {
        let [u, v, z] = point.as_slice().try_into().unwrap();
        *point = vec![v, u, v + z];
    }
    check_periodic_partition(tube, bowl, [0.03125, 0.21875]);
}

#[test]
fn closed_ss_census_covers_both_sides_of_the_periodic_seam() {
    let source = intersections::circle();
    // The two quadratic spans at the seam are collinear and have the
    // same parameter speed. The rest of this periodic profile bends away.
    let ring = [
        [0., 0.],
        [1., 0.],
        [2., 0.],
        [3., 1.],
        [2., 2.],
        [-1., 2.],
        [-2., 1.],
        [-1., 0.],
        [0., 0.],
        [1., 0.],
    ];
    let mut tube = source.first().clone();
    tube.degree_v = 2;
    tube.periodic_v = true;
    tube.knots_v = (-2..=10).map(|k| f64::from(k) / 8.).collect();
    tube.control_points = [0., 1.]
        .map(|y| ring.map(|[x, z]| vec![x, y, z]).to_vec())
        .to_vec();
    tube.weights = vec![vec![1.; 10]; 2];
    let mut bowl = source.second().clone();
    for point in bowl.control_points.iter_mut().flatten() {
        point.swap(0, 1);
    }
    use nurbs_core::{ss_intersection as ss, surface::Axis};
    let mut lifted = nurbs_core::surface_edit::lift_two_periods(&tube, Axis::V).unwrap();
    for knot in &mut lifted.knots_v {
        *knot -= 1.;
    }
    let source = ss::FiberSources::new(&lifted, &bowl).unwrap();
    let domain = [[0.125, 0.875], [-0.0625, 0.0625], [0., 1.], [0., 1.]];
    let owner = ss::construct_qualified_closed_ss_branches_in_box(
        &source,
        domain,
        ss::ClosedSSBuildLimits {
            max_repairs: 32,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(owner.branches().len(), 1);
    assert_eq!(owner.owner().proof().coverage().domain(), domain);
    let roots = owner.root_enclosures();
    assert!(roots.iter().any(|bounds| bounds[1][1] < 0.));
    assert!(roots.iter().any(|bounds| bounds[1][0] > 0.));
    owner.reverify().unwrap();
    let request = periodic_partition_request(&tube, bowl, [-0.0625, 0.0625]);
    if let Some(path) = std::env::var_os("CLOSED_SS_SEAM_FIXTURE") {
        std::fs::write(path, value_codec::to_string(&request).unwrap()).unwrap();
    }
    let result = dispatch(request).unwrap();
    assert_eq!(result["branches"], 1);
    assert_eq!(result["originalSourceClosedCoverage"], false);
    assert_eq!(result["materialClassified"], false);
    assert!(result["worldErrorUpper"].as_f64().unwrap() < 1e-4);
    for error in result["uvErrorUpper"].as_array().unwrap() {
        assert!(error.as_f64().unwrap() < 1e-4);
    }
    for key in ["a", "b"] {
        let model: brep_core::Model =
            value_codec::Deserialize::from_value(result[key].clone()).unwrap();
        model.validate().unwrap();
        assert!(model.persistent_naming_complete());
        assert_eq!(model.faces.len(), 2);
        assert_eq!(model.faces[0].holes.len(), 1);
        if key == "a" {
            assert!(model.faces.iter().all(|face| face.surface == tube));
            let cut = &model.loops[model.faces[0].holes[0]];
            let v = cut
                .coedges
                .iter()
                .flat_map(|edge| &edge.pcurve.control_points)
                .map(|point| point[1]);
            assert!(v.clone().any(|value| value < 0.));
            assert!(v.clone().any(|value| value > 0.));
        }
    }
    ss::order_closed_fiber_cycle(
        &owner,
        0,
        ss::ClosedFiberCycleLimits {
            max_start_attempts: 50,
            ..Default::default()
        },
    )
    .unwrap()
    .reverify()
    .unwrap();
}

fn check_periodic_partition(
    tube: nurbs_core::surface::Surface,
    bowl: nurbs_core::surface::Surface,
    [lo, hi]: [f64; 2],
) {
    let result = dispatch(periodic_partition_request(&tube, bowl, [lo, hi])).unwrap();
    assert_eq!(result["branches"], 1);
    assert_eq!(result["originalSourceClosedCoverage"], false);
    let output: brep_core::Model =
        value_codec::Deserialize::from_value(result["a"].clone()).unwrap();
    output.validate().unwrap();
    assert_eq!(output.faces.len(), 2);
    assert_eq!(output.faces[0].holes.len(), 1);
    assert!(output.faces.iter().all(|face| face.surface == tube));
}

fn periodic_partition_request(
    tube: &nurbs_core::surface::Surface,
    bowl: nurbs_core::surface::Surface,
    [lo, hi]: [f64; 2],
) -> Value {
    let mut a = sheets::sheet(intersections::circle().first().clone());
    let corners = [[0.125, lo], [0.875, lo], [0.875, hi], [0.125, hi]];
    for (vertex, [u, v]) in a.vertices.iter_mut().zip(corners) {
        vertex.point = tube.evaluate(u, v.rem_euclid(1.)).unwrap().point;
    }
    let vertices = a.vertices.iter().map(|v| v.point).collect::<Vec<_>>();
    for edge in &mut a.edges {
        edge.curve = nurbs_core::curve::Curve::from_polyline(
            edge.vertices.map(|index| vertices[index].to_vec()).to_vec(),
        )
        .unwrap();
    }
    for (i, coedge) in a.loops[0].coedges.iter_mut().enumerate() {
        coedge.pcurve = nurbs_core::curve::Curve::from_polyline(vec![
            corners[i].to_vec(),
            corners[(i + 1) % 4].to_vec(),
        ])
        .unwrap();
    }
    a.faces[0].surface = tube.clone();
    let mut b = sheets::sheet(bowl);
    let tolerance = 1e-4;
    a.tolerance_mm = tolerance;
    b.tolerance_mm = tolerance;
    json!({
        "op":"brep_partition_all_closed_ss", "a":a, "b":b, "faceA":0, "faceB":0,
        "toleranceMm":tolerance, "toleranceUv":tolerance, "maxRepairs":32, "maxStartAttempts":50
    })
}

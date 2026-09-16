use brep_core::{
    Model, TopoId, analysis::mass_properties, boolean, cuboid, cylinder, sphere, tube,
};

fn translate(mut model: Model, offset: [f64; 3]) -> Model {
    for vertex in &mut model.vertices {
        for i in 0..3 {
            vertex.point[i] += offset[i];
        }
    }
    for point in model
        .0
        .edges
        .iter_mut()
        .flat_map(|e| &mut e.curve.control_points)
        .chain(
            model
                .0
                .faces
                .iter_mut()
                .flat_map(|f| f.surface.control_points.iter_mut().flatten()),
        )
    {
        for i in 0..3 {
            point[i] += offset[i];
        }
    }
    model.rebuild_topology_ids();
    model.validate().unwrap();
    model
}
fn volume(model: &Model) -> f64 {
    if model.is_empty() {
        0.
    } else {
        mass_properties(model, 1e-7, 300_000)
            .unwrap()
            .signed_volume_mm3
    }
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < b.abs().max(1.) * 1e-6, "{a} != {b}");
}

#[test]
fn empty_and_identical_operands_obey_regularized_set_identities() {
    let empty = Model::empty(1e-7).unwrap();
    assert!(Model::empty(f64::NAN).is_err());
    let solid = sphere(3.).unwrap();
    for operation in ["difference", "xor"] {
        assert!(boolean(&solid, &solid, operation).unwrap().is_empty());
    }
    for operation in ["union", "intersection"] {
        assert_eq!(
            boolean(&solid, &solid, operation).unwrap().1.faces,
            solid.1.faces
        );
    }
    for operation in ["union", "xor"] {
        near(
            volume(&boolean(&empty, &solid, operation).unwrap()),
            36. * std::f64::consts::PI,
        );
        assert_eq!(
            boolean(&solid, &empty, operation).unwrap().1.faces,
            solid.1.faces
        );
    }
    assert!(boolean(&empty, &solid, "difference").unwrap().is_empty());
    assert!(boolean(&solid, &empty, "intersection").unwrap().is_empty());
    let serialized = value_codec::Serialize::to_value(&empty);
    let roundtrip: Model = value_codec::Deserialize::from_value(serialized).unwrap();
    assert!(roundtrip.is_empty());
    roundtrip.validate().unwrap();
}

#[test]
fn fully_removed_and_contact_only_planar_regions_are_empty() {
    let inner = cuboid([1.; 3], [2.; 3]).unwrap();
    let outer = cuboid([0.; 3], [3.; 3]).unwrap();
    assert!(boolean(&inner, &outer, "difference").unwrap().is_empty());
    let touching = cuboid([3., 0., 0.], [4., 3., 3.]).unwrap();
    assert!(
        boolean(&outer, &touching, "intersection")
            .unwrap()
            .is_empty()
    );
    let remaining = boolean(&inner, &outer, "xor").unwrap();
    near(volume(&remaining), 26.);
    let rebuilt = boolean(
        &boolean(&inner, &outer, "difference").unwrap(),
        &outer,
        "union",
    )
    .unwrap();
    near(volume(&rebuilt), 27.);
}

#[test]
fn disjoint_curved_shells_keep_independent_topology_and_geometry() {
    let a = sphere(3.).unwrap();
    let b = translate(sphere(2.).unwrap(), [20., 0., 0.]);
    for operation in ["union", "xor"] {
        let result = boolean(&a, &b, operation).unwrap();
        assert_eq!(result.bodies.len(), 2);
        assert_eq!(result.vertices.len(), a.vertices.len() + b.vertices.len());
        assert!(
            a.1.faces
                .iter()
                .chain(&b.1.faces)
                .all(|id| result.1.faces.contains(id))
        );
        near(volume(&result), (36. + 32. / 3.) * std::f64::consts::PI);
    }
    assert!(boolean(&a, &b, "intersection").unwrap().is_empty());
    assert_eq!(boolean(&a, &b, "difference").unwrap().1.faces, a.1.faces);
}

#[test]
fn circle_circle_and_circle_line_csg_retains_curved_carriers() {
    let a = cylinder(3., 5.).unwrap();
    let b = translate(cylinder(3., 5.).unwrap(), [2., 0., 0.]);
    let disk = 9. * std::f64::consts::PI;
    let lens = 18. * (1. / 3f64).acos() - 2. * 8f64.sqrt();
    for (operation, expected) in [
        ("intersection", lens),
        ("union", 2. * disk - lens),
        ("difference", disk - lens),
        ("xor", 2. * (disk - lens)),
    ] {
        let result = boolean(&a, &b, operation).unwrap();
        result.validate().unwrap();
        assert!(result.edges.iter().any(|edge| edge.curve.degree == 2));
        assert!(
            result
                .faces
                .iter()
                .any(|f| f.surface.degree_u == 2 || f.surface.degree_v == 2)
        );
        near(volume(&result), expected * 5.);
        if operation == "xor" {
            assert_eq!(result.bodies.len(), 2);
            let mut reordered = result.clone();
            let count = reordered.vertices.len();
            reordered.vertices.reverse();
            for edge in &mut reordered.edges {
                edge.vertices = edge.vertices.map(|i| count - 1 - i);
            }
            let count = reordered.edges.len();
            reordered.edges.reverse();
            for wire in &mut reordered.loops {
                for use_ in &mut wire.coedges {
                    use_.edge = count - 1 - use_.edge;
                }
                wire.coedges.rotate_left(1);
            }
            reordered.rebuild_topology_ids();
            reordered.validate().unwrap();
            let mut original = result.clone();
            original.rebuild_topology_ids();
            let keys = |ids: &[TopoId]| {
                ids.iter()
                    .cloned()
                    .collect::<std::collections::BTreeSet<_>>()
            };
            assert_eq!(keys(&reordered.1.vertices), keys(&original.1.vertices));
            assert_eq!(keys(&reordered.1.edges), keys(&original.1.edges));
        }
    }
    let half = cuboid([0., -4., -1.], [4., 4., 6.]).unwrap();
    let cut = boolean(&a, &half, "difference").unwrap();
    near(volume(&cut), disk * 2.5);
    let rebuilt = boolean(&cut, &a, "union").unwrap();
    near(volume(&rebuilt), disk * 5.);
}

#[test]
fn prismatic_holes_and_z_overlap_keep_exact_round_sections() {
    let annulus = tube(3., 1., 5.).unwrap();
    let plug = cylinder(1., 5.).unwrap();
    near(
        volume(&boolean(&annulus, &plug, "union").unwrap()),
        45. * std::f64::consts::PI,
    );
    let elevated = translate(cylinder(3., 5.).unwrap(), [0., 0., 2.]);
    near(
        volume(&boolean(&annulus, &elevated, "intersection").unwrap()),
        24. * std::f64::consts::PI,
    );
    let stacked = boolean(&cylinder(3., 5.).unwrap(), &elevated, "union").unwrap();
    near(volume(&stacked), 63. * std::f64::consts::PI);
}

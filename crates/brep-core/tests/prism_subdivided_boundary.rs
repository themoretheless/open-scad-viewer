use brep_core::{Model, prism, prism_frame};
fn divided() -> Model {
    let source = brep_core::cuboid([0.; 3], [40., 30., 20.]).unwrap();
    let edge = source
        .edges
        .iter()
        .position(|e| {
            e.vertices
                .iter()
                .all(|&v| source.vertices[v].point[0] == 0. && source.vertices[v].point[2] == 20.)
        })
        .unwrap();
    let d = source.edges[edge].curve.domain();
    brep_core::feature_ops::edge_partition::split(
        &source,
        edge,
        &[d[0] + (d[1] - d[0]) * 0.25, d[0] + (d[1] - d[0]) * 0.75],
        "prism-subdivided-rim",
        Default::default(),
    )
    .unwrap()
    .model()
    .clone()
}
#[test]
fn subdivided_rectangular_side_preserves_prism_and_frame_recognition() {
    let m = divided();
    let before = m.clone();
    assert!(m.loops.iter().any(|l| l.coedges.len() > 4));
    assert!(prism::recognize(&m).unwrap().is_some());
    assert!(prism_frame::localize(&m, &m).unwrap().is_some());
    assert_eq!(m, before);
}
#[test]
fn small_gap_in_side_coverage_is_not_hidden_by_model_tolerance() {
    let mut m = divided();
    m.tolerance_mm = 1e-3;
    let f = m
        .faces
        .iter()
        .find(|f| {
            m.loops[f.outer].coedges.len() > 4 && {
                let z = f
                    .surface
                    .control_points
                    .iter()
                    .flatten()
                    .map(|p| p[2])
                    .collect::<Vec<_>>();
                z.iter().any(|x| *x == 0.) && z.iter().any(|x| *x == 20.)
            }
        })
        .unwrap()
        .outer;
    let slot = m.loops[f]
        .coedges
        .iter()
        .position(|c| {
            m.edges[c.edge]
                .vertices
                .iter()
                .all(|&v| m.vertices[v].point[2] == 20.)
        })
        .unwrap();
    let c = &mut m.loops[f].coedges[slot];
    let k = (0..2)
        .find(|&k| c.pcurve.control_points[0][k] != c.pcurve.control_points[1][k])
        .unwrap();
    let sign = (c.pcurve.control_points[1][k] - c.pcurve.control_points[0][k]).signum();
    c.pcurve.control_points[1][k] -= sign * 1e-9;
    m.validate().unwrap();
    assert!(prism::recognize(&m).unwrap().is_none());
}
#[test]
fn boolean_enclosure_with_subdivided_walls_reaches_audited_radius_one_fillet() {
    let m = brep_core::boolean(
        &brep_core::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
        &brep_core::cuboid([2.; 3], [38., 28., 22.]).unwrap(),
        "difference",
    )
    .unwrap();
    let before = m.clone();
    let edges = m
        .edges
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            e.vertices
                .iter()
                .all(|&v| m.vertices[v].point[0] == 0. && m.vertices[v].point[1] == 0.)
                .then_some(i)
        })
        .collect::<Vec<_>>();
    assert!(!edges.is_empty());
    let r = brep_core::exact_layered_prism_fillet(&m, &edges, 1.).unwrap();
    assert!(r.audit.ok && r.feature.complete);
    assert_eq!(m, before);
    let volume = brep_core::analysis::mass_properties(&r.model, 1e-7, 200_000)
        .unwrap()
        .signed_volume_mm3;
    let expected = 7152. - 20. * (1. - std::f64::consts::PI / 4.);
    assert!((volume - expected).abs() < 1e-5, "{volume} vs {expected}");
}

#[test]
fn explicit_full_natural_trim_metadata_preserves_the_subdivided_side() {
    let mut m = divided();
    let domains = m.edges.iter().map(|e| e.curve.domain()).collect::<Vec<_>>();
    for wire in &mut m.loops {
        for c in &mut wire.coedges {
            let trim = brep_topology::CoedgeTrim {
                curve_parameter: domains[c.edge],
                pcurve_parameter: c.pcurve.domain(),
                periodic_lift: [[0; 2]; 2],
            };
            *c = c.clone().with_trim(trim).unwrap();
        }
    }
    m.validate().unwrap();
    assert!(prism::recognize(&m).unwrap().is_some());
}

#[test]
fn layered_enclosure_accepts_a_subdivided_bottom_profile_without_changing_material() {
    let m = brep_core::boolean(
        &brep_core::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
        &brep_core::cuboid([2.; 3], [38., 28., 22.]).unwrap(),
        "difference",
    )
    .unwrap();
    let edge = m
        .edges
        .iter()
        .position(|e| {
            e.vertices
                .iter()
                .all(|&v| m.vertices[v].point[0] == 0. && m.vertices[v].point[2] == 0.)
        })
        .unwrap();
    let d = m.edges[edge].curve.domain();
    let divided = brep_core::feature_ops::edge_partition::split(
        &m,
        edge,
        &[d[0] + (d[1] - d[0]) * 0.25, d[0] + (d[1] - d[0]) * 0.75],
        "layered-bottom-subdivision",
        Default::default(),
    )
    .unwrap();
    let model = divided.model();
    let before = model.clone();
    assert!(prism_frame::localize(model, model).unwrap().is_some());
    let edges = model
        .edges
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            e.vertices
                .iter()
                .all(|&v| model.vertices[v].point[0] == 0. && model.vertices[v].point[1] == 0.)
                .then_some(i)
        })
        .collect::<Vec<_>>();
    let result = brep_core::exact_layered_prism_fillet(model, &edges, 1.).unwrap();
    assert!(result.audit.ok && result.feature.complete);
    assert_eq!(model, &before);
    let volume = brep_core::analysis::mass_properties(&result.model, 1e-7, 200_000)
        .unwrap()
        .signed_volume_mm3;
    assert!((volume - (7152. - 20. * (1. - std::f64::consts::PI / 4.))).abs() < 1e-5);
}

use brep_core::{
    Model,
    trimmed_shell::{self, Limits, Patch},
};
fn patches(model: &Model) -> Vec<Patch> {
    let mut reversed = vec![false; model.faces.len()];
    for shell in &model.shells {
        for face in &shell.faces {
            reversed[face.face] = face.reversed;
        }
    }
    model
        .faces
        .iter()
        .enumerate()
        .map(|(i, f)| Patch {
            surface: f.surface.clone(),
            outer: model.loops[f.outer].coedges.clone(),
            holes: f
                .holes
                .iter()
                .map(|&l| model.loops[l].coedges.clone())
                .collect(),
            reversed: reversed[i],
        })
        .collect()
}
#[test]
fn authored_cube_is_admitted_after_all_continuous_gates() {
    let source = brep_core::cuboid([0.; 3], [2., 3., 4.]).unwrap();
    let before = source.clone();
    let result = trimmed_shell::assemble(
        source.vertices.clone(),
        source.edges.clone(),
        patches(&source),
        Limits::default(),
    )
    .unwrap();
    assert!(result.trims.all_valid && result.vertex_links_valid && result.connected);
    assert!(result.closure.agreement.complete);
    assert!(
        result
            .regularity
            .iter()
            .all(|r| r.as_ref().unwrap().spanwise_regular)
    );
    let model = result.model.unwrap();
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    assert!(model.shells[0].closed);
    assert!(model.bodies.is_empty());
    assert_eq!(source, before);
}
#[test]
fn rational_curved_walls_and_hole_trims_form_a_manifold_shell() {
    let section = |z| {
        vec![
            vec![nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], 3.).unwrap()],
            vec![
                nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], 1.)
                    .unwrap()
                    .reverse()
                    .unwrap(),
            ],
        ]
    };
    let source = brep_core::rational_section_loft(&[section(0.), section(5.)]).unwrap();
    let result = trimmed_shell::assemble(
        source.vertices.clone(),
        source.edges.clone(),
        patches(&source),
        Limits::default(),
    )
    .unwrap();
    assert!(
        result.model.is_some(),
        "closure={:?}, trims={}, links={}, connected={}, regular={:?}",
        result.closure.status,
        result.trims.all_valid,
        result.vertex_links_valid,
        result.connected,
        result
            .regularity
            .iter()
            .map(|r| r.as_ref().map(|x| x.spanwise_regular))
            .collect::<Vec<_>>()
    );
    let model = result.model.unwrap();
    assert_eq!(
        model.faces.iter().filter(|f| !f.holes.is_empty()).count(),
        2
    );
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
}
#[test]
fn exhausted_audits_and_missing_patch_never_author_a_closed_shell() {
    let source = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let result = trimmed_shell::assemble(
        source.vertices.clone(),
        source.edges.clone(),
        patches(&source),
        Limits {
            agreement_cells: 1,
            ..Limits::default()
        },
    )
    .unwrap();
    assert!(result.model.is_none());
    assert!(!result.closure.agreement.complete);
    let mut open = patches(&source);
    open.pop();
    match trimmed_shell::assemble(
        source.vertices.clone(),
        source.edges.clone(),
        open,
        Limits::default(),
    ) {
        Ok(r) => assert!(r.model.is_none()),
        Err(_) => {}
    }
    assert!(
        trimmed_shell::assemble(
            source.vertices.clone(),
            source.edges.clone(),
            vec![],
            Limits::default()
        )
        .is_err()
    );
}

#[test]
fn matching_endpoints_do_not_hide_a_bulging_pcurve() {
    let source = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let mut network = patches(&source);
    let mut curve = network[0].outer[0].pcurve.elevate(3).unwrap();
    let a = &curve.control_points[0];
    let b = curve.control_points.last().unwrap();
    let axis = if (b[0] - a[0]).abs() > (b[1] - a[1]).abs() {
        1
    } else {
        0
    };
    let delta = if curve.control_points[1][axis] < 0.5 {
        0.1
    } else {
        -0.1
    };
    curve.control_points[1][axis] += delta;
    network[0].outer[0].pcurve = curve;
    let report = trimmed_shell::assemble(
        source.vertices.clone(),
        source.edges.clone(),
        network,
        Limits::default(),
    )
    .unwrap();
    assert!(report.vertex_links_valid && report.connected);
    assert!(report.model.is_none());
    assert!(
        report
            .closure
            .agreement
            .uses
            .iter()
            .any(|u| u.status == nurbs_core::curve_surface_agreement::Status::Mismatch)
    );
}

#[test]
fn unsupported_discontinuous_face_bases_are_refused() {
    let source = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let mut network = patches(&source);
    let surface = &mut network[0].surface;
    assert_eq!(surface.degree_u, 1);
    let middle: Vec<Vec<f64>> = surface.control_points[0]
        .iter()
        .zip(&surface.control_points[1])
        .map(|(a, b)| a.iter().zip(b).map(|(x, y)| 0.5 * x + 0.5 * y).collect())
        .collect();
    surface.control_points = vec![
        surface.control_points[0].clone(),
        middle.clone(),
        middle,
        surface.control_points[1].clone(),
    ];
    surface.weights = vec![vec![1.; 2]; 4];
    surface.knots_u = vec![0., 0., 0.5, 0.5, 1., 1.];
    let result = trimmed_shell::assemble(
        source.vertices.clone(),
        source.edges.clone(),
        network,
        Limits::default(),
    );
    match result {
        Err(e) => assert!(e.message.contains("Interior multiplicity"), "{e:?}"),
        Ok(r) => panic!(
            "Discontinuous basis unexpectedly reached admission: {}",
            r.model.is_some()
        ),
    }
}

use brep_core::{
    Model,
    feature_ops::{
        PatternMode,
        region_pattern::{self, FeatureRole, Limits, Placement},
    },
};
use nurbs_core::{curve::Curve, foundation::guards::Budget};
fn budget() -> Budget {
    Budget::new(10000000, 16, 60000).unwrap()
}
fn stock() -> Model {
    brep_core::cuboid([0.; 3], [12., 8., 2.]).unwrap()
}
fn boss() -> Model {
    brep_core::operations::boolean(
        &stock(),
        &brep_core::cuboid([1., 1., 2.], [3., 3., 3.]).unwrap(),
        "union",
    )
    .unwrap()
}
fn selected(m: &Model, low: f64, high: f64) -> Vec<usize> {
    m.faces
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] >= low - 1e-9 && p[2] <= high + 1e-9)
                && f.surface
                    .control_points
                    .iter()
                    .flatten()
                    .any(|p| p[2] > low + 1e-9)
        })
        .map(|(i, _)| i)
        .collect()
}
fn shift(key: &str, x: f64, z: f64) -> Placement {
    Placement {
        key: key.into(),
        matrix: [
            [1., 0., 0., x],
            [0., 1., 0., 0.],
            [0., 0., 1., z],
            [0., 0., 0., 1.],
        ],
    }
}
fn volume(m: &Model) -> f64 {
    brep_core::analysis::mass_properties(m, 1e-6, 2000000)
        .unwrap()
        .signed_volume_mm3
}
fn circle() -> Vec<Curve> {
    let r = 1.;
    let (x, y) = (2., 3.);
    let arcs = [
        [[x + r, y], [x + r, y + r], [x, y + r]],
        [[x, y + r], [x - r, y + r], [x - r, y]],
        [[x - r, y], [x - r, y - r], [x, y - r]],
        [[x, y - r], [x + r, y - r], [x + r, y]],
    ];
    arcs.into_iter()
        .map(|a| Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: a.into_iter().map(|p| p.to_vec()).collect(),
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        })
        .collect()
}
fn curved_source(pocket: bool) -> Model {
    let tool = brep_core::prism::extrude(
        &[circle()],
        if pocket { 1. } else { 2. },
        if pocket { 2. } else { 3. },
    )
    .unwrap();
    brep_core::operations::boolean(&stock(), &tool, if pocket { "difference" } else { "union" })
        .unwrap()
}
#[test]
fn original_boss_region_closes_on_its_support_and_patterns_with_lineage() {
    let mut s = boss();
    let vertex = s.vertices.iter().position(|v| v.point[2] == 3.).unwrap();
    let vertex_id = s.1.vertices[vertex];
    s.1.local_tolerances_mm
        .insert(vertex_id, s.tolerance_mm / 2.);
    let before = s.clone();
    let faces = selected(&s, 2., 3.);
    assert_eq!(faces.len(), 5);
    let f = region_pattern::extract(&s, &faces, FeatureRole::Additive, Limits::default()).unwrap();
    assert!((volume(f.tool()) - 4.).abs() < 1e-6);
    assert_eq!(f.tool().faces.len(), 6);
    assert_eq!(
        f.tool().1.local_tolerances_mm.get(&vertex_id),
        Some(&(s.tolerance_mm / 2.))
    );
    for &face in &faces {
        let id = s.1.faces[face];
        let new = f.tool().1.faces.iter().position(|p| *p == id).unwrap();
        assert_eq!(f.tool().faces[new].surface, s.faces[face].surface);
    }
    assert!(f.reverify(Limits::default()).unwrap().boundary_complete);
    let p = region_pattern::pattern(
        &s,
        &f,
        &[shift("one", 4., 0.)],
        PatternMode::Join,
        "boss-array",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert!((volume(p.model()) - 200.).abs() < 1e-5);
    assert!(p.model().persistent_naming_complete());
    let moved_vertex = p.instances()[0].source_ids()[&vertex_id];
    assert_eq!(
        p.instances()[0]
            .model()
            .1
            .local_tolerances_mm
            .get(&moved_vertex),
        Some(&(s.tolerance_mm / 2.))
    );
    assert!(
        !p.instances()[0]
            .model()
            .1
            .local_tolerances_mm
            .contains_key(&vertex_id)
    );
    let top = faces
        .iter()
        .copied()
        .find(|i| {
            s.faces[*i]
                .surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 3.)
        })
        .unwrap();
    let copy = p.instances()[0].source_ids()[&s.1.faces[top]];
    assert!(!p.descendants(copy).is_empty());
    assert!(p.reverify(&budget()).unwrap().audit.ok);
    assert_eq!(s, before);
}
#[test]
fn curved_original_walls_remain_rational_through_join_and_cut_patterns() {
    let s = curved_source(false);
    let faces = selected(&s, 2., 3.);
    let f = region_pattern::extract(&s, &faces, FeatureRole::Additive, Limits::default()).unwrap();
    assert!(
        f.tool()
            .faces
            .iter()
            .any(|f| f.surface.degree_u > 1 || f.surface.degree_v > 1)
    );
    for (i, id) in f.tool().1.edges.iter().enumerate() {
        let original = s.1.edges.iter().position(|v| v == id).unwrap();
        assert_eq!(f.tool().edges[i].curve, s.edges[original].curve);
    }
    assert!((volume(f.tool()) - std::f64::consts::PI).abs() < 1e-5);
    let p = region_pattern::pattern(
        &s,
        &f,
        &[shift("one", 4., 0.)],
        PatternMode::Join,
        "round-boss-array",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert!((volume(p.model()) - 192. - 2. * std::f64::consts::PI).abs() < 1e-4);
    assert!(p.model().edges.iter().any(|e| e.curve.degree > 1));
    let cut = region_pattern::pattern(
        &s,
        &f,
        &[shift("one", 4., -1.)],
        PatternMode::Cut,
        "round-pocket-array",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert!((volume(cut.model()) - 192.).abs() < 1e-4);
    assert!(cut.reverify(&budget()).unwrap().boundary_complete);
}
#[test]
fn nonextruded_original_spline_face_is_copied_without_reconstructing_a_depth() {
    let mut s = boss();
    let top = s
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 3.)
        })
        .unwrap();
    let t =
        nurbs_core::surface_edit::elevate(&s.faces[top].surface, nurbs_core::surface::Axis::U, 3)
            .unwrap();
    let mut t = nurbs_core::surface_edit::elevate(&t, nurbs_core::surface::Axis::V, 3).unwrap();
    t.control_points[1][1][2] += 0.2;
    s.faces[top].surface = t.clone();
    s.validate().unwrap();
    let faces = selected(&s, 2., 3.3);
    let f = region_pattern::extract(&s, &faces, FeatureRole::Additive, Limits::default()).unwrap();
    let copied = f
        .tool()
        .1
        .faces
        .iter()
        .position(|id| *id == s.1.faces[top])
        .unwrap();
    assert_eq!(f.tool().faces[copied].surface, t);
    assert!((volume(f.tool()) - 4.05).abs() < 1e-5);
    let empty = Model::empty(s.tolerance_mm).unwrap();
    let p = region_pattern::pattern(
        &empty,
        &f,
        &[shift("curved", 4., 0.)],
        PatternMode::Join,
        "freeform-feature",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert!((volume(p.model()) - 4.05).abs() < 1e-5);
    assert!(p.reverify(&budget()).unwrap().charts_injective);
}
#[test]
fn pocket_region_inverts_material_use_and_patterns_the_actual_removed_volume() {
    let s = curved_source(true);
    let faces = s
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            let vertices = std::iter::once(&f.outer)
                .chain(&f.holes)
                .flat_map(|w| s.loops[*w].coedges.iter())
                .flat_map(|c| s.edges[c.edge].vertices)
                .map(|v| s.vertices[v].point)
                .collect::<Vec<_>>();
            vertices.iter().all(|p| {
                p[0] >= 1. - 1e-9
                    && p[0] <= 3. + 1e-9
                    && p[1] >= 2. - 1e-9
                    && p[1] <= 4. + 1e-9
                    && p[2] >= 1. - 1e-9
                    && p[2] <= 2. + 1e-9
            }) && vertices.iter().any(|p| p[2] < 2. - 1e-9)
        })
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    let f =
        region_pattern::extract(&s, &faces, FeatureRole::Subtractive, Limits::default()).unwrap();
    assert!((volume(f.tool()) - std::f64::consts::PI).abs() < 1e-5);
    assert!(region_pattern::extract(&s, &faces, FeatureRole::Additive, Limits::default()).is_err());
    let p = region_pattern::pattern(
        &s,
        &f,
        &[shift("next", 4., 0.)],
        PatternMode::Cut,
        "pocket-region",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert!((volume(p.model()) - 192. + 2. * std::f64::consts::PI).abs() < 1e-4);
}
#[test]
fn occurrence_ids_follow_keys_across_reordering_and_parameter_edits() {
    let s = boss();
    let f = region_pattern::extract(
        &s,
        &selected(&s, 2., 3.),
        FeatureRole::Additive,
        Limits::default(),
    )
    .unwrap();
    let a = region_pattern::pattern(
        &s,
        &f,
        &[shift("a", 4., 0.), shift("b", 8., 0.)],
        PatternMode::Join,
        "stable",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    let b = region_pattern::pattern(
        &s,
        &f,
        &[shift("b", 8., 0.), shift("a", 4., 0.)],
        PatternMode::Join,
        "stable",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert_eq!(a.instances()[0].tool_ids(), b.instances()[1].tool_ids());
    assert_eq!(a.instances()[1].tool_ids(), b.instances()[0].tool_ids());
    let c = region_pattern::pattern(
        &s,
        &f,
        &[shift("a", 5., 0.)],
        PatternMode::Join,
        "stable",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert_eq!(a.instances()[0].tool_ids(), c.instances()[0].tool_ids());
    assert!((volume(a.model()) - 204.).abs() < 1e-5);
    let mut reordered = s.clone();
    let n = reordered.faces.len();
    reordered.faces.reverse();
    reordered.1.faces.reverse();
    for shell in &mut reordered.shells {
        for u in &mut shell.faces {
            u.face = n - 1 - u.face;
        }
    }
    reordered.validate().unwrap();
    let f2 = region_pattern::extract(
        &reordered,
        &selected(&reordered, 2., 3.),
        FeatureRole::Additive,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        f.tool().1.faces[f.cap_face()],
        f2.tool().1.faces[f2.cap_face()]
    );
    let p2 = region_pattern::pattern(
        &reordered,
        &f2,
        &[shift("a", 4., 0.)],
        PatternMode::Join,
        "stable",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert_eq!(a.instances()[0].tool_ids(), p2.instances()[0].tool_ids());
}
#[test]
fn invalid_selections_keys_work_and_transforms_publish_no_partial_pattern() {
    let s = boss();
    let before = s.clone();
    let faces = selected(&s, 2., 3.);
    for ids in [vec![], vec![999], vec![faces[0], faces[0]], vec![faces[0]]] {
        assert!(
            region_pattern::extract(&s, &ids, FeatureRole::Additive, Limits::default()).is_err()
        );
    }
    let mut scarce = Limits::default();
    scarce.max_cells = 1;
    assert!(region_pattern::extract(&s, &faces, FeatureRole::Additive, scarce).is_err());
    let f = region_pattern::extract(&s, &faces, FeatureRole::Additive, Limits::default()).unwrap();
    assert!(
        region_pattern::pattern(
            &s,
            &f,
            &[shift("same", 4., 0.), shift("same", 8., 0.)],
            PatternMode::Join,
            "bad",
            Limits::default(),
            &budget()
        )
        .is_err()
    );
    let mut invalid = shift("bad", 4., 0.);
    invalid.matrix[2][2] = 0.;
    assert!(
        region_pattern::pattern(
            &s,
            &f,
            &[invalid],
            PatternMode::Join,
            "bad",
            Limits::default(),
            &budget()
        )
        .is_err()
    );
    let small = brep_core::cuboid([1.25, 1.25, 2.25], [2.75, 2.75, 2.75]).unwrap();
    let empty = region_pattern::pattern(
        &small,
        &f,
        &[shift("cover", 0., 0.)],
        PatternMode::Cut,
        "empty",
        Limits::default(),
        &budget(),
    )
    .unwrap();
    assert!(empty.model().is_empty());
    assert_eq!(empty.evidence().numerical_volume_mm3, 0.);
    assert_eq!(s, before);
}

#[test]
fn original_edge_and_pcurve_clocks_are_retained_independently() {
    let mut s = curved_source(false);
    for edge in &mut s.edges {
        for knot in &mut edge.curve.knots {
            *knot = -7. + 20. * *knot;
        }
    }
    for wire in &mut s.loops {
        for coedge in &mut wire.coedges {
            for knot in &mut coedge.pcurve.knots {
                *knot = 2. + 3. * *knot;
            }
        }
    }
    s.validate().unwrap();
    let f = region_pattern::extract(
        &s,
        &selected(&s, 2., 3.),
        FeatureRole::Additive,
        Limits::default(),
    )
    .unwrap();
    for (i, id) in f.tool().1.edges.iter().enumerate() {
        let original = s.1.edges.iter().position(|v| v == id).unwrap();
        assert_eq!(f.tool().edges[i].curve, s.edges[original].curve);
    }
    assert!(f.reverify(Limits::default()).unwrap().boundary_complete);
}

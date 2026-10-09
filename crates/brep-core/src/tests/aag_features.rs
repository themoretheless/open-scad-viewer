use super::*;

fn budget() -> Budget {
    Budget::new(1_000_000, 8, 60_000).unwrap()
}

fn graph_with_attrs(model: &Model) -> Aag {
    let mut graph = Aag::build(model, &budget()).unwrap();
    graph.attach_face_attrs(model, &budget()).unwrap();
    graph
}

/// Cup profile in the (r, z) half-plane: outer radius 5, height 6, bore
/// radius 2 from z=2 to z=6 (blind hole, floor at z=2). CCW.
const CUP: [[f64; 2]; 6] = [
    [0., 0.],
    [5., 0.],
    [5., 6.],
    [2., 6.],
    [2., 2.],
    [0., 2.],
];

#[test]
fn through_hole_in_tube() {
    // Inner wall of a tube is four quadrant patches of one cylinder:
    // split-cylinder grouping must merge them into one through hole.
    let model = crate::analytic::tube(5., 2., 6.).unwrap();
    let graph = graph_with_attrs(&model);
    let holes = find_holes(&model, &graph, &budget()).unwrap();
    assert_eq!(holes.len(), 1, "exactly the bore: {holes:?}");
    let hole = &holes[0];
    assert_eq!(hole.kind, HoleKind::Through);
    assert_eq!(hole.entry, HoleEntry::Plain);
    assert!(hole.faces.len() >= 4, "all inner patches merged: {:?}", hole.faces);
    assert!((hole.diameter - 4.).abs() < 1e-4, "diameter: {}", hole.diameter);
    assert!((hole.depth - 6.).abs() < 1e-6, "depth: {}", hole.depth);
    assert!(
        (hole.angle_span - 2. * std::f64::consts::PI).abs() < 0.3,
        "full circle: {}",
        hole.angle_span
    );
    assert!(hole.bottom_faces.is_empty());
    // The outer wall (convex edges) must not appear anywhere.
    assert!(hole.faces.iter().all(|&f| {
        graph.nodes[f]
            .edges
            .iter()
            .all(|&e| graph.edges[e].class != DihedralClass::Convex)
    }));
}

#[test]
fn blind_hole_in_revolved_cup() {
    let model = crate::analytic::revolve(&CUP).unwrap();
    let graph = graph_with_attrs(&model);
    let holes = find_holes(&model, &graph, &budget()).unwrap();
    assert_eq!(holes.len(), 1, "{holes:?}");
    let hole = &holes[0];
    assert_eq!(hole.kind, HoleKind::Blind);
    assert_eq!(hole.entry, HoleEntry::Plain);
    assert!((hole.diameter - 4.).abs() < 1e-4);
    assert!((hole.depth - 4.).abs() < 1e-6, "depth z=2..6: {}", hole.depth);
    assert!(!hole.bottom_faces.is_empty(), "floor face reported");
    let axis = hole.axis;
    assert!((axis.direction[2].abs() - 1.).abs() < 1e-6, "bore along z");
}

#[test]
fn countersink_blind_hole() {
    // Bore r=2 for z in 2..4, then a cone flaring to r=4 at z=6.
    let profile = [
        [0., 0.],
        [5., 0.],
        [5., 6.],
        [4., 6.],
        [2., 4.],
        [2., 2.],
        [0., 2.],
    ];
    let model = crate::analytic::revolve(&profile).unwrap();
    let graph = graph_with_attrs(&model);
    let holes = find_holes(&model, &graph, &budget()).unwrap();
    assert_eq!(holes.len(), 1, "{holes:?}");
    let hole = &holes[0];
    assert_eq!(hole.kind, HoleKind::Blind);
    assert_eq!(hole.entry, HoleEntry::Countersink);
    assert!((hole.diameter - 4.).abs() < 1e-4);
    assert!(
        !hole.step_faces.is_empty(),
        "cone step faces reported (one per patch)"
    );
    assert!(
        hole.step_faces.iter().all(|&f| {
            graph.nodes[f].attrs.as_ref().unwrap().class == SurfaceClass::Cone
        }),
        "step faces are the cone patches"
    );
}

#[test]
fn counterbore_blind_hole() {
    // Bore r=2 for z in 2..4, larger coaxial cylinder r=4 for z in 4..6.
    let profile = [
        [0., 0.],
        [5., 0.],
        [5., 6.],
        [4., 6.],
        [4., 4.],
        [2., 4.],
        [2., 2.],
        [0., 2.],
    ];
    let model = crate::analytic::revolve(&profile).unwrap();
    let graph = graph_with_attrs(&model);
    let holes = find_holes(&model, &graph, &budget()).unwrap();
    assert_eq!(holes.len(), 1, "{holes:?}");
    let hole = &holes[0];
    assert_eq!(hole.kind, HoleKind::Blind);
    assert_eq!(hole.entry, HoleEntry::Counterbore);
    assert!((hole.diameter - 4.).abs() < 1e-4);
    assert!(!hole.step_faces.is_empty(), "counterbore step faces reported");
}

#[test]
fn half_hole_on_body_boundary_matches_by_angle_range() {
    // Partial (180°) revolution of the cup: the bore wall spans π, not a
    // full circle, and the two cut planes cap the profile. Matching must
    // not require a complete circumference.
    let model = crate::analytic::revolve_angle(&CUP, 180.).unwrap();
    let graph = graph_with_attrs(&model);
    let holes = find_holes(&model, &graph, &budget()).unwrap();
    assert_eq!(holes.len(), 1, "{holes:?}");
    let hole = &holes[0];
    assert_eq!(hole.kind, HoleKind::Blind);
    assert!(
        (hole.angle_span - std::f64::consts::PI).abs() < 0.35,
        "half coverage, not full: {}",
        hole.angle_span
    );
    assert!((hole.diameter - 4.).abs() < 1e-4);
}

#[test]
fn solid_primitives_have_no_holes() {
    // Boss cylinders and boxes read convex at the wall boundary: rejected.
    for model in [
        crate::cuboid([0.; 3], [4.; 3]).unwrap(),
        crate::analytic::cylinder(2., 5.).unwrap(),
        crate::analytic::sphere(3.).unwrap(),
        crate::analytic::torus(4., 1.).unwrap(),
        crate::analytic::frustum(3., 1.5, 4.).unwrap(),
    ] {
        let graph = graph_with_attrs(&model);
        let holes = find_holes(&model, &graph, &budget()).unwrap();
        assert!(holes.is_empty(), "false positive: {holes:?}");
    }
}

#[test]
fn missing_attrs_are_a_typed_error() {
    let model = crate::analytic::tube(5., 2., 6.).unwrap();
    let graph = Aag::build(&model, &budget()).unwrap();
    let error = find_holes(&model, &graph, &budget()).unwrap_err();
    assert_eq!(error.code, "BREP_AAG_FEATURES_INPUT");
}

#[test]
fn budget_exhaustion_is_a_typed_error() {
    let model = crate::analytic::tube(5., 2., 6.).unwrap();
    let graph = graph_with_attrs(&model);
    let tight = Budget::with_iterations(2).unwrap();
    let error = find_holes(&model, &graph, &tight).unwrap_err();
    assert!(
        error.code.contains("RESOURCE") || error.code.contains("BUDGET"),
        "budget exhaustion must surface as a resource error: {error:?}"
    );
}

#[test]
fn corpus_precision_recall_and_through_blind() {
    // Labeled corpus: five models with exactly one hole each, four solid
    // controls with none. Precision and recall must both be 100% (≥ 95%
    // required) and through/blind must be exact on 100% of the corpus.
    let cases: Vec<(Model, Option<HoleKind>)> = vec![
        (crate::analytic::tube(5., 2., 6.).unwrap(), Some(HoleKind::Through)),
        (crate::analytic::revolve(&CUP).unwrap(), Some(HoleKind::Blind)),
        (
            crate::analytic::revolve(&[
                [0., 0.],
                [5., 0.],
                [5., 6.],
                [4., 6.],
                [2., 4.],
                [2., 2.],
                [0., 2.],
            ])
            .unwrap(),
            Some(HoleKind::Blind),
        ),
        (
            crate::analytic::revolve(&[
                [0., 0.],
                [5., 0.],
                [5., 6.],
                [4., 6.],
                [4., 4.],
                [2., 4.],
                [2., 2.],
                [0., 2.],
            ])
            .unwrap(),
            Some(HoleKind::Blind),
        ),
        (
            crate::analytic::revolve_angle(&CUP, 180.).unwrap(),
            Some(HoleKind::Blind),
        ),
        (crate::cuboid([0.; 3], [4.; 3]).unwrap(), None),
        (crate::analytic::cylinder(2., 5.).unwrap(), None),
        (crate::analytic::sphere(3.).unwrap(), None),
        (crate::analytic::torus(4., 1.).unwrap(), None),
    ];
    let (mut tp, mut fp, mut fn_) = (0usize, 0usize, 0usize);
    let (mut kind_ok, mut kind_total) = (0usize, 0usize);
    for (model, expected) in &cases {
        let graph = graph_with_attrs(model);
        let holes = find_holes(model, &graph, &budget()).unwrap();
        match (expected, holes.len()) {
            (Some(want), 1) => {
                tp += 1;
                kind_total += 1;
                if holes[0].kind == *want {
                    kind_ok += 1;
                }
            }
            (Some(_), 0) => fn_ += 1,
            (Some(_), _) => {
                tp += 1;
                fp += holes.len() - 1;
            }
            (None, n) => fp += n,
        }
    }
    let precision = tp as f64 / (tp + fp).max(1) as f64;
    let recall = tp as f64 / (tp + fn_).max(1) as f64;
    assert!(
        precision >= 0.95 && recall >= 0.95,
        "precision {precision}, recall {recall} (tp={tp} fp={fp} fn={fn_})"
    );
    assert_eq!(
        kind_ok, kind_total,
        "through/blind must be correct on 100% of the corpus"
    );
}

// ------------------------------------------------------------------
// Pockets (checklist 867)
// ------------------------------------------------------------------

/// Closed rectangular pocket: floor at z=2 (36×26), walls z=2..20.
fn closed_pocket_model() -> Model {
    crate::boolean(
        &crate::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
        &crate::cuboid([2., 2., 2.], [38., 28., 22.]).unwrap(),
        "difference",
    )
    .unwrap()
}

#[test]
fn closed_pocket_straight_walls() {
    let model = closed_pocket_model();
    let graph = graph_with_attrs(&model);
    let pockets = find_pockets(&model, &graph, &budget()).unwrap();
    assert_eq!(pockets.len(), 1, "{pockets:?}");
    let p = &pockets[0];
    assert_eq!(p.kind, PocketKind::Closed);
    assert_eq!(p.walls, PocketWalls::Straight);
    assert_eq!(p.draft_angle_deg, None);
    assert!((p.depth - 18.).abs() < 1e-6, "depth: {}", p.depth);
    assert!(
        (p.floor_area - 36. * 26.).abs() < 1e-6,
        "floor area: {}",
        p.floor_area
    );
    assert!((p.floor_normal[2] - 1.).abs() < 1e-9, "floor normal +z");
    assert!(p.parent.is_none() && p.children.is_empty());
    assert!(!p.floor_faces.is_empty() && p.wall_faces.len() >= 4);
    assert!(p.fillet_faces.is_empty(), "no blends in this model");
}

#[test]
fn open_pocket_at_body_boundary() {
    // Slot cut through the x=40 side: the floor reaches the exterior.
    let model = crate::boolean(
        &crate::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
        &crate::cuboid([30., 2., 2.], [42., 28., 22.]).unwrap(),
        "difference",
    )
    .unwrap();
    let graph = graph_with_attrs(&model);
    let pockets = find_pockets(&model, &graph, &budget()).unwrap();
    assert_eq!(pockets.len(), 1, "{pockets:?}");
    let p = &pockets[0];
    assert_eq!(p.kind, PocketKind::Open, "slot opens at x=40");
    assert!((p.depth - 18.).abs() < 1e-6);
    assert_eq!(p.wall_faces.len(), 3, "three walls, the fourth side open");
}

#[test]
fn drafted_pocket_reports_draft_angle() {
    // Truncated-pyramid tool: 16×16 at z=2 growing to 22×22 at z=25,
    // i.e. 3 mm of lean over 23 mm of height on each side.
    let tool = crate::operations::faceted_loft(&[
        vec![
            [12., 7., 2.],
            [28., 7., 2.],
            [28., 23., 2.],
            [12., 23., 2.],
        ],
        vec![
            [9., 4., 25.],
            [31., 4., 25.],
            [31., 26., 25.],
            [9., 26., 25.],
        ],
    ])
    .unwrap();
    let model = crate::boolean(
        &crate::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
        &tool,
        "difference",
    )
    .unwrap();
    let graph = graph_with_attrs(&model);
    let pockets = find_pockets(&model, &graph, &budget()).unwrap();
    assert_eq!(pockets.len(), 1, "{pockets:?}");
    let p = &pockets[0];
    assert_eq!(p.walls, PocketWalls::Drafted, "{p:?}");
    let expected = (3f64 / 23.).atan().to_degrees();
    let draft = p.draft_angle_deg.unwrap();
    assert!(
        (draft - expected).abs() < 0.2,
        "draft {draft}, expected {expected}"
    );
    assert_eq!(p.kind, PocketKind::Closed);
}

/// Open pocket with two floor blends, built as a prism: the cross-section
/// is a 40×20 rectangle with a 8-wide, 3-deep notch whose bottom corners
/// are rounded (r=1 quarter arcs), extruded z=0..10. The revolved
/// counterpart is `revolved_round_pocket_model` (reachable since the AAG
/// pcurve-synchronization fix in `aag::classify_edge`).
fn filleted_pocket_model() -> Model {
    let line = |a: [f64; 2], b: [f64; 2]| nurbs_core::curve::Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    };
    let arc = |p0: [f64; 2], p1: [f64; 2], p2: [f64; 2]| nurbs_core::curve::Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![p0.to_vec(), p1.to_vec(), p2.to_vec()],
        weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
        periodic: false,
    };
    let wire = vec![
        line([0., 0.], [40., 0.]),
        line([40., 0.], [40., 20.]),
        line([40., 20.], [24., 20.]),
        line([24., 20.], [24., 18.]),
        arc([24., 18.], [24., 17.], [23., 17.]),
        line([23., 17.], [17., 17.]),
        arc([17., 17.], [16., 17.], [16., 18.]),
        line([16., 18.], [16., 20.]),
        line([16., 20.], [0., 20.]),
        line([0., 20.], [0., 0.]),
    ];
    crate::prism::extrude(&[wire], 0., 10.).unwrap()
}

/// Round pocket revolved from an analytic wire (regression for the AAG
/// pcurve-synchronization fix): cylindrical wall, quarter-arc floor blend
/// (torus), flat disk floor. Body z 0..8, radius 12, pocket radius 10
/// with floor at z=5 and a radius-1 blend.
fn revolved_round_pocket_model() -> Model {
    let line = |a: [f64; 2], b: [f64; 2]| nurbs_core::curve::Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    };
    let arc = |p0: [f64; 2], p1: [f64; 2], p2: [f64; 2]| nurbs_core::curve::Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![p0.to_vec(), p1.to_vec(), p2.to_vec()],
        weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
        periodic: false,
    };
    let wire = vec![
        line([0., 0.], [12., 0.]),
        line([12., 0.], [12., 8.]),
        line([12., 8.], [10., 8.]),
        line([10., 8.], [10., 6.]),
        arc([10., 6.], [10., 5.], [9., 5.]),
        line([9., 5.], [0., 5.]),
        line([0., 5.], [0., 0.]),
    ];
    crate::revolve_wire(&wire, 1e-7).unwrap()
}

#[test]
fn revolved_round_pocket_matches_through_fixed_aag() {
    let model = revolved_round_pocket_model();
    let graph = graph_with_attrs(&model);
    // Sanity: tangent line→arc joints classify Smooth, not Degenerate.
    assert_eq!(
        graph.edges_of_class(crate::aag::DihedralClass::Degenerate),
        graph
            .edges
            .iter()
            .filter(|e| model.edges[e.edge].degenerate)
            .map(|e| e.edge)
            .collect::<Vec<_>>(),
        "no falsely degenerate tangent joints"
    );
    let pockets = find_pockets(&model, &graph, &budget()).unwrap();
    assert_eq!(pockets.len(), 1, "{pockets:?}");
    let p = &pockets[0];
    assert_eq!(p.kind, PocketKind::Closed, "round pocket is blind");
    assert_eq!(p.walls, PocketWalls::Straight);
    assert!(!p.fillet_faces.is_empty(), "torus blend absorbed");
    assert!(p.fillet_faces.iter().all(|f| p.faces.contains(f)));
    // Rim at z=8, floor at z=5; floor disk radius 9.
    assert!((p.depth - 3.).abs() < 1e-6, "depth: {}", p.depth);
    assert!(
        (p.floor_area - 81. * std::f64::consts::PI).abs() < 1e-4,
        "floor area: {}",
        p.floor_area
    );
    assert!((p.floor_normal[2] - 1.).abs() < 1e-9, "floor normal +z");
}

#[test]
fn pocket_with_floor_fillet_collapses_chain() {
    let model = filleted_pocket_model();
    let graph = graph_with_attrs(&model);
    // Sanity: both blends are real fillet chains for 869.
    let chains = crate::aag_fillets::find_fillet_chains(&model, &graph, &budget()).unwrap();
    assert_eq!(chains.len(), 2, "two floor blend chains: {chains:?}");
    let pockets = find_pockets(&model, &graph, &budget()).unwrap();
    assert_eq!(pockets.len(), 1, "{pockets:?}");
    let p = &pockets[0];
    assert_eq!(p.kind, PocketKind::Open, "the prism slot is through in z");
    assert_eq!(p.walls, PocketWalls::Straight);
    let chain_faces: usize = chains.iter().map(|c| c.faces.len()).sum();
    assert_eq!(
        p.fillet_faces.len(),
        chain_faces,
        "both chains are absorbed into the pocket"
    );
    assert!(p.fillet_faces.iter().all(|f| p.faces.contains(f)));
    // Depth: rim at y=20 down to the floor at y=17; floor is 6×10.
    assert!((p.depth - 3.).abs() < 1e-6, "depth: {}", p.depth);
    assert!(
        (p.floor_area - 60.).abs() < 1e-6,
        "floor area: {}",
        p.floor_area
    );
    assert!((p.floor_normal[1] - 1.).abs() < 1e-9, "floor normal +y");
    assert_eq!(p.wall_faces.len(), 2, "two straight walls");
}

#[test]
fn nested_pockets_form_a_hierarchy() {
    // Outer pocket floor at z=2; a deeper pit cut into that floor down
    // to z=1 — its rim lands on the outer pocket's floor.
    let outer = closed_pocket_model();
    let model = crate::boolean(
        &outer,
        &crate::cuboid([13., 13., 1.], [27., 27., 12.]).unwrap(),
        "difference",
    )
    .unwrap();
    let graph = graph_with_attrs(&model);
    let pockets = find_pockets(&model, &graph, &budget()).unwrap();
    assert_eq!(pockets.len(), 2, "two pockets, not one merged: {pockets:?}");
    let (outer_i, inner_i) = pockets
        .iter()
        .enumerate()
        .partition::<Vec<_>, _>(|(_, p)| p.depth > 2.);
    let outer = &pockets[outer_i[0].0];
    let inner = &pockets[inner_i[0].0];
    assert!(outer.parent.is_none(), "outer is top-level");
    assert_eq!(inner.parent, Some(outer_i[0].0), "inner nests in outer");
    assert_eq!(outer.children, vec![inner_i[0].0]);
    assert!((inner.depth - 1.).abs() < 1e-6, "inner depth: {}", inner.depth);
    // The inner rim must land on the outer pocket's floor.
    assert!(
        inner
            .rim_faces
            .iter()
            .all(|f| outer.faces.contains(f) || outer.floor_faces.contains(f)),
        "rim lands inside the parent: {:?}",
        inner.rim_faces
    );
}

#[test]
fn boss_and_primitives_have_no_pockets() {
    // Boss on a plate: the mirror subgraph (concave base ring, convex
    // top rim) must be rejected by the convex internal corner edges and
    // the wall probes landing in material. Zero false positives.
    let boss = crate::boolean(
        &crate::cuboid([0.; 3], [40., 30., 2.]).unwrap(),
        &crate::cuboid([15., 10., 2.], [25., 20., 12.]).unwrap(),
        "union",
    )
    .unwrap();
    for model in [
        boss,
        crate::cuboid([0.; 3], [4.; 3]).unwrap(),
        // A through tube's annular cap grows the bore wall as a
        // "wall"; the concave bottom ring must reject the subgraph.
        crate::analytic::tube(5., 2., 6.).unwrap(),
        crate::analytic::cylinder(2., 5.).unwrap(),
    ] {
        let graph = graph_with_attrs(&model);
        let pockets = find_pockets(&model, &graph, &budget()).unwrap();
        assert!(pockets.is_empty(), "false positive pocket: {pockets:?}");
    }
}

#[test]
fn pockets_require_fresh_attrs_and_budget() {
    let model = closed_pocket_model();
    let graph = Aag::build(&model, &budget()).unwrap();
    let error = find_pockets(&model, &graph, &budget()).unwrap_err();
    assert_eq!(error.code, "BREP_AAG_POCKETS_INPUT");
    let graph = graph_with_attrs(&model);
    let tight = Budget::with_iterations(2).unwrap();
    let error = find_pockets(&model, &graph, &tight).unwrap_err();
    assert!(
        error.code.contains("RESOURCE") || error.code.contains("BUDGET"),
        "budget exhaustion must surface as a resource error: {error:?}"
    );
}

#[test]
fn pocket_corpus_recall_and_hierarchy() {
    // Labeled corpus: five pocket models (closed, open, drafted,
    // floor-filleted, nested×2 = 6 expected pockets) and two negative
    // controls (boss, plain block). Recall ≥ 90% required; hierarchy and
    // zero boss false positives asserted exactly.
    let drafted_tool = crate::operations::faceted_loft(&[
        vec![
            [12., 7., 2.],
            [28., 7., 2.],
            [28., 23., 2.],
            [12., 23., 2.],
        ],
        vec![
            [9., 4., 25.],
            [31., 4., 25.],
            [31., 26., 25.],
            [9., 26., 25.],
        ],
    ])
    .unwrap();
    let nested = crate::boolean(
        &closed_pocket_model(),
        &crate::cuboid([13., 13., 1.], [27., 27., 12.]).unwrap(),
        "difference",
    )
    .unwrap();
    let boss = crate::boolean(
        &crate::cuboid([0.; 3], [40., 30., 2.]).unwrap(),
        &crate::cuboid([15., 10., 2.], [25., 20., 12.]).unwrap(),
        "union",
    )
    .unwrap();
    let cases: Vec<(Model, usize)> = vec![
        (closed_pocket_model(), 1),
        (
            crate::boolean(
                &crate::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
                &crate::cuboid([30., 2., 2.], [42., 28., 22.]).unwrap(),
                "difference",
            )
            .unwrap(),
            1,
        ),
        (
            crate::boolean(
                &crate::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
                &drafted_tool,
                "difference",
            )
            .unwrap(),
            1,
        ),
        (filleted_pocket_model(), 1),
        (nested, 2),
        (boss, 0),
        (crate::cuboid([0.; 3], [4.; 3]).unwrap(), 0),
    ];
    let (mut found, mut expected_total, mut extra) = (0usize, 0usize, 0usize);
    for (model, want) in &cases {
        let graph = graph_with_attrs(model);
        let pockets = find_pockets(model, &graph, &budget()).unwrap();
        found += pockets.len().min(*want);
        extra += pockets.len().saturating_sub(*want);
        expected_total += want;
    }
    let recall = found as f64 / expected_total.max(1) as f64;
    assert!(
        recall >= 0.9 && extra == 0,
        "recall {recall} ({found}/{expected_total}), false positives {extra}"
    );
}

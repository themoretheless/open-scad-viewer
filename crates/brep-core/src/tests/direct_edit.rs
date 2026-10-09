use super::offset::radial_distance;
use super::*;
use crate::aag::Aag;
use crate::aag_fillets::find_fillet_chains;
use crate::aag_features::{find_holes, find_pockets};
use crate::analytic_features::analytic_fillet;

fn budget() -> Budget {
    Budget::new(10_000_000, 8, 60_000).unwrap()
}

fn graph_with_attrs(model: &Model) -> Aag {
    let mut graph = Aag::build(model, &budget()).unwrap();
    graph.attach_face_attrs(model, &budget()).unwrap();
    graph
}

fn vertical_edges(model: &Model) -> Vec<usize> {
    model
        .edges
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let a = model.vertices[e.vertices[0]].point;
            let b = model.vertices[e.vertices[1]].point;
            ((a[0] - b[0]).abs() <= 1e-12 && (a[1] - b[1]).abs() <= 1e-12).then_some(i)
        })
        .collect()
}

fn translated(model: &Model, delta: [f64; 3]) -> Model {
    crate::transform::affine(
        model,
        [
            [1., 0., 0., delta[0]],
            [0., 1., 0., delta[1]],
            [0., 0., 1., delta[2]],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap()
}

#[test]
fn suppress_single_fillet_on_cuboid_restores_sharp_corner() {
    let sharp = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
    let edges = vertical_edges(&sharp);
    let (rounded, _) = analytic_fillet(&sharp, edges[0], 1.).unwrap();
    let graph = graph_with_attrs(&rounded);
    let chains = find_fillet_chains(&rounded, &graph, &budget()).unwrap();
    assert_eq!(chains.len(), 1);
    let before = rounded.clone();

    let report = suppress_feature(
        &rounded,
        &FeatureRef::FilletChain(chains[0].clone()),
        &budget(),
    )
    .unwrap();
    assert_eq!(report.kind, "fillet-chain");
    assert_eq!(report.new_sharp_edges, 1);
    // Validity gate ran inside suppress_feature; result is one solid body.
    report.model.validate().unwrap();
    assert_eq!(report.model.bodies.len(), 1);
    // The sharp corner is back: 8 vertices, box volume 1000.
    assert!(
        (report.volume_after_mm3 - 1000.).abs() < 1e-3,
        "sharp box volume: {}",
        report.volume_after_mm3
    );
    // ΔV within the declared estimate: (1 − π/4) r² L, r = 1, L = 10.
    let expected = (1. - std::f64::consts::FRAC_PI_4) * 10.;
    assert!(
        (report.delta_volume_actual_mm3 - expected).abs() <= 0.05 * expected,
        "actual ΔV {} vs estimate {}",
        report.delta_volume_actual_mm3,
        report.delta_volume_estimate_mm3
    );
    assert!(
        (report.delta_volume_estimate_mm3 - expected).abs() <= 0.01 * expected,
        "declared estimate {} vs analytic {}",
        report.delta_volume_estimate_mm3,
        expected
    );
    // Every new vertex lands on a corner of the original box.
    for v in &report.model.vertices {
        let on_corner = (0..8).any(|c| {
            let corner = [
                if c & 1 == 0 { 0. } else { 10. },
                if c & 2 == 0 { 0. } else { 10. },
                if c & 4 == 0 { 0. } else { 10. },
            ];
            dist(v.point, corner) < 1e-6
        });
        assert!(on_corner, "vertex {:?} is not a box corner", v.point);
    }
    // Input untouched.
    assert_eq!(rounded, before);
}

#[test]
fn suppress_through_hole_in_block() {
    let block = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
    let drill = translated(&crate::cylinder(2., 14.).unwrap(), [5., 5., -2.]);
    let model = crate::boolean(&block, &drill, "difference").unwrap();
    let graph = graph_with_attrs(&model);
    let holes = find_holes(&model, &graph, &budget()).unwrap();
    assert_eq!(holes.len(), 1, "one bore: {holes:?}");
    let before = model.clone();

    let report = suppress_feature(&model, &FeatureRef::Hole(holes[0].clone()), &budget())
        .unwrap();
    assert_eq!(report.kind, "hole");
    assert_eq!(report.new_sharp_edges, 0);
    report.model.validate().unwrap();
    // The bore is gone: volume back to the full block.
    assert!(
        (report.volume_after_mm3 - 1000.).abs() < 1e-2,
        "volume after hole suppression: {}",
        report.volume_after_mm3
    );
    // ΔV = π r² h = π · 4 · 10 within the mass-properties tolerance.
    let expected = std::f64::consts::PI * 4. * 10.;
    assert!(
        (report.delta_volume_actual_mm3 - expected).abs() <= 0.02 * expected,
        "actual ΔV {} vs π·4·10",
        report.delta_volume_actual_mm3
    );
    assert!(
        (report.delta_volume_estimate_mm3 - expected).abs() <= 0.01 * expected,
        "estimate {} vs π·4·10",
        report.delta_volume_estimate_mm3
    );
    // No cylindrical face survives.
    let graph = graph_with_attrs(&report.model);
    let holes = find_holes(&report.model, &graph, &budget()).unwrap();
    assert!(holes.is_empty(), "hole is gone: {holes:?}");
    assert_eq!(model, before);
}

#[test]
fn suppress_closed_pocket_in_block() {
    // Canonical 867 fixture: floor at z=2 (36×26), walls z=2..20. The
    // boolean fragments the mouth region into coplanar shards whose outer
    // loops carry the rim edges — the merge path must re-sew them into
    // one face. All surroundings are planar.
    let model = crate::boolean(
        &crate::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
        &crate::cuboid([2., 2., 2.], [38., 28., 22.]).unwrap(),
        "difference",
    )
    .unwrap();
    let graph = graph_with_attrs(&model);
    let pockets = find_pockets(&model, &graph, &budget()).unwrap();
    assert_eq!(pockets.len(), 1, "{pockets:?}");
    assert_eq!(pockets[0].kind, PocketKind::Closed);
    let before = model.clone();

    let report =
        suppress_feature(&model, &FeatureRef::Pocket(pockets[0].clone()), &budget())
            .unwrap();
    assert_eq!(report.kind, "pocket");
    report.model.validate().unwrap();
    // Filled pocket → full block 40×30×20.
    let full = 40. * 30. * 20.;
    assert!(
        (report.volume_after_mm3 - full).abs() < 1e-6 * full,
        "volume after pocket suppression: {}",
        report.volume_after_mm3
    );
    // ΔV = floor 36×26 × depth 18.
    let expected = 36. * 26. * 18.;
    assert!(
        (report.delta_volume_actual_mm3 - expected).abs() < 1e-6 * expected,
        "actual ΔV {}",
        report.delta_volume_actual_mm3
    );
    assert!(
        (report.delta_volume_estimate_mm3 - expected).abs() < 1e-6 * expected,
        "estimate {}",
        report.delta_volume_estimate_mm3
    );
    // No pocket remains; the model is a plain block.
    let graph = graph_with_attrs(&report.model);
    assert!(find_pockets(&report.model, &graph, &budget()).unwrap().is_empty());
    assert_eq!(model, before);
}

#[test]
fn suppress_closed_pocket_in_cup() {
    // Round blind pocket (the bore of the cup doubles as a closed round
    // pocket, a documented matcher overlap): the mouth lives on the outer
    // loops of the coplanar annulus shards, so the merge path re-sews
    // them into one disk face; the curved bore wall is deleted outright.
    const CUP: [[f64; 2]; 6] = [
        [0., 0.],
        [5., 0.],
        [5., 6.],
        [2., 6.],
        [2., 2.],
        [0., 2.],
    ];
    let model = crate::analytic::revolve(&CUP).unwrap();
    let graph = graph_with_attrs(&model);
    let pockets = find_pockets(&model, &graph, &budget()).unwrap();
    assert_eq!(pockets.len(), 1, "one pocket: {pockets:?}");
    assert_eq!(pockets[0].kind, PocketKind::Closed);
    let before = model.clone();

    let report =
        suppress_feature(&model, &FeatureRef::Pocket(pockets[0].clone()), &budget())
            .unwrap();
    assert_eq!(report.kind, "pocket");
    report.model.validate().unwrap();
    // Filled pocket → solid cylinder r = 5, h = 6.
    let full = std::f64::consts::PI * 25. * 6.;
    assert!(
        (report.volume_after_mm3 - full).abs() < 1e-2 * full,
        "volume after pocket suppression: {}",
        report.volume_after_mm3
    );
    // ΔV = π r² depth = π · 4 · 4.
    let expected = std::f64::consts::PI * 4. * 4.;
    assert!(
        (report.delta_volume_actual_mm3 - expected).abs() <= 0.02 * expected,
        "actual ΔV {}",
        report.delta_volume_actual_mm3
    );
    assert!(
        (report.delta_volume_estimate_mm3 - expected).abs() <= 0.01 * expected,
        "estimate {}",
        report.delta_volume_estimate_mm3
    );
    assert_eq!(model, before);
}

#[test]
fn diverging_supports_fail_with_face_ids_and_rollback() {
    let sharp = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
    let edges = vertical_edges(&sharp);
    let (rounded, _) = analytic_fillet(&sharp, edges[0], 1.).unwrap();
    let graph = graph_with_attrs(&rounded);
    let chains = find_fillet_chains(&rounded, &graph, &budget()).unwrap();
    assert_eq!(chains.len(), 1);
    // Artificially diverging surroundings: claim the blend rolls on the
    // two parallel side walls (x = 0 and x = 10). Their extensions never
    // meet, so suppression must refuse and name the faces.
    let parallel_walls: Vec<usize> = (0..rounded.faces.len())
        .filter(|&f| {
            let plane = face_plane_fit(&rounded, f);
            matches!(plane, Ok(p) if p.normal[0].abs() > 0.99 && !chains[0].faces.contains(&f))
        })
        .collect();
    assert_eq!(parallel_walls.len(), 2);
    let mut bogus = chains[0].clone();
    bogus.supports = [parallel_walls[0], parallel_walls[1]];

    let before = rounded.clone();
    let error =
        suppress_feature(&rounded, &FeatureRef::FilletChain(bogus), &budget()).unwrap_err();
    assert_eq!(error.code, SUPPRESS_DIVERGENT, "{error:?}");
    assert!(
        error.message.contains(&parallel_walls[0].to_string())
            && error.message.contains(&parallel_walls[1].to_string()),
        "culpable faces named: {}",
        error.message
    );
    // Honest rollback: the model is bit-for-bit unchanged.
    assert_eq!(rounded, before);
    let graph = graph_with_attrs(&rounded);
    assert_eq!(
        find_fillet_chains(&rounded, &graph, &budget()).unwrap().len(),
        1,
        "feature still there after rollback"
    );
}

#[test]
fn open_pocket_is_refused_typed() {
    // Slot open to the body boundary: PocketKind::Open.
    let block = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
    let tool = crate::cuboid([0., 3., 6.], [7., 7., 12.]).unwrap();
    let model = crate::boolean(&block, &tool, "difference").unwrap();
    let graph = graph_with_attrs(&model);
    let pockets = find_pockets(&model, &graph, &budget()).unwrap();
    if let Some(pocket) = pockets.iter().find(|p| p.kind == PocketKind::Open) {
        let before = model.clone();
        let error =
            suppress_feature(&model, &FeatureRef::Pocket(pocket.clone()), &budget())
                .unwrap_err();
        assert_eq!(error.code, SUPPRESS_UNSUPPORTED, "{error:?}");
        assert_eq!(model, before);
    }
}

#[test]
fn out_of_range_feature_is_invalid() {
    let model = crate::cuboid([0., 0., 0.], [4., 4., 4.]).unwrap();
    let mut chain = FilletChain {
        faces: vec![usize::MAX],
        edges: vec![],
        kind: crate::aag_fillets::FilletKind::ConstantRadius,
        radius_min: 1.,
        radius_max: 1.,
        supports: [0, 1],
    };
    let before = model.clone();
    let error = suppress_feature(&model, &FeatureRef::FilletChain(chain.clone()), &budget())
        .unwrap_err();
    assert_eq!(error.code, SUPPRESS_INVALID, "{error:?}");
    chain.faces = vec![];
    let error = suppress_feature(&model, &FeatureRef::FilletChain(chain), &budget())
        .unwrap_err();
    assert_eq!(error.code, SUPPRESS_INVALID, "{error:?}");
    assert_eq!(model, before);
}

// ------------------------------------------------------------------
// move face (871)
// ------------------------------------------------------------------

/// Index of the unique face whose canonical plane sits at `axis` = value
/// with outward normal +1 along that axis (e.g. the x=10 wall of a box).
fn face_at(model: &Model, axis: usize, value: f64) -> usize {
    model
        .faces
        .iter()
        .enumerate()
        .find_map(|(i, f)| {
            let points: Vec<[f64; 3]> = loop_vertices(model, f.outer)
                .unwrap()
                .iter()
                .map(|&v| model.vertices[v].point)
                .collect();
            (points
                .iter()
                .all(|p| (p[axis] - value).abs() < 1e-9))
            .then_some(i)
        })
        .expect("face at the given plane exists")
}

/// Ids of all faces sitting on the plane `axis` = `value`.
fn face_ids_at(model: &Model, axis: usize, value: f64) -> Vec<crate::TopoId> {
    model
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            loop_vertices(model, f.outer)
                .unwrap()
                .iter()
                .all(|&v| (model.vertices[v].point[axis] - value).abs() < 1e-9)
        })
        .map(|(i, _)| model.1.faces[i])
        .collect()
}

#[test]
fn move_block_wall_extends_box() {
    let model = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
    let wall = face_at(&model, 0, 10.);
    let opposite_ids = face_ids_at(&model, 0, 0.);
    assert_eq!(opposite_ids.len(), 1);
    let before = model.clone();

    let report = move_face(&model, wall, [1., 0., 0.], 2., &budget()).unwrap();
    assert_eq!(report.moved_face, wall);
    assert!(report.driven_faces.is_empty(), "no coplanar shards on a box");
    assert_eq!(report.absorbing_faces.len(), 4, "four side walls bound the move");
    assert!(report.absorbed_faces.is_empty() && report.annihilated_faces.is_empty());
    report.model.validate().unwrap();
    assert!(
        (report.volume_after_mm3 - 1200.).abs() < 1e-6 * 1200.,
        "volume: {}",
        report.volume_after_mm3
    );
    // ΔV = 100 mm² × 2 mm, estimate and measurement agree to 1e-6 rel.
    assert!((report.delta_volume_estimate_mm3 - 200.).abs() < 1e-9);
    assert!(
        (report.delta_volume_actual_mm3 - 200.).abs() < 1e-6 * 200.,
        "actual ΔV {}",
        report.delta_volume_actual_mm3
    );
    // Persistent id of the untouched opposite face survives the rebuild.
    let after_ids = face_ids_at(&report.model, 0, 0.);
    assert_eq!(after_ids, opposite_ids, "untouched face id preserved");
    // Input untouched.
    assert_eq!(model, before);
}

#[test]
fn move_pocket_floor_deeper_and_shallower() {
    let pocket = || {
        crate::boolean(
            &crate::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
            &crate::cuboid([2., 2., 2.], [38., 28., 22.]).unwrap(),
            "difference",
        )
        .unwrap()
    };
    let model = pocket();
    let floor = face_at(&model, 2, 2.);
    let floor_area = 36. * 26.;
    let volume_before = 40. * 30. * 20. - floor_area * 18.;
    let before = model.clone();

    // Deeper: floor outward normal is +z; moving −z grows the void.
    let deeper = move_face(&model, floor, [0., 0., -1.], 1., &budget()).unwrap();
    deeper.model.validate().unwrap();
    assert!(
        (deeper.delta_volume_estimate_mm3 + floor_area).abs() < 1e-6,
        "estimate: {}",
        deeper.delta_volume_estimate_mm3
    );
    assert!(
        (deeper.volume_after_mm3 - (volume_before - floor_area)).abs()
            < 1e-6 * volume_before,
        "deeper volume: {}",
        deeper.volume_after_mm3
    );
    assert!(
        (deeper.delta_volume_actual_mm3 + floor_area).abs() < 1e-6 * floor_area,
        "deeper ΔV: {}",
        deeper.delta_volume_actual_mm3
    );

    // Shallower: +z by 1 mm gives the mirror delta.
    let shallower = move_face(&model, floor, [0., 0., 1.], 1., &budget()).unwrap();
    assert!(
        (shallower.volume_after_mm3 - (volume_before + floor_area)).abs()
            < 1e-6 * volume_before,
        "shallower volume: {}",
        shallower.volume_after_mm3
    );
    assert!(
        (shallower.delta_volume_actual_mm3 - floor_area).abs() < 1e-6 * floor_area,
        "shallower ΔV: {}",
        shallower.delta_volume_actual_mm3
    );
    // Untouched faces (block bottom z=0) keep their persistent ids.
    let bottom_before = face_ids_at(&model, 2, 0.);
    let bottom_after = face_ids_at(&shallower.model, 2, 0.);
    assert_eq!(bottom_before, bottom_after, "untouched shard ids preserved");
    assert_eq!(model, before);
}

#[test]
fn move_absorbs_thin_wall_between_pockets() {
    let model = crate::boolean(
        &crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap(),
        &crate::cuboid([2., 2., 4.], [4.75, 8., 12.]).unwrap(),
        "difference",
    )
    .unwrap();
    let model = crate::boolean(
        &model,
        &crate::cuboid([5.25, 2., 4.], [8., 8., 12.]).unwrap(),
        "difference",
    )
    .unwrap();
    let wall = face_at(&model, 0, 4.75);
    let volume_before = 1000. - 2. * (2.75 * 6. * 6.);
    let before = model.clone();

    // Outward normal of the pocket-1 wall points −x; pushing +x by the
    // full 0.5 mm wall thickness consumes the wall: the strip above it is
    // absorbed and the moved face annihilates with the far side x=5.25.
    let report = move_face(&model, wall, [1., 0., 0.], 0.5, &budget()).unwrap();
    report.model.validate().unwrap();
    assert!(!report.absorbed_faces.is_empty(), "top strip absorbed: {report:?}");
    assert!(
        report.annihilated_faces.contains(&wall),
        "moved face annihilated with the far wall side: {:?}",
        report.annihilated_faces
    );
    // Merged pocket 6×6×6: volume and ΔV are analytic.
    let volume_after = 1000. - 6. * 6. * 6.;
    assert!(
        (report.volume_after_mm3 - volume_after).abs() < 1e-6 * volume_before,
        "volume after absorption: {} (want {volume_after})",
        report.volume_after_mm3
    );
    assert!(
        (report.delta_volume_actual_mm3 - (-0.5 * 36.)).abs() < 1e-6 * 18.,
        "actual ΔV {}",
        report.delta_volume_actual_mm3
    );
    assert!(
        (report.delta_volume_estimate_mm3 - (-0.5 * 36.)).abs() < 1e-9,
        "estimate {}",
        report.delta_volume_estimate_mm3
    );
    // No zombie faces at the consumed wall planes.
    assert!(
        face_ids_at(&report.model, 0, 4.75).is_empty()
            && face_ids_at(&report.model, 0, 5.25).is_empty(),
        "no faces left at x=4.75 / x=5.25"
    );
    // The two pockets merged into one.
    let graph = graph_with_attrs(&report.model);
    let pockets = find_pockets(&report.model, &graph, &budget()).unwrap();
    assert_eq!(pockets.len(), 1, "merged pocket: {pockets:?}");
    assert_eq!(model, before);
}

#[test]
fn move_top_plane_carries_coplanar_shards() {
    // The boolean fragments the block's top into coplanar shards; moving
    // one shard must drive the whole coplanar component.
    let model = crate::boolean(
        &crate::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
        &crate::cuboid([2., 2., 2.], [38., 28., 22.]).unwrap(),
        "difference",
    )
    .unwrap();
    let top_shards = face_ids_at(&model, 2, 20.);
    assert!(top_shards.len() > 1, "fragmented top: {}", top_shards.len());
    let shard = face_at(&model, 2, 20.);
    let before = model.clone();

    let report = move_face(&model, shard, [0., 0., -1.], 1., &budget()).unwrap();
    assert_eq!(
        report.driven_faces.len(),
        top_shards.len() - 1,
        "every other coplanar shard is driven: {:?}",
        report.driven_faces
    );
    report.model.validate().unwrap();
    // The whole rim frame (40×30 minus the 36×26 mouth) went down 1 mm;
    // the pocket walls and floor are unchanged.
    let expected = -(40. * 30. - 36. * 26.);
    assert!(
        (report.delta_volume_estimate_mm3 - expected).abs() < 1e-6,
        "estimate {}",
        report.delta_volume_estimate_mm3
    );
    assert!(
        (report.delta_volume_actual_mm3 - expected).abs() < 1e-6 * expected.abs(),
        "actual ΔV {}",
        report.delta_volume_actual_mm3
    );
    // Driven shards kept their identity? No — they moved. The untouched
    // bottom shards keep theirs.
    let bottom_before = face_ids_at(&model, 2, 0.);
    let bottom_after = face_ids_at(&report.model, 2, 0.);
    assert_eq!(bottom_before, bottom_after);
    assert_eq!(model, before);
}

#[test]
fn move_curved_face_is_refused_typed() {
    let model = crate::cylinder(2., 5.).unwrap();
    let side = model
        .faces
        .iter()
        .enumerate()
        .find_map(|(i, f)| {
            (f.surface.degree_u > 1 || f.surface.degree_v > 1).then_some(i)
        })
        .expect("cylinder side");
    let before = model.clone();
    let error = move_face(&model, side, [1., 0., 0.], 1., &budget()).unwrap_err();
    assert_eq!(error.code, MOVE_UNSUPPORTED, "{error:?}");
    assert!(error.message.contains(&side.to_string()));
    assert_eq!(model, before);
}

#[test]
fn move_off_normal_is_refused_typed() {
    let model = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
    let wall = face_at(&model, 0, 10.);
    let before = model.clone();
    let s = 0.5 * std::f64::consts::SQRT_2;
    let error = move_face(&model, wall, [s, 0., s], 1., &budget()).unwrap_err();
    assert_eq!(error.code, MOVE_UNSUPPORTED, "{error:?}");
    assert_eq!(model, before);
}

#[test]
fn move_through_parallel_face_is_refused_typed() {
    let model = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
    let wall = face_at(&model, 0, 10.);
    let before = model.clone();
    // Crossing the opposite wall (x=0) overshoots the absorbing limit.
    let error = move_face(&model, wall, [-1., 0., 0.], 12., &budget()).unwrap_err();
    assert_eq!(error.code, MOVE_UNSUPPORTED, "{error:?}");
    assert_eq!(model, before);
    // And the full-body consumption (landing exactly on x=0) fails too.
    let error = move_face(&model, wall, [-1., 0., 0.], 10., &budget()).unwrap_err();
    assert_eq!(model, before);
    let _ = error;
}

#[test]
fn move_invalid_input_is_refused() {
    let model = crate::cuboid([0., 0., 0.], [4., 4., 4.]).unwrap();
    let before = model.clone();
    assert_eq!(
        move_face(&model, usize::MAX, [1., 0., 0.], 1., &budget())
            .unwrap_err()
            .code,
        MOVE_INVALID
    );
    assert_eq!(
        move_face(&model, 0, [0., 0., 0.], 1., &budget())
            .unwrap_err()
            .code,
        MOVE_INVALID
    );
    assert_eq!(
        move_face(&model, 0, [1., 0., 0.], f64::NAN, &budget())
            .unwrap_err()
            .code,
        MOVE_INVALID
    );
    assert_eq!(model, before);
}

// ------------------------------------------------------------------
// offset face (872)
// ------------------------------------------------------------------

/// Radial distance of every surface sample of `faces` to `axis`;
/// returns (min, max) over a small grid — a wall-thickness probe.
fn radial_samples(model: &Model, faces: &[usize], axis: &Axis) -> (f64, f64) {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for &f in faces {
        let sampler = nurbs_core::surface::SurfaceSampler::new(&model.faces[f].surface).unwrap();
        for i in 0..=4 {
            for j in 0..=4 {
                let p = sampler
                    .evaluate(i as f64 / 4., j as f64 / 4.)
                    .unwrap()
                    .point;
                let r = radial_distance(axis, [p[0], p[1], p[2]]);
                lo = lo.min(r);
                hi = hi.max(r);
            }
        }
    }
    (lo, hi)
}

fn tube_wall_faces(model: &Model) -> (Vec<usize>, Vec<usize>, Axis) {
    let mut aag = Aag::build(model, &budget()).unwrap();
    aag.attach_face_attrs(model, &budget()).unwrap();
    let mut walls: Vec<(usize, f64)> = vec![];
    let mut axis = None;
    for (i, node) in aag.nodes.iter().enumerate() {
        let a = node.attrs.as_ref().unwrap();
        if a.class == SurfaceClass::Cylinder {
            axis = a.axis;
            walls.push((i, a.radius.unwrap()));
        }
    }
    // Inner wall = the smaller of the two fitted radii (radius-agnostic,
    // so the helper works before AND after the offset).
    let mut radii: Vec<f64> = walls.iter().map(|(_, r)| *r).collect();
    radii.sort_by(f64::total_cmp);
    radii.dedup_by(|b, a| (*a - *b).abs() < 1e-3);
    assert_eq!(radii.len(), 2, "a tube has two coaxial wall radii");
    let (inner_r, outer_r) = (radii[0], radii[1]);
    let inner = walls
        .iter()
        .filter(|(_, r)| (*r - inner_r).abs() < 1e-3)
        .map(|(i, _)| *i)
        .collect();
    let outer = walls
        .iter()
        .filter(|(_, r)| (*r - outer_r).abs() < 1e-3)
        .map(|(i, _)| *i)
        .collect();
    (inner, outer, axis.expect("tube walls have an axis"))
}
#[test]
fn offset_tube_wall_auto_expands_to_concentric_pair() {
    let model = crate::analytic::tube(5., 2., 6.).unwrap();
    let (inner, outer, axis) = tube_wall_faces(&model);
    assert!(!inner.is_empty() && !outer.is_empty());
    let before = model.clone();

    let report = offset_face(&model, inner[0], 0.5, OffsetOptions::default(), &budget())
        .unwrap();
    assert_eq!(report.kind, "cylinder-radius");
    assert!(report.auto_expanded, "pair auto-expanded");
    // Both walls moved: sibling patches of both cylinders.
    for &f in inner.iter().chain(&outer) {
        assert!(report.offset_faces.contains(&f), "face {f} offset");
    }
    assert_eq!(report.old_radii.len(), 2);
    assert!((report.old_radii[0] - 2.).abs() < 1e-3 && (report.old_radii[1] - 5.).abs() < 1e-3);
    assert!((report.new_radii[0] - 2.5).abs() < 1e-9 && (report.new_radii[1] - 5.5).abs() < 1e-9);
    // Wall thickness preserved exactly (pair offset by the same delta).
    let tb = report.wall_thickness_before_mm.unwrap();
    let ta = report.wall_thickness_after_mm.unwrap();
    assert!((tb - 3.).abs() < 1e-3 && (ta - tb).abs() < 1e-9, "{tb} -> {ta}");
    report.model.validate().unwrap();

    // Acceptance probe: wall thickness in a sample grid, 1e-9 · scale.
    let scale = 6.;
    let (inner_after, outer_after, axis_after) = tube_wall_faces(&report.model);
    let (ilo, ihi) = radial_samples(&report.model, &inner_after, &axis_after);
    let (olo, ohi) = radial_samples(&report.model, &outer_after, &axis_after);
    assert!((ilo - 2.5).abs() < 1e-9 * scale && (ihi - 2.5).abs() < 1e-9 * scale,
        "inner radius samples: {ilo}..{ihi}");
    assert!((olo - 5.5).abs() < 1e-9 * scale && (ohi - 5.5).abs() < 1e-9 * scale,
        "outer radius samples: {olo}..{ohi}");
    let thickness = olo - ihi;
    assert!((thickness - 3.).abs() < 1e-9 * scale, "thickness {thickness}");
    let _ = axis;

    // ΔV = π h [(5.5²−5²) − (2.5²−2²)] = π·6·(5.25−2.25) = 18π.
    let expected = std::f64::consts::PI * 6. * (5.25 - 2.25);
    assert!(
        (report.delta_volume_estimate_mm3 - expected).abs() < 1e-6 * expected,
        "estimate {} vs {expected}",
        report.delta_volume_estimate_mm3
    );
    assert!(
        (report.delta_volume_actual_mm3 - expected).abs() < 2e-3 * expected,
        "actual ΔV {} vs {expected}",
        report.delta_volume_actual_mm3
    );
    // Geometry-only mutation: persistent ids untouched everywhere.
    assert_eq!(report.model.1.faces, before.1.faces);
    assert_eq!(report.model.1.edges, before.1.edges);
    assert_eq!(report.model.1.vertices, before.1.vertices);
    assert_eq!(model, before);
}

#[test]
fn offset_single_member_of_pair_strict_refuses_with_diagnosis() {
    let model = crate::analytic::tube(5., 2., 6.).unwrap();
    let (inner, outer, _) = tube_wall_faces(&model);
    let before = model.clone();
    let error = offset_face(
        &model,
        inner[0],
        0.5,
        OffsetOptions { strict: true },
        &budget(),
    )
    .unwrap_err();
    assert_eq!(error.code, OFFSET_UNSUPPORTED, "{error:?}");
    assert!(
        outer.iter().any(|f| error.message.contains(&f.to_string())),
        "partner faces named: {}",
        error.message
    );
    assert_eq!(model, before);
}

#[test]
fn offset_solid_cylinder_changes_radius() {
    let model = crate::cylinder(2., 5.).unwrap();
    let (side,) = {
        let mut aag = Aag::build(&model, &budget()).unwrap();
        aag.attach_face_attrs(&model, &budget()).unwrap();
        let side: Vec<usize> = aag
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| {
                n.attrs.as_ref().unwrap().class == SurfaceClass::Cylinder
            })
            .map(|(i, _)| i)
            .collect();
        assert!(!side.is_empty());
        (side,)
    };
    let before = model.clone();
    let report = offset_face(&model, side[0], 1., OffsetOptions::default(), &budget())
        .unwrap();
    assert!(!report.auto_expanded, "no concentric partner on a solid");
    assert_eq!(report.old_radii.len(), 1);
    assert!((report.new_radii[0] - 3.).abs() < 1e-9);
    report.model.validate().unwrap();
    // ΔV = π·5·(9−4) = 25π.
    let expected = std::f64::consts::PI * 5. * (9. - 4.);
    assert!(
        (report.delta_volume_actual_mm3 - expected).abs() < 2e-3 * expected,
        "actual ΔV {} vs {expected}",
        report.delta_volume_actual_mm3
    );
    assert!(
        (report.delta_volume_estimate_mm3 - expected).abs() < 1e-6 * expected,
        "estimate {}",
        report.delta_volume_estimate_mm3
    );
    assert_eq!(model, before);
}

#[test]
fn offset_cylinder_next_to_fillet_is_refused_typed() {
    // Rounded cuboid: the fillet cylinder rolls tangentially on two
    // planar walls; changing its radius would tear the tangency.
    let sharp = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
    let edges: Vec<usize> = sharp
        .edges
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let a = sharp.vertices[e.vertices[0]].point;
            let b = sharp.vertices[e.vertices[1]].point;
            ((a[0] - b[0]).abs() <= 1e-12 && (a[1] - b[1]).abs() <= 1e-12).then_some(i)
        })
        .collect();
    let (rounded, _) = crate::analytic_features::analytic_fillet(&sharp, edges[0], 1.).unwrap();
    let mut aag = Aag::build(&rounded, &budget()).unwrap();
    aag.attach_face_attrs(&rounded, &budget()).unwrap();
    let fillet = aag
        .nodes
        .iter()
        .enumerate()
        .find_map(|(i, n)| {
            (n.attrs.as_ref().unwrap().class == SurfaceClass::Cylinder).then_some(i)
        })
        .expect("fillet cylinder");
    let before = rounded.clone();
    let error = offset_face(&rounded, fillet, 0.5, OffsetOptions::default(), &budget())
        .unwrap_err();
    assert_eq!(error.code, OFFSET_UNSUPPORTED, "{error:?}");
    assert!(
        error.message.contains("tangent") || error.message.contains("blend"),
        "blend diagnosis: {}",
        error.message
    );
    assert_eq!(rounded, before);
}

#[test]
fn offset_planar_face_delegates_to_move() {
    let model = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
    let wall = face_at(&model, 0, 10.);
    let before = model.clone();
    let report = offset_face(&model, wall, 2., OffsetOptions::default(), &budget()).unwrap();
    assert_eq!(report.kind, "planar");
    assert!((report.volume_after_mm3 - 1200.).abs() < 1e-6 * 1200.);
    assert!(
        (report.delta_volume_actual_mm3 - 200.).abs() < 1e-6 * 200.,
        "actual ΔV {}",
        report.delta_volume_actual_mm3
    );
    assert_eq!(model, before);
}

#[test]
fn offset_unsupported_classes_are_refused_typed() {
    // Torus and cone faces are outside offset-face/1.
    let torus = crate::torus(4., 1.).unwrap();
    let before = torus.clone();
    let face = torus
        .faces
        .iter()
        .enumerate()
        .find_map(|(i, f)| {
            (f.surface.degree_u > 1 || f.surface.degree_v > 1).then_some(i)
        })
        .expect("torus side");
    let error = offset_face(&torus, face, 0.5, OffsetOptions::default(), &budget())
        .unwrap_err();
    assert_eq!(error.code, OFFSET_UNSUPPORTED, "{error:?}");
    assert_eq!(torus, before);

    let cone = crate::analytic::frustum(3., 1.5, 4.).unwrap();
    let before = cone.clone();
    let error = offset_face(&cone, 0, 0.5, OffsetOptions::default(), &budget()).unwrap_err();
    assert!(
        error.code == OFFSET_UNSUPPORTED || error.code == OFFSET_INVALID,
        "{error:?}"
    );
    assert_eq!(cone, before);

    // Offset consuming the bore is a topology mutation: refused.
    let model = crate::analytic::tube(5., 2., 6.).unwrap();
    let (inner, _, _) = tube_wall_faces(&model);
    let before = model.clone();
    let error = offset_face(&model, inner[0], -2.5, OffsetOptions::default(), &budget())
        .unwrap_err();
    assert_eq!(error.code, OFFSET_UNSUPPORTED, "{error:?}");
    assert_eq!(model, before);
}

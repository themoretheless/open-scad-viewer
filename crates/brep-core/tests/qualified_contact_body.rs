#[path = "../examples/support/nurbs_contact_stock.rs"]
mod fixture;
use brep_core::{Model, region_replacement as r};

// Recreate the fixture's four boundary vertices through fresh original-edge
// roots, rather than accepting its pre-authored vertex coordinates as proof.
fn root_prepared(f: &fixture::Fixture) -> Model {
    let mut model = f.source.clone();
    for vertex in &f.prepared.vertices[f.source.vertices.len()..] {
        let p = vertex.point;
        let (edge, axis) = f
            .source
            .edges
            .iter()
            .enumerate()
            .find_map(|(i, e)| {
                if e.curve.degree != 1 {
                    return None;
                }
                let [a, b] = e.vertices.map(|v| f.source.vertices[v].point);
                let axis = (0..3).find(|&k| a[k] != b[k])?;
                ((0..3).all(|k| k == axis || (a[k] == p[k] && b[k] == p[k]))
                    && p[axis] > a[axis].min(b[axis])
                    && p[axis] < a[axis].max(b[axis]))
                .then_some((i, axis))
            })
            .unwrap();
        let mut lo = [-100.; 3];
        let mut hi = [100.; 3];
        lo[axis] = p[axis];
        hi[axis] = p[axis] + 1.;
        let cutter = brep_core::cuboid(lo, hi).unwrap();
        let face = cutter
            .faces
            .iter()
            .position(|f| {
                f.surface
                    .control_points
                    .iter()
                    .flatten()
                    .all(|q| q[axis] == p[axis])
            })
            .unwrap();
        let result = brep_core::edge_partition::partition_at_face(
            &model, edge, &cutter, face, 0, 1e-8, 100_000,
        )
        .unwrap();
        assert_eq!(result.result.partition.vertex, model.vertices.len());
        model = result.result.partition.model;
    }
    model
}
fn cuts(f: &fixture::Fixture) -> Vec<r::ContactCut<'_>> {
    f.cuts
        .iter()
        .map(|c| r::ContactCut {
            face: c.face,
            endpoints: c.endpoints,
            segments: &c.segments,
            replacement_pcurves: &c.replacement,
            remove: c.anchor,
            smooth: c.smooth,
        })
        .collect()
}
fn graph(f: &fixture::Fixture) -> r::CentralGraph {
    r::CentralGraph {
        surface: f.blend.clone(),
        pcurves: f.cuts.iter().flat_map(|c| c.replacement.clone()).collect(),
        // Even forged constructor success is never an admission certificate.
        sampled_jet_residual: [0.; 6],
    }
}
fn graph_cuts(f: &fixture::Fixture) -> Vec<r::GraphContactCut<'_>> {
    f.cuts
        .iter()
        .map(|c| r::GraphContactCut {
            face: c.face,
            endpoints: c.endpoints,
            segments: &c.segments,
            remove: c.anchor,
            smooth: c.smooth,
        })
        .collect()
}
#[test]
fn graph_admission_rechecks_saved_body_and_refuses_incomplete_boundary_or_work() {
    let f = fixture::build();
    let source = root_prepared(&f);
    let before = source.clone();
    let mut graph = graph(&f);
    // Missing constructor evidence must not veto an independently proved body.
    graph.sampled_jet_residual = [f64::NAN; 6];
    let cuts = graph_cuts(&f);
    let limits = r::ContactQualificationLimits::default();
    let body =
        r::trim_and_replace_graph_contacts_g2(&source, &cuts, &graph, true, 1e-8, limits).unwrap();
    assert_eq!(body.model().faces.len(), 7);
    assert_eq!(body.seams().len(), 2);
    assert_eq!(body.fields().len(), body.model().faces.len());
    assert!(
        body.fields()
            .iter()
            .all(|p| p.absolute_curvature_upper.is_some_and(|x| x <= 10.))
    );
    assert_eq!(
        body.contacts().replacement.closure.closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    assert!(
        r::trim_and_replace_graph_contacts_g2(
            &source,
            &cuts,
            &graph,
            true,
            1e-8,
            r::ContactQualificationLimits { cells: 8, ..limits },
        )
        .is_err()
    );
    // Preserved supports and identities survive the atomic operation and reload.
    for (id, kept) in &body.contacts().retained_supports {
        let original = source.1.faces.iter().position(|x| x == id).unwrap();
        assert_eq!(
            body.model().faces[*kept].surface,
            source.faces[original].surface
        );
    }
    #[cfg(feature = "codec")]
    {
        let loaded: Model =
            value_codec::from_str(&value_codec::to_string(body.model()).unwrap()).unwrap();
        assert_eq!(&loaded, body.model());
        assert_eq!(
            brep_core::watertight::inspect_with_exact(&loaded, 100_000, 1_000_000)
                .unwrap()
                .closure
                .status,
            brep_core::watertight::Closure::WithinTolerance
        );
    }
    let good = graph.pcurves[0].clone();
    graph.pcurves[0] = graph.pcurves[0].reverse().unwrap();
    assert!(
        r::trim_and_replace_graph_contacts_g2(&source, &cuts, &graph, true, 1e-8, limits,).is_err()
    );
    graph.pcurves[0] = good;
    graph.pcurves.pop();
    assert!(
        r::trim_and_replace_graph_contacts_g2(&source, &cuts, &graph, true, 1e-8, limits,).is_err()
    );
    assert_eq!(source, before);
}
#[test]
fn original_roots_trim_supports_sew_and_admit_actual_g2_body_under_ten() {
    let f = fixture::build();
    let source = root_prepared(&f);
    let before = source.clone();
    let limits = r::ContactQualificationLimits::default();
    let body =
        r::trim_and_replace_contacts_g2(&source, &cuts(&f), &f.blend, true, 1e-8, limits).unwrap();
    assert_eq!(source, before);
    assert_eq!(
        (
            body.model().faces.len(),
            body.model().edges.len(),
            body.model().vertices.len()
        ),
        (7, 15, 10)
    );
    assert_eq!(body.seams().len(), 2);
    assert_eq!(body.fields().len(), body.model().faces.len());
    for report in body.fields() {
        assert!(report.absolute_curvature_upper.unwrap() <= 10.);
    }
    for face in &f.source.faces {
        assert!(
            body.model()
                .faces
                .iter()
                .any(|saved| saved.surface == face.surface)
        );
    }
    assert_eq!(
        body.contacts().replacement.closure.closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    let (cells, exact, domain, shape) = body.work();
    assert!(
        cells <= limits.cells
            && exact <= limits.exact_work
            && domain <= limits.domain_cells
            && shape <= limits.shape_cells
    );
    eprintln!(
        "qualified_body faces={} seams={} curvature_upper={:?} work={:?}",
        body.model().faces.len(),
        body.seams().len(),
        body.fields()
            .iter()
            .map(|p| p.absolute_curvature_upper.unwrap())
            .fold(0., f64::max),
        body.work()
    );
}
#[test]
fn missing_smooth_scope_sharp_cap_and_exhausted_work_cannot_publish_a_body() {
    let f = fixture::build();
    let before = f.prepared.clone();
    let mut c = cuts(&f);
    let limits = r::ContactQualificationLimits::default();
    assert!(
        r::trim_and_replace_contacts_g2(
            &f.prepared,
            &c,
            &f.blend,
            true,
            1e-8,
            r::ContactQualificationLimits { cells: 8, ..limits }
        )
        .is_err()
    );
    c[0].smooth = true; // The end-cap seam is intentionally sharp, not G2.
    assert!(
        r::trim_and_replace_contacts_g2(&f.prepared, &c, &f.blend, true, 1e-8, limits).is_err()
    );
    for cut in &mut c {
        cut.smooth = false;
    }
    assert!(
        r::trim_and_replace_contacts_g2(&f.prepared, &c, &f.blend, true, 1e-8, limits).is_err()
    );
    assert_eq!(f.prepared, before);
}

#[test]
fn closed_smooth_but_overcurved_scaled_body_is_refused() {
    let mut f = fixture::build();
    // Exact power-of-two scale preserves incidence and G2, but increases
    // principal curvature by 1024. Closure alone must not authorize it.
    let scale = |p: &mut Vec<f64>| {
        for x in p {
            *x /= 1024.;
        }
    };
    for v in &mut f.prepared.vertices {
        for x in &mut v.point {
            *x /= 1024.;
        }
    }
    for e in &mut f.prepared.edges {
        for p in &mut e.curve.control_points {
            scale(p);
        }
    }
    for face in &mut f.prepared.faces {
        for p in face.surface.control_points.iter_mut().flatten() {
            scale(p);
        }
    }
    for p in f.blend.control_points.iter_mut().flatten() {
        scale(p);
    }
    for c in &mut f.cuts {
        for segment in &mut c.segments {
            for p in &mut segment.curve.control_points {
                scale(p);
            }
        }
    }
    f.prepared.rebuild_topology_ids();
    let c = cuts(&f);
    let before = f.prepared.clone();
    let candidate =
        r::trim_and_replace_contacts(&f.prepared, &c, &f.blend, true, 1e-8, 100_000, 1_000_000)
            .unwrap();
    let proof = brep_core::face_curvature::inspect(
        &candidate.replacement.model,
        candidate.replacement.face,
        10.,
        1e-8,
        brep_core::face_curvature::Limits {
            cells: 50_000,
            domain_cells: 1_000_000,
            shape_cells: 1_000_000,
        },
        &[[0.5, 0.5]],
    )
    .unwrap();
    assert_eq!(proof.decision, brep_core::face_curvature::Decision::Exceeds);
    assert!(
        r::trim_and_replace_contacts_g2(
            &f.prepared,
            &c,
            &f.blend,
            true,
            1e-8,
            r::ContactQualificationLimits::default()
        )
        .is_err()
    );
    // A graph with forged zero residuals still cannot publish this closed,
    // G2-compatible but overcurved candidate.
    assert!(
        r::trim_and_replace_graph_contacts_g2(
            &f.prepared,
            &graph_cuts(&f),
            &graph(&f),
            true,
            1e-8,
            r::ContactQualificationLimits::default(),
        )
        .is_err()
    );
    assert_eq!(f.prepared, before);
}

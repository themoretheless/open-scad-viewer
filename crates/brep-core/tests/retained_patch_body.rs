#[path = "../examples/support/retained_patch_fixture.rs"]
mod support;
use brep_core::{Model, face_cut, region_replacement as r};
use support::{fan, source};

#[test]
fn original_face_trim_and_three_patch_fan_are_one_qualified_transaction() {
    let (m, face, cut, seeds) = source();
    let before = m.clone();
    let cuts = [cut];
    let requests = [face_cut::FaceRetention {
        face,
        cuts: &cuts,
        keep_uv: &seeds,
    }];
    let limits = r::ContactQualificationLimits::default();
    let out =
        r::trim_and_sew_g2_patch_set(&m, &requests, "trim-three-g2-patches", 1e-8, limits, fan)
            .unwrap();
    assert_eq!(m, before);
    assert_eq!(out.patch_faces().len(), 3);
    assert_eq!(out.source_faces()[face].len(), 1);
    let kept = out.source_faces()[face][0];
    assert_eq!(out.model().faces[kept].surface, m.faces[face].surface);
    for f in 0..m.faces.len() {
        for &mapped in &out.source_faces()[f] {
            assert_eq!(out.model().faces[mapped].surface, m.faces[f].surface);
            if f != face {
                assert_eq!(out.model().1.faces[mapped], m.1.faces[f]);
            }
        }
    }
    assert_eq!(out.contact_edges().len(), 1);
    assert!(out.contact_edges()[0].iter().all(Option::is_some));
    for (i, mapped) in out.source_edges().iter().enumerate() {
        assert_eq!(out.model().1.edges[mapped.unwrap()], m.1.edges[i]);
    }
    assert_eq!(out.source_vertices().len(), m.vertices.len());
    assert!(out.seams().complete && out.seams().within_tolerance);
    assert!(
        out.fields()
            .iter()
            .all(|p| p.absolute_curvature_upper.is_some_and(|v| v <= 10.))
    );
    assert_eq!(
        out.closure().closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    let (c, e, d, s) = out.work();
    assert!(
        c <= limits.cells
            && e <= limits.exact_work
            && d <= limits.domain_cells
            && s <= limits.shape_cells
    );
    #[cfg(feature = "codec")]
    {
        let loaded: Model =
            value_codec::from_str(&value_codec::to_string(out.model()).unwrap()).unwrap();
        assert_eq!(&loaded, out.model());
        assert_eq!(
            brep_core::watertight::inspect_with_exact(&loaded, 100_000, 1_000_000)
                .unwrap()
                .closure
                .status,
            brep_core::watertight::Closure::WithinTolerance
        );
    }
    eprintln!(
        "retained_patch_body F={} E={} V={} seams={} upper={} work={:?}",
        out.model().faces.len(),
        out.model().edges.len(),
        out.model().vertices.len(),
        out.seams().seams.len(),
        out.fields()
            .iter()
            .filter_map(|p| p.absolute_curvature_upper)
            .fold(0., f64::max),
        out.work()
    );
}
#[test]
fn incomplete_transition_or_exhausted_planning_cannot_mutate_original() {
    let (m, face, cut, seeds) = source();
    let before = m.clone();
    let cuts = [cut];
    let requests = [face_cut::FaceRetention {
        face,
        cuts: &cuts,
        keep_uv: &seeds,
    }];
    let limits = r::ContactQualificationLimits::default();
    assert!(
        r::trim_and_sew_g2_patch_set(&m, &requests, "no-material-leak", 1e-8, limits, |p| {
            let mut net = fan(p)?;
            net.patches.pop();
            Ok(net)
        })
        .is_err()
    );
    assert!(
        r::trim_and_sew_g2_patch_set(&m, &requests, "bad-g2", 1e-8, limits, |p| {
            let mut net = fan(p)?;
            net.patches[0].surface.control_points[1][1][2] += 0.1;
            Ok(net)
        })
        .is_err()
    );
    assert!(
        r::trim_and_sew_g2_patch_set(
            &m,
            &requests,
            "exhausted",
            1e-8,
            r::ContactQualificationLimits {
                domain_cells: 1,
                ..limits
            },
            |_| panic!("No construction after failed planning"),
        )
        .is_err()
    );
    assert_eq!(m, before);
}

#[test]
fn trimmed_support_sews_a_five_valent_shared_node_without_welding() {
    let (m, face, cut, seeds) = source();
    let boundary = m.loops[m.faces[face].outer].coedges[2].edge;
    let d = m.edges[boundary].curve.domain();
    let split = brep_core::feature_ops::edge_partition::split(
        &m,
        boundary,
        &[d[0] + (d[1] - d[0]) * 0.25, d[0] + (d[1] - d[0]) * 0.75],
        "five-node-source-rim",
        Default::default(),
    )
    .unwrap();
    let source = split.model();
    let before = source.clone();
    let cuts = [cut];
    let requests = [face_cut::FaceRetention {
        face,
        cuts: &cuts,
        keep_uv: &seeds,
    }];
    let out = r::trim_and_sew_g2_patch_set(
        source,
        &requests,
        "trim-five-node",
        1e-8,
        Default::default(),
        fan,
    )
    .unwrap();
    assert_eq!(source, &before);
    assert_eq!(out.patch_faces().len(), 5);
    let center = out.model().vertices.len() - 1;
    assert_eq!(
        out.model()
            .edges
            .iter()
            .filter(|e| e.vertices.contains(&center))
            .count(),
        5
    );
    assert!(out.seams().complete && out.seams().within_tolerance);
    assert_eq!(out.source_faces()[face].len(), 1);
    assert!(
        out.fields()
            .iter()
            .all(|p| p.absolute_curvature_upper.is_some_and(|v| v <= 10.))
    );
    eprintln!(
        "five_valent_trim F={} E={} V={} seams={} work={:?}",
        out.model().faces.len(),
        out.model().edges.len(),
        out.model().vertices.len(),
        out.seams().seams.len(),
        out.work()
    );
}

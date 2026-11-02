#[path = "../examples/support/retained_patch_fixture.rs"]
mod fixture;
use brep_core::{boundary_embedding as e, face_cut, region_replacement as r};
fn audit(model: &brep_core::Model) -> e::Report {
    e::inspect_with_linear(
        model,
        1e-8,
        e::Limits {
            exact_work: 1_000_000,
            trim_pairs: 10_000,
            trim_cells: 100_000,
            trim_domain_cells: 1_000_000,
            spans: 10_000,
            contacts: brep_core::face_contacts::Limits {
                pairs: 10_000,
                cells: 1_000_000,
                domain_cells: 1_000_000,
                cells_per_pair: 1000,
                domain_cells_per_pair: 10_000,
            },
        },
        100_000,
    )
    .unwrap()
}
#[test]
fn global_audit_classifies_all_six_common_carrier_pairs_and_charges_work() {
    let (m, face, cut, seeds) = fixture::source();
    let before = m.clone();
    let cuts = [cut];
    let requests = [face_cut::FaceRetention {
        face,
        cuts: &cuts,
        keep_uv: &seeds,
    }];
    let body = r::trim_and_sew_g2_patch_set(
        &m,
        &requests,
        "same-carrier-audit",
        1e-8,
        Default::default(),
        fixture::fan,
    )
    .unwrap();
    let report = audit(body.model());
    assert!(report.proven);
    assert!(
        report.agreement.all_equal && report.agreement.all_joins_exact && report.trim.all_valid
    );
    let certificates = report
        .intersections
        .pairs
        .pairs
        .iter()
        .filter_map(|p| match &p.boundary {
            Some(brep_core::face_contacts::SharedBoundary::SameSupportPartition(c)) => {
                assert_eq!(c.faces, p.faces);
                assert!(c.carrier.certified);
                assert!(c.cells > 0 && c.cells <= p.source_chart_cells);
                assert!(c.edge().is_some() != c.vertex().is_some());
                assert!(p.boundary_cells + p.source_chart_cells + p.hull_cells <= 1000);
                Some(c)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(certificates.len(), 6);
    assert!(certificates.iter().any(|c| c.edge().is_some()));
    assert!(certificates.iter().any(|c| c.vertex().is_some()));
    assert_eq!(m, before);
    assert!(report.intersections.pairs.cells <= 1_000_000);
    assert!(report.intersections.pairs.next_pair.is_none());
}

#[test]
fn five_valent_trimmed_body_has_no_unproven_global_pairs_and_survives_reload() {
    let (m, face, cut, seeds) = fixture::source();
    let edge = m.loops[m.faces[face].outer].coedges[2].edge;
    let d = m.edges[edge].curve.domain();
    let split = brep_core::feature_ops::edge_partition::split(
        &m,
        edge,
        &[d[0] + (d[1] - d[0]) * 0.25, d[0] + (d[1] - d[0]) * 0.75],
        "projected-source-rim",
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
    let body = r::trim_and_sew_g2_patch_set(
        source,
        &requests,
        "projected-five-node",
        1e-8,
        Default::default(),
        fixture::fan,
    )
    .unwrap();
    let report = audit(body.model());
    assert!(report.proven && report.intersections.absence_proven);
    assert_eq!(report.intersections.pairs.total_pairs, 595);
    assert!(report.intersections.pairs.next_pair.is_none());
    assert!(report.intersections.pairs.cells <= 1_000_000);
    let projected = report
        .intersections
        .pairs
        .pairs
        .iter()
        .filter_map(|p| match &p.boundary {
            Some(brep_core::face_contacts::SharedBoundary::ProjectedTrimPartition(c)) => {
                assert_eq!(c.faces, p.faces);
                assert!(c.cells > 0 && c.cells <= p.source_chart_cells);
                assert!(c.edge().is_some() != c.vertex().is_some());
                assert!(c.determinant_signs.iter().all(|s| *s == 1 || *s == -1));
                assert!(p.boundary_cells + p.source_chart_cells + p.hull_cells <= 1000);
                Some(c)
            }
            _ => None,
        })
        .count();
    assert!(projected >= 13);
    assert!(
        body.fields()
            .iter()
            .all(|p| p.absolute_curvature_upper.is_some_and(|x| x <= 10.))
    );
    assert_eq!(source, &before);
    #[cfg(feature = "codec")]
    {
        let restored: brep_core::Model =
            value_codec::from_str(&value_codec::to_string(body.model()).unwrap()).unwrap();
        assert_eq!(&restored, body.model());
        assert!(audit(&restored).proven);
    }
    eprintln!(
        "five_valent_global proven={} pairs={} projected={} cells={}",
        report.proven,
        report.intersections.pairs.total_pairs,
        projected,
        report.intersections.pairs.cells
    );
}

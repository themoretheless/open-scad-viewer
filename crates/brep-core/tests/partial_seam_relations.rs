use brep_core::feature_ops::edge_partition::{self, Limits, seam};
use num_rational::BigRational as Q;
#[cfg(feature = "codec")]
use value_codec::{Deserialize, Serialize};
#[path = "common/closed_periodic_source.rs"]
mod fixture;
fn source() -> brep_core::Model {
    let mut s = fixture::source();
    for l in &mut s.loops {
        let c = &mut l.coedges[0];
        *c = c
            .clone()
            .with_trim(brep_topology::CoedgeTrim {
                curve_parameter: [1., 3.],
                pcurve_parameter: [1., 3.],
                periodic_lift: [[0; 2]; 2],
            })
            .unwrap();
    }
    s
}
fn wire_starts(m: &brep_core::Model, vertex: usize) {
    for l in &m.loops {
        let c = &l.coedges[0];
        assert_eq!(m.edges[c.edge].vertices[usize::from(c.reversed)], vertex);
    }
}
#[test]
fn a_partial_relation_crossing_the_new_seam_has_exact_actual_fragment_ownership() {
    let s = source();
    let saved = s.clone();
    let p = seam::relocate(&s, 0, 2., "cross", Limits::default()).unwrap();
    let m = p.model();
    assert_eq!(m.edges.len(), 2);
    assert_eq!(m.vertices.len(), 2);
    assert_eq!(m.vertices[0], s.vertices[0]);
    assert_eq!(p.seam_vertex(), 1);
    assert!(m.persistent_naming_complete());
    let generated =
        m.1.change_set
            .changes
            .iter()
            .find(|c| {
                c.kind == brep_core::ChangeKind::Generated
                    && c.children.contains(&m.1.vertices[p.seam_vertex()])
            })
            .unwrap();
    assert_eq!(generated.provenance.operation, "edge-partition");
    assert_eq!(generated.role, "original-edge-cut-vertex");
    assert_eq!(p.cycle(), &[1, 0]);
    wire_starts(m, p.seam_vertex());
    assert_eq!(m.1.faces, s.1.faces);
    assert_eq!(m.1.bodies, s.1.bodies);
    for r in p.relations() {
        assert_eq!(r.original.curve_parameter, [1., 3.]);
        assert_eq!(r.fragments.len(), 2);
        assert_eq!(
            r.fragments[0].relation_range,
            [Q::from_integer(0.into()), Q::new(1.into(), 2.into())]
        );
        assert_eq!(
            r.fragments[1].relation_range,
            [Q::new(1.into(), 2.into()), Q::from_integer(1.into())]
        );
        for f in &r.fragments {
            assert!(
                m.loops[r.wire]
                    .coedges
                    .iter()
                    .any(|c| c.edge == f.edge && c.trim() == Some(f.transferred))
            );
        }
    }
    assert!(p.reverify().unwrap().trim_regions_valid);
    assert!((p.evidence().numerical_volume_mm3 - std::f64::consts::PI).abs() < 1e-6);
    assert_eq!(s, saved);
}
#[test]
fn reversed_traversal_and_independent_directed_partial_clocks_rotate_both_wires() {
    let mut s = source();
    s.edges[0].curve = s.edges[0].curve.reverse().unwrap();
    for k in &mut s.edges[0].curve.knots {
        *k = -3. + 2.5 * *k;
    }
    for (i, l) in s.loops.iter_mut().enumerate() {
        let c = &mut l.coedges[0];
        c.reversed = true;
        for k in &mut c.pcurve.knots {
            *k = -11. + 10. * *k;
        }
        let mut g = [-1., 5.];
        let mut p = [-3., 21.];
        if i == 1 {
            g.reverse();
            p.reverse();
        }
        *c = c
            .clone()
            .with_trim(brep_topology::CoedgeTrim {
                curve_parameter: g,
                pcurve_parameter: p,
                periodic_lift: [[0; 2]; 2],
            })
            .unwrap();
    }
    let saved = s.clone();
    let p = seam::relocate(&s, 0, 0.125, "backwards", Limits::default()).unwrap();
    wire_starts(p.model(), p.seam_vertex());
    assert!(p.reverify().unwrap().boundary_complete);
    for r in p.relations() {
        assert_eq!(r.fragments.len(), 2);
        for f in &r.fragments {
            assert_eq!(
                f.transferred.curve_parameter[0] < f.transferred.curve_parameter[1],
                r.wire == 0
            );
        }
    }
    assert_eq!(s, saved);
}
#[test]
fn transported_clock_breaks_survive_model_codec_and_later_native_edge_edits() {
    let s = source();
    let p = seam::relocate(&s, 0, 2., "persist", Limits::default()).unwrap();
    #[cfg(feature = "codec")]
    {
        let value = p.model().to_value();
        let loaded = brep_core::Model::from_value(value).unwrap();
        assert_eq!(&loaded, p.model());
    }
    let later =
        edge_partition::split(p.model(), p.cycle()[0], &[2.5], "later", Limits::default()).unwrap();
    assert!(later.reverify().unwrap().trim_regions_valid);
    assert_eq!(later.relations().len(), 2);
    assert_eq!(p.model().edges.len(), 2);
}
#[test]
fn seam_outside_the_requested_relation_retains_one_fragment_and_original_seam_is_a_noop() {
    let s = source();
    let p = seam::relocate(&s, 0, 0.375, "outside", Limits::default()).unwrap();
    for r in p.relations() {
        assert_eq!(r.fragments.len(), 1);
        assert_eq!(
            r.fragments[0].relation_range,
            [Q::from_integer(0.into()), Q::from_integer(1.into())]
        );
    }
    wire_starts(p.model(), p.seam_vertex());
    let unchanged = seam::relocate(&s, 0, 0., "identity", Limits::default()).unwrap();
    assert_eq!(unchanged.model(), &s);
    assert_eq!(unchanged.cycle(), &[0]);
    assert!(unchanged.reverify().unwrap().trim_regions_valid);
}
#[test]
fn source_relation_defects_and_exhausted_shared_work_do_not_publish_a_partial_result() {
    let s = source();
    let saved = s.clone();
    assert!(
        seam::relocate(
            &s,
            0,
            2.,
            "tiny",
            Limits {
                max_cells: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    let mut bad = s.clone();
    let c = &mut bad.loops[0].coedges[0];
    let mut trim = c.trim().unwrap();
    trim.pcurve_parameter = [0.75, 3.];
    *c = c.clone().with_trim(trim).unwrap();
    assert!(seam::relocate(&bad, 0, 2., "bad", Limits::default()).is_err());
    assert_eq!(s, saved);
}

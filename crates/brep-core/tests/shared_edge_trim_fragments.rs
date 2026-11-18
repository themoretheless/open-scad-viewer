use brep_core::{Model, feature_ops::edge_partition::{self, Limits}};
use brep_topology::CoedgeTrim;
use num_rational::BigRational as Q;
use num_traits::{One, Zero};
fn source(g: [f64; 2], p: [f64; 2], fraction: [f64; 2]) -> Model {
    let mut s = brep_core::cuboid([0.; 3], [2., 3., 4.]).unwrap();
    for e in &mut s.edges {
        for k in &mut e.curve.knots {
            *k = g[0] + (g[1] - g[0]) * *k;
        }
    }
    for w in &mut s.loops {
        for c in &mut w.coedges {
            for k in &mut c.pcurve.knots {
                *k = p[0] + (p[1] - p[0]) * *k;
            }
            if c.edge == 0 {
                let mut curve = fraction.map(|t| g[0] + (g[1] - g[0]) * t);
                let mut pc = fraction.map(|t| p[0] + (p[1] - p[0]) * t);
                if c.reversed {
                    curve.reverse();
                    pc.reverse();
                }
                *c = c
                    .clone()
                    .with_trim(CoedgeTrim {
                        curve_parameter: curve,
                        pcurve_parameter: pc,
                        periodic_lift: [[0; 2]; 2],
                    })
                    .unwrap();
            }
        }
    }
    s.validate().unwrap();
    s
}
fn ownership(p: &edge_partition::Split) {
    for relation in p.relations() {
        let original = &p.source().loops[relation.wire].coedges[relation.coedge];
        assert_eq!(original.trim(), Some(relation.original));
        assert_eq!(
            relation.fragments.first().unwrap().relation_range[0],
            Q::zero()
        );
        assert_eq!(
            relation.fragments.last().unwrap().relation_range[1],
            Q::one()
        );
        for pair in relation.fragments.windows(2) {
            assert_eq!(pair[0].relation_range[1], pair[1].relation_range[0]);
        }
        let face = &p.model().faces[relation.face];
        for f in &relation.fragments {
            assert!(f.original_distance_upper <= p.source().tolerance_mm);
            assert!(f.original_curve_distance_upper <= p.source().tolerance_mm);
            assert!(f.relation_range[0] < f.relation_range[1]);
            let actual = p.model().loops[face.outer]
                .coedges
                .iter()
                .find(|c| c.edge == f.edge)
                .unwrap();
            assert_eq!(actual.trim(), Some(f.transferred));
        }
    }
    assert!(p.reverify().unwrap().boundary_complete);
}
#[test]
fn partial_nonunit_descending_clocks_retain_exact_original_relation_coverage() {
    let s = source([-3., 7.], [-11., 29.], [0.1, 0.8]);
    let before = s.clone();
    let p =
        edge_partition::split(&s, 0, &[-1., 1., 4.], "partial-clocks", Limits::default()).unwrap();
    assert_eq!(p.relations().len(), 2);
    let mut counts = p
        .relations()
        .iter()
        .map(|r| r.fragments.len())
        .collect::<Vec<_>>();
    counts.sort();
    assert_eq!(counts, vec![3, 4]);
    ownership(&p);
    assert_eq!(s, before);
}
#[test]
fn untouched_relation_ends_do_not_expand_into_unrequested_children() {
    let s = source([0., 1.], [0., 1.], [0.26, 0.31]);
    let p = edge_partition::split(&s, 0, &[0.1, 0.5, 0.9], "small-relation", Limits::default())
        .unwrap();
    assert!(p.relations().iter().all(|r| r.fragments.len() == 1));
    let metadata = p
        .model()
        .loops
        .iter()
        .flat_map(|w| &w.coedges)
        .filter(|c| p.edges().contains(&c.edge) && c.trim().is_some())
        .count();
    assert_eq!(metadata, 2);
    ownership(&p);
}
#[test]
fn rational_arc_fragments_use_actual_saved_pcurves_and_independent_original_clocks() {
    let mut s = brep_core::cylinder(2., 3.).unwrap();
    let e = s
        .edges
        .iter()
        .position(|e| {
            e.curve.degree == 2 && e.curve.weights.iter().any(|w| *w != e.curve.weights[0])
        })
        .unwrap();
    let d = s.edges[e].curve.domain();
    for w in &mut s.loops {
        for c in &mut w.coedges {
            if c.edge == e {
                for k in &mut c.pcurve.knots {
                    *k = 2. + 8. * *k;
                }
                *c = c
                    .clone()
                    .with_trim(CoedgeTrim {
                        curve_parameter: [d[0] + 0.13 * (d[1] - d[0]), d[0] + 0.87 * (d[1] - d[0])],
                        pcurve_parameter: [3.04, 8.96],
                        periodic_lift: [[0; 2]; 2],
                    })
                    .unwrap();
            }
        }
    }
    let before = s.clone();
    let p = edge_partition::split(
        &s,
        e,
        &[d[0] + 0.2 * (d[1] - d[0]), d[0] + 0.6 * (d[1] - d[0])],
        "rational-partial",
        Limits::default(),
    )
    .unwrap();
    ownership(&p);
    assert_eq!(s, before);
}
#[test]
fn invalid_authored_relations_and_exhaustion_cannot_lose_original_metadata() {
    let s = source([0., 1.], [0., 1.], [0.17, 0.83]);
    let before = s.clone();
    assert!(
        edge_partition::split(
            &s,
            0,
            &[0.3, 0.7],
            "budget",
            Limits {
                max_cells: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    let mut bad = s.clone();
    for w in &mut bad.loops {
        for c in &mut w.coedges {
            if c.edge == 0 {
                let mut t = c.trim().unwrap();
                t.pcurve_parameter = [0.4, 0.6];
                *c = c.clone().with_trim(t).unwrap();
            }
        }
    }
    assert!(
        edge_partition::split(&bad, 0, &[0.3, 0.7], "wrong-relation", Limits::default()).is_err()
    );
    assert_eq!(s, before);
}

#[test]
fn exact_reflected_fragment_endpoints_are_not_replaced_by_rounded_scalar_identity() {
    use num_traits::ToPrimitive;
    let s = source([-3.1, 7.3], [-11.7, 29.4], [0.13, 0.83]);
    let p = edge_partition::split(
        &s,
        0,
        &[-1.2, 1.1, 4.4],
        "reflected-rounding",
        Limits::default(),
    )
    .unwrap();
    ownership(&p);
    let mut nonrepresentable = false;
    for r in p.relations() {
        let a = Q::from_float(r.original.curve_parameter[0]).unwrap();
        let b = Q::from_float(r.original.curve_parameter[1]).unwrap();
        for f in &r.fragments {
            for t in &f.relation_range {
                let exact = &a + (&b - &a) * t;
                nonrepresentable |= Q::from_float(exact.to_f64().unwrap()).unwrap() != exact;
            }
        }
    }
    assert!(
        nonrepresentable,
        "fixture must exercise exact-to-binary64 clock error enclosure"
    );
    let e = p.edges()[1];
    let d = p.model().edges[e].curve.domain();
    let second = edge_partition::split(
        p.model(),
        e,
        &[(d[0] + d[1]) * 0.5],
        "repeat-partial",
        Limits::default(),
    )
    .unwrap();
    ownership(&second);
}

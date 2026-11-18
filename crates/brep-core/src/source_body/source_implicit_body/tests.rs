use super::*;
fn limits() -> Limits {
    Limits {
        implicit_exact_work: 16000000,
        implicit_cells: 100000,
        regularity_cells: 100000,
        linear_cells: 0,
        volume: crate::volume_validity::Limits {
            boundary: crate::boundary_embedding::Limits {
                exact_work: 1000000,
                trim_pairs: 10000,
                trim_cells: 100000,
                trim_domain_cells: 1000000,
                spans: 1000,
                contacts: crate::face_contacts::Limits {
                    pairs: 10000,
                    cells: 100000,
                    domain_cells: 1000000,
                    cells_per_pair: 1000,
                    domain_cells_per_pair: 10000,
                },
            },
            nesting_pairs: 100,
            nesting_cells: 100000,
            nesting_domain_cells: 1000000,
            orientation_cells: 100000,
            orientation_domain_cells: 1000000,
            orientation_spans: 100,
        },
    }
}
fn body(attempt: Attempt) -> Box<SphereBody> {
    match attempt {
        Attempt::Certified(b) => b,
        Attempt::Refused(r) => {
            if let Some(v) = &r.volume {
                eprintln!(
                    "agreement={} joins={} trim={} face proofs={:?} pair proofs={:?} nesting={:?} orientation={:?}",
                    v.boundary.agreement.all_equal,
                    v.boundary.agreement.all_joins_exact,
                    v.boundary.trim.all_valid,
                    v.boundary
                        .intersections
                        .faces
                        .faces
                        .iter()
                        .map(|f| (
                            f.face,
                            f.absence_proven(),
                            f.result.as_ref().map(|r| (r.reason, r.spans))
                        ))
                        .collect::<Vec<_>>(),
                    v.boundary
                        .intersections
                        .pairs
                        .pairs
                        .iter()
                        .map(|p| (p.faces, p.reason))
                        .collect::<Vec<_>>(),
                    v.nesting.as_ref().map(|n| n.roles_consistent),
                    v.orientations.iter().map(|o| o.outward).collect::<Vec<_>>()
                );
            }
            panic!(
                "implicit source refused {:?}; implicitWork={} cells={}, links={} regularity={:?} volume={:?}",
                r.reason,
                r.implicit_exact_work,
                r.implicit_cells,
                r.quotient_vertex_links_complete,
                r.regularity
                    .as_ref()
                    .map(|p| (&p.unresolved_faces, p.all_faces_regular_modulo_poles)),
                r.volume.as_ref().map(|p| p.proven)
            )
        }
    }
}
#[test]
fn original_exact_sphere_including_real_poles_is_globally_owned_and_g2() {
    for radius in [2., 3_f64.next_up(), 3., 0.125] {
        let model = crate::analytic::sphere_exact(radius).unwrap();
        let before = model.clone();
        let b = body(verify_sphere(&model, [0.; 3], radius, 1e-8, limits()).unwrap());
        let r = b.report();
        assert!(
            r.certified
                && r.source_boundary_exact
                && r.source_topology_preserved
                && r.source_g1
                && r.source_g2
        );
        assert_eq!(r.boundary_error_upper, Some(0.));
        assert!(r.quotient_vertex_links_complete);
        assert_eq!(b.model(), &before);
        assert_eq!(model, before);
        assert!(
            r.volume
                .as_ref()
                .unwrap()
                .boundary
                .intersections
                .absence_proven
        );
        if radius == 2. {
            assert_eq!(r.collapsed_edges.len(), 2);
            assert!(!r.regularity.as_ref().unwrap().all_faces_regular);
            assert!(
                r.regularity
                    .as_ref()
                    .unwrap()
                    .all_faces_regular_modulo_poles
            );
        }
        eprintln!(
            "original sphere radius={radius} exact/global/G2; pole edges={} source exact work={} gradient cells={}",
            r.collapsed_edges.len(),
            r.implicit_exact_work,
            r.implicit_cells
        );
    }
}
#[test]
fn wrong_source_coefficient_or_incomplete_pole_fan_never_returns_smooth_solid() {
    let original = crate::analytic::sphere_exact(2.).unwrap();
    for case in 0..5 {
        let mut m = original.clone();
        let mut radius = 2.;
        match case {
            0 => radius = 2_f64.next_up(),
            1 => {
                m.faces[0].surface.control_points[1][1][0] =
                    m.faces[0].surface.control_points[1][1][0].next_up()
            }
            2 => m.faces[0].surface.weights[1][1] = m.faces[0].surface.weights[1][1].next_up(),
            3 => m.shells[0].faces[0].reversed ^= true,
            4 => {
                m.faces.pop();
            }
            _ => unreachable!(),
        }
        let before = m.clone();
        let r = verify_sphere(&m, [0.; 3], radius, 1e-8, limits());
        assert!(
            !matches!(r, Ok(Attempt::Certified(_))),
            "case {case} returned Solid"
        );
        assert_eq!(m, before);
    }
}

fn signed_chart(model: &mut Model, gauge: u8) {
    for face_id in 0..model.faces.len() {
        let wires = std::iter::once(model.faces[face_id].outer)
            .chain(model.faces[face_id].holes.iter().copied())
            .collect::<Vec<_>>();
        let s = &mut model.faces[face_id].surface;
        if gauge & 4 != 0 {
            *s = crate::face_injectivity::canonical_pole_surface(s, 1);
        }
        if gauge & 1 != 0 {
            s.control_points.reverse();
            s.weights.reverse();
        }
        if gauge & 2 != 0 {
            for row in &mut s.control_points {
                row.reverse();
            }
            for row in &mut s.weights {
                row.reverse();
            }
        }
        for wire in wires {
            for c in &mut model.loops[wire].coedges {
                for p in &mut c.pcurve.control_points {
                    if gauge & 4 != 0 {
                        p.swap(0, 1);
                    }
                    if gauge & 1 != 0 {
                        p[0] = 1. - p[0];
                    }
                    if gauge & 2 != 0 {
                        p[1] = 1. - p[1];
                    }
                }
            }
            if gauge.count_ones() % 2 != 0 {
                model.loops[wire].coedges.reverse();
                for c in &mut model.loops[wire].coedges {
                    c.reversed ^= true;
                    c.pcurve = c.pcurve.reverse().unwrap();
                }
            }
        }
    }
    if gauge.count_ones() % 2 != 0 {
        for shell in &mut model.shells {
            for u in &mut shell.faces {
                u.reversed ^= true;
            }
        }
    }
}
#[test]
fn exact_original_pole_g2_survives_all_signed_tensor_chart_gauges() {
    for radius in [2., 3.] {
        let original = crate::analytic::sphere_exact(radius).unwrap();
        for gauge in 1..8 {
            let mut m = original.clone();
            signed_chart(&mut m, gauge);
            m.validate().unwrap();
            let before = m.clone();
            let b = body(verify_sphere(&m, [0.; 3], radius, 1e-8, limits()).unwrap());
            assert!(b.report().source_g2);
            assert_eq!(b.model(), &before);
            assert_eq!(m, before);
            assert_eq!(
                b.report().collapsed_edges.len(),
                if radius == 2. { 2 } else { 0 }
            );
            for p in b
                .report()
                .regularity
                .as_ref()
                .unwrap()
                .pole_quotients
                .iter()
                .flatten()
            {
                assert_eq!(p.collapsed_axis, usize::from(gauge & 4 != 0));
            }
            eprintln!("sphere radius={radius} exact original G2 tensor gauge {gauge} passed");
        }
    }
}
#[test]
fn implicit_source_sphere_budgets_and_invalid_tail_inputs_never_return_partial_solids() {
    let m = crate::analytic::sphere_exact(2.).unwrap();
    let b = body(verify_sphere(&m, [0.; 3], 2., 1e-8, limits()).unwrap());
    let r = b.report();
    for stage in 0..8 {
        let mut l = limits();
        match stage {
            0 => l.implicit_exact_work = 0,
            1 => l.implicit_exact_work = r.implicit_exact_work - 1,
            2 => l.implicit_cells = 0,
            3 => l.implicit_cells = r.implicit_cells - 1,
            4 => l.regularity_cells = 0,
            5 => l.regularity_cells = r.regularity.as_ref().unwrap().cells - 1,
            6 => {
                l.volume.boundary.exact_work =
                    r.volume.as_ref().unwrap().boundary.agreement.work - 1
            }
            7 => l.volume.boundary.contacts.pairs = 27,
            _ => unreachable!(),
        }
        let Attempt::Refused(refused) = verify_sphere(&m, [0.; 3], 2., 1e-8, l).unwrap() else {
            panic!("short original sphere stage {stage} returned Solid");
        };
        assert!(
            !refused.certified
                && !refused.source_g1
                && !refused.source_g2
                && !refused.source_topology_preserved
        );
        assert!(
            refused.implicit_exact_work <= l.implicit_exact_work
                && refused.implicit_cells <= l.implicit_cells
        );
    }
    let mut l = limits();
    l.implicit_exact_work = 0;
    l.volume.orientation_cells = 1000001;
    assert!(verify_sphere(&m, [0.; 3], 2., 1e-8, l).is_err());
    let mut malformed = m;
    malformed.faces.last_mut().unwrap().surface.weights[0][0] = 0.;
    let mut l = limits();
    l.implicit_exact_work = 0;
    assert!(verify_sphere(&malformed, [0.; 3], 2., 1e-8, l).is_err());
}

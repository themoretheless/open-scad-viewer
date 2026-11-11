use super::*;
use crate::sweep_smoothness::complete::classified::{self as actual, Class};
use nurbs_core::{
    progressive_sweep::{
        Options, Orientation, OriginalClassifiedVolumeLimits, OriginalSmoothVolumeLimits, Spacing,
        constant_vector_law,
    },
    surface::Surface,
};
fn limits() -> ClassifiedClosedLimits {
    ClassifiedClosedLimits {
        original: OriginalClassifiedClosedMaterialLimits {
            volume: OriginalClassifiedVolumeLimits {
                charts: OriginalSmoothVolumeLimits {
                    order: 2,
                    cells: 1000000,
                    exact_work: 1000000,
                    products: 1000000,
                },
                steps: 1000000,
            },
            material: MaterialLimits {
                pairs: 10000,
                cells: 100000,
                domain_cells: 1000000,
            },
        },
        retained: smooth::SmoothClosedLimits {
            source: super::super::tests::limits().into(),
            original_smoothness: None,
            candidate: crate::AffineStationLimits {
                quantum: 2f64.powi(-18),
                tolerance: 0.03,
                work: 1000000,
                matrix_cells: 100000,
            },
            smoothness: crate::sweep_smoothness::complete::Limits {
                order: 2,
                exact_work: 16000000,
                agreement_work: 1000000,
                regularity_cells: 100000,
            },
            atlas: IdealLimits {
                cells: 1000000,
                exact_work: 1000000,
                projection: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            },
            binding_work: 1000000,
            orientation_cells: 100000,
        },
        incidence_steps: 1000000,
    }
}
fn material(g1: bool, hole: bool) -> (LoftCap, Vec<Curve>, Vec<usize>, Vec<OriginalProfileClass>) {
    let r = 1. / 128.;
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-1., -1., 3., 3.],
        knots_v: vec![2., 2., 6., 6.],
        control_points: vec![
            vec![vec![0., -r, -r], vec![0., -r, r]],
            vec![vec![0., r, -r], vec![0., r, r]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let mut profiles = Vec::new();
    let mut trims = Vec::new();
    let mut sizes = Vec::new();
    let mut classes = Vec::new();
    for ring in 0..if hole { 2 } else { 1 } {
        let scale = if ring == 0 { 1. } else { 0.5 };
        let (points, degree, ring_classes) = if g1 {
            (
                vec![
                    [1., 0.],
                    [1., 1.],
                    [0., 1.],
                    [-1., 1.],
                    [-1., 0.],
                    [-1., -1.],
                    [0., -1.],
                    [1., -1.],
                    [1., 0.],
                ],
                2,
                vec![
                    OriginalProfileClass::G1,
                    OriginalProfileClass::G2,
                    OriginalProfileClass::G2,
                    OriginalProfileClass::G1,
                ],
            )
        } else if ring == 0 {
            (
                vec![
                    [-1., -1.],
                    [0., -1.],
                    [1., -1.],
                    [1., 1.],
                    [-1., 1.],
                    [-1., -1.],
                ],
                1,
                vec![
                    OriginalProfileClass::G2,
                    OriginalProfileClass::C0,
                    OriginalProfileClass::C0,
                    OriginalProfileClass::C0,
                    OriginalProfileClass::C0,
                ],
            )
        } else {
            (
                vec![
                    [-1., -1.],
                    [-1., 0.],
                    [-1., 1.],
                    [1., 1.],
                    [1., -1.],
                    [-1., -1.],
                ],
                1,
                vec![
                    OriginalProfileClass::G2,
                    OriginalProfileClass::C0,
                    OriginalProfileClass::C0,
                    OriginalProfileClass::C0,
                    OriginalProfileClass::C0,
                ],
            )
        };
        let count = (points.len() - 1) / degree;
        let mut wire = Vec::new();
        for i in 0..count {
            let weights = if degree == 2 {
                vec![1., if i == 0 { 0.75 } else { 0.5 }, 1.]
            } else {
                vec![1.; 2]
            };
            let pts = &points[i * degree..=i * degree + degree];
            profiles.push(Curve {
                degree,
                knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
                control_points: pts
                    .iter()
                    .map(|p| vec![0., r * scale * p[0], r * scale * p[1]])
                    .collect(),
                weights: weights.clone(),
                periodic: false,
            });
            wire.push(Curve {
                degree,
                knots: [vec![7.; degree + 1], vec![11.; degree + 1]].concat(),
                control_points: pts
                    .iter()
                    .map(|p| vec![1. + 2. * scale * p[0], 4. + 2. * scale * p[1]])
                    .collect(),
                weights,
                periodic: false,
            });
        }
        sizes.push(count);
        classes.extend(ring_classes);
        trims.push(wire);
    }
    (LoftCap { surface, trims }, profiles, sizes, classes)
}
fn run(g1: bool, hole: bool, sharp: bool, mode: usize, arc: bool, image: bool, nonplanar: bool) {
    run_profiles(g1, hole, sharp, mode, arc, image, nonplanar, false)
}
fn run_profiles(
    g1: bool,
    hole: bool,
    sharp: bool,
    mode: usize,
    arc: bool,
    image: bool,
    nonplanar: bool,
    multispan: bool,
) {
    let (mut material, mut profiles, mut sizes, classes) = material(g1, hole);
    if multispan {
        assert!(!image);
        // Preserve two exact spans while putting their requested geometric join
        // INSIDE one NURBS curve. World and UV clocks have different raw domains.
        let mut offset = 0;
        for (ring, size) in sizes.iter_mut().enumerate() {
            let mut world = profiles[offset].clone();
            world
                .control_points
                .extend_from_slice(&profiles[offset + 1].control_points[1..]);
            world
                .weights
                .extend_from_slice(&profiles[offset + 1].weights[1..]);
            world.knots = [
                vec![-4.; world.degree + 1],
                vec![0.; world.degree],
                vec![4.; world.degree + 1],
            ]
            .concat();
            profiles.splice(offset..offset + 2, [world]);
            let wire = &mut material.trims[ring];
            let mut uv = wire[0].clone();
            uv.control_points
                .extend_from_slice(&wire[1].control_points[1..]);
            uv.weights.extend_from_slice(&wire[1].weights[1..]);
            uv.knots = [
                vec![7.; uv.degree + 1],
                vec![9.; uv.degree],
                vec![11.; uv.degree + 1],
            ]
            .concat();
            wire.splice(0..2, [uv]);
            *size -= 1;
            offset += *size;
        }
        assert_eq!(classes.len(), profiles.len() + sizes.len());
    }
    if nonplanar {
        assert!(!g1);
        let r = 1. / 128.;
        let height = 1. / 65536.;
        material.surface.control_points[1][1][0] = height;
        for c in &mut profiles {
            for p in &mut c.control_points {
                let u = (p[1] / r + 1.) * 0.5;
                let v = (p[2] / r + 1.) * 0.5;
                p[0] = height * u * v;
            }
        }
    }
    if image {
        for c in material.trims.iter_mut().flatten() {
            for (i, w) in c.weights.iter_mut().enumerate() {
                *w *= 2f64.powi(i as i32);
            }
        }
    }
    let path = super::super::tests::path(false, true);
    let scale = if mode == 0 {
        constant_vector_law([1., 0., 0.]).unwrap()
    } else {
        scalar([1., 1., 1.125, 1., 1.], 2., 6.)
    };
    let twist = if mode == 0 {
        constant_vector_law([0.; 3]).unwrap()
    } else {
        scalar([0., 0., 1. / 64., 0., 0.], 7., 11.)
    };
    let axis = Curve {
        degree: 8,
        knots: [vec![2.; 9], vec![6.; 9]].concat(),
        control_points: path
            .control_points
            .windows(2)
            .map(|p| (0..3).map(|k| 9. * (p[1][k] - p[0][k])).collect())
            .collect(),
        weights: vec![1.; 9],
        periodic: false,
    };
    let normal = constant_vector_law([0., 0., 1.]).unwrap();
    let mut guide = path.clone();
    for p in &mut guide.control_points {
        p[2] += 1.;
    }
    let axes = constant_vector_law([1., 1.125, 0.875]).unwrap();
    let center = constant_vector_law([1. / 1024., 1. / 2048., 0.]).unwrap();
    let count = if arc || mode == 2 { 65 } else { 33 };
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [0., 0., 1.],
            orientation: match mode {
                1 => Orientation::Fixed,
                2 => Orientation::FixedNormal,
                _ => Orientation::RotationMinimizing,
            },
            spacing: if arc {
                Spacing::ArcLength {
                    tolerance: 0.001,
                    max_cells: 100000,
                }
            } else {
                Spacing::Parameter
            },
            initial_sections: 33,
            max_sections: 129,
            max_deviation: 0.05,
        },
    )
    .unwrap();
    let sweep = match mode {
        1 => sweep.with_frame_laws(&axis, &normal).unwrap(),
        2 => sweep.with_orientation_guide(&guide).unwrap(),
        _ => sweep
            .with_spatial_rmf_error_limits(if arc { 4096 } else { 1024 }, 100000, 1000000)
            .unwrap(),
    };
    let sweep = if mode > 0 {
        sweep.with_affine_laws(&axes, &center).unwrap()
    } else {
        sweep
    };
    let before = (
        material.surface.clone(),
        material.trims.clone(),
        profiles.clone(),
        path.clone(),
    );
    let sharp = if sharp { vec![0] } else { Vec::new() };
    if image {
        assert!(
            matches!(
                prepare_closed_source(
                    &sweep,
                    &sizes,
                    count,
                    &material,
                    1e-8,
                    0.05,
                    limits().retained.source
                )
                .unwrap(),
                PreparedAttempt::Refused(_)
            ),
            "image ownership must not grant the legacy normalized-clock proof"
        );
    }
    if mode == 2 {
        let ClassifiedClosedAttempt::Refused(coarse) =
            construct_classified_closed_with_original_material(
                &sweep,
                &sizes,
                33,
                &material,
                &classes,
                1e-8,
                0.05,
                &sharp,
                limits(),
            )
            .unwrap()
        else {
            panic!("coarse guide level violated the unchanged error budget")
        };
        assert_eq!(coarse.reason, "boundary-error-budget-exceeded");
        assert!(coarse.binding.is_none());
    }
    let r = construct_classified_closed_with_original_material(
        &sweep,
        &sizes,
        count,
        &material,
        &classes,
        1e-8,
        0.05,
        &sharp,
        limits(),
    )
    .unwrap();
    let body = match r {
        ClassifiedClosedAttempt::Certified(b) => b,
        ClassifiedClosedAttempt::Refused(r) => panic!(
            "classified closed mode={mode} arc={arc} image={image} nonplanar={nonplanar} g1={g1} hole={hole} sharp={sharp:?}: {}; roles={:?} boundary={:?} source={:?}",
            r.reason,
            r.edge_roles.as_ref().map(|r| (r.reason, r.uncertain_edge)),
            r.boundary
                .as_ref()
                .map(|r| (r.reason, r.uncertain_edge, r.exact_work)),
            r.source.as_ref().map(|r| r.reason)
        ),
    };
    assert!(std::ptr::eq(body.original().material(), &material));
    assert!(std::ptr::eq(body.original().volume().sweep(), &sweep));
    assert_eq!(body.original().boundary_classes(), classes);
    assert_eq!(body.original().closed_seam_order(), 2);
    if image {
        assert_eq!(
            if g1 {
                body.original().projective_image_spans()
            } else {
                body.original().line_image_spans()
            },
            profiles.len()
        );
        assert!(
            body.original()
                .bindings()
                .iter()
                .all(|b| b.kind() != MaterialSpanBindingKind::NormalizedMap)
        );
    }

    let r = body.report();
    assert!(
        r.certified
            && r.volume.as_ref().unwrap().embedding_certified
            && r.volume.as_ref().unwrap().parameter_topology_preserved
    );
    assert!(r.boundary_error_upper.unwrap() <= 0.05);
    let edges = r.edge_roles.as_ref().unwrap();
    let boundary = r.boundary.as_ref().unwrap();
    assert!(edges.certified && boundary.certified);
    assert_eq!(edges.classes, boundary.classes);
    assert!(edges.roles.iter().all(Option::is_some));
    assert_eq!(edges.classes.len(), body.model().edges.len());
    let layers = count - 1;
    let spans = classes.len();
    let source_c0 = classes
        .iter()
        .filter(|c| **c == OriginalProfileClass::C0)
        .count();
    assert_eq!(
        edges.classes.iter().filter(|c| **c == Class::C0).count(),
        source_c0 * layers + if sharp.is_empty() { 0 } else { spans }
    );
    assert_eq!(
        edges.classes.iter().filter(|c| **c == Class::G1).count(),
        classes
            .iter()
            .filter(|c| **c == OriginalProfileClass::G1)
            .count()
            * layers
    );
    let closing = edges
        .roles
        .iter()
        .enumerate()
        .filter(|(_, r)| matches!(r, Some(EdgeRole::Station { station: 0, .. })))
        .collect::<Vec<_>>();
    assert_eq!(closing.len(), spans);
    assert!(closing.iter().all(|(i, _)| edges.classes[*i]
        == if sharp.is_empty() {
            Class::G2
        } else {
            Class::C0
        }));
    let tight = actual::Limits {
        incidence_steps: boundary.steps,
        exact_work: boundary.exact_work,
        agreement_work: boundary.agreement.as_ref().unwrap().work,
        regularity_cells: boundary.regularity.as_ref().unwrap().cells,
    };
    assert!(
        actual::inspect(body.model(), &edges.classes, tight)
            .unwrap()
            .certified
    );
    for d in 0..4 {
        let mut short = tight;
        match d {
            0 => short.incidence_steps -= 1,
            1 => short.exact_work -= 1,
            2 => short.agreement_work -= 1,
            _ => short.regularity_cells -= 1,
        }
        let q = actual::inspect(body.model(), &edges.classes, short).unwrap();
        assert!(
            !q.certified && q.steps <= short.incidence_steps && q.exact_work <= short.exact_work
        );
    }
    let mut joint = limits();
    joint.incidence_steps = r.incidence_steps;
    assert!(matches!(
        construct_classified_closed_with_original_material(
            &sweep, &sizes, count, &material, &classes, 1e-8, 0.05, &sharp, joint
        )
        .unwrap(),
        ClassifiedClosedAttempt::Certified(_)
    ));
    joint.incidence_steps -= 1;
    let ClassifiedClosedAttempt::Refused(short) =
        construct_classified_closed_with_original_material(
            &sweep, &sizes, count, &material, &classes, 1e-8, 0.05, &sharp, joint,
        )
        .unwrap()
    else {
        panic!("incomplete joint role/census budget admitted")
    };
    assert!(short.incidence_steps <= joint.incidence_steps && short.volume.is_none());
    if g1 {
        let mut all_g2 = edges.classes.clone();
        for c in &mut all_g2 {
            if *c == Class::G1 {
                *c = Class::G2;
            }
        }
        assert!(
            !actual::inspect(
                body.model(),
                &all_g2,
                actual::Limits {
                    incidence_steps: 1000000,
                    exact_work: 16000000,
                    agreement_work: 1000000,
                    regularity_cells: 100000
                }
            )
            .unwrap()
            .certified
        );
        let mut upgraded = classes.clone();
        upgraded[0] = OriginalProfileClass::G2;
        let ClassifiedClosedAttempt::Refused(q) =
            construct_classified_closed_with_original_material(
                &sweep,
                &sizes,
                count,
                &material,
                &upgraded,
                1e-8,
                0.05,
                &sharp,
                limits(),
            )
            .unwrap()
        else {
            panic!("source curvature jump acquired G2")
        };
        assert!(q.original_refusal.is_some() && q.binding.is_none());
        if mode == 0 && !arc && !image {
            let transformed = gauged_and_permuted(body.model());
            let q = verify_classified_closed_with_original_material(
                ClosedSweepSource { sweep: &sweep, loop_sizes: &sizes, sections: count, material: &material },
                &classes,
                SweepAccuracy { tolerance_uv: 1e-8, error_budget: 0.05 },
                &sharp,
                limits(),
                &transformed,
            )
            .unwrap();
            let q = match q {
                ClassifiedClosedAttempt::Certified(q) => q,
                ClassifiedClosedAttempt::Refused(r) => panic!(
                    "gauged actual roles: {} binding={:?} roles={:?} boundary={:?}",
                    r.reason,
                    r.binding.as_ref().map(|b| (b.certified, b.reason)),
                    r.edge_roles.as_ref().map(|r| (r.reason, r.uncertain_edge)),
                    r.boundary.as_ref().map(|r| r.reason)
                ),
            };
            assert_eq!(
                q.report().edge_roles.as_ref().unwrap().classes,
                edges.classes
            );
            assert_eq!(q.report().edge_roles.as_ref().unwrap().roles, edges.roles);
            assert!(q.report().binding.as_ref().unwrap().reparameterized_faces() > 0);
            let mut broken = body.model().clone();
            broken.faces[0].surface.control_points[1][1][0] += 2f64.powi(-40);
            let q = verify_classified_closed_with_original_material(
                ClosedSweepSource { sweep: &sweep, loop_sizes: &sizes, sections: count, material: &material },
                &classes,
                SweepAccuracy { tolerance_uv: 1e-8, error_budget: 0.05 },
                &sharp,
                limits(),
                &broken,
            )
            .unwrap();
            let ClassifiedClosedAttempt::Refused(q) = q else {
                panic!("sub-tolerance interior coefficient change borrowed a source field")
            };
            assert_eq!(q.reason, "classified-closed-actual-binding-unproved");
            assert!(q.volume.is_none());
        }
    }
    eprintln!(
        "classified closed mode={mode} arc={arc} image={image} nonplanar={nonplanar} g1={g1} hole={hole} sharp={sharp:?}: faces={} edges={} C0={} G1={} G2={} incidence={} exact={} error={:?}",
        body.model().faces.len(),
        body.model().edges.len(),
        boundary.c0_edges.len(),
        edges.classes.iter().filter(|c| **c == Class::G1).count(),
        edges.classes.iter().filter(|c| **c == Class::G2).count(),
        r.incidence_steps,
        boundary.exact_work,
        r.boundary_error_upper
    );
    assert_eq!(
        before,
        (
            material.surface.clone(),
            material.trims.clone(),
            profiles.clone(),
            path.clone()
        )
    );
}
#[test]
fn actual_closed_g1_curvature_jumps_and_g2_seam_share_the_original_material() {
    run(true, false, false, 0, false, false, false)
}
#[test]
fn actual_closed_hollow_body_preserves_explicit_profile_c0_and_g2_seams() {
    run(false, true, false, 0, false, false, false)
}
#[test]
fn sharp_closed_station_stays_c0_beside_source_g2_motion() {
    run(false, false, true, 0, false, false, false)
}

// Every exact signed tensor symmetry is represented within ONE actual body;
// face order is then reversed. World geometry and material roles are retained.
fn gauged_and_permuted(model: &Model) -> Model {
    let mut chart = model.clone();
    for face in 0..chart.faces.len() {
        let gauge = (face % 8) as u8;
        let outer = chart.faces[face].outer;
        let s = &mut chart.faces[face].surface;
        if gauge & 4 != 0 {
            std::mem::swap(&mut s.degree_u, &mut s.degree_v);
            std::mem::swap(&mut s.knots_u, &mut s.knots_v);
            let old = std::mem::take(&mut s.control_points);
            s.control_points = (0..old[0].len())
                .map(|j| old.iter().map(|r| r[j].clone()).collect())
                .collect();
            let old = std::mem::take(&mut s.weights);
            s.weights = (0..old[0].len())
                .map(|j| old.iter().map(|r| r[j]).collect())
                .collect();
        }
        if gauge & 1 != 0 {
            s.control_points.reverse();
            s.weights.reverse();
        }
        if gauge & 2 != 0 {
            for r in &mut s.control_points {
                r.reverse();
            }
            for r in &mut s.weights {
                r.reverse();
            }
        }
        for c in &mut chart.loops[outer].coedges {
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
            let wire = &mut chart.loops[outer];
            wire.coedges.reverse();
            for c in &mut wire.coedges {
                c.reversed = !c.reversed;
                c.pcurve = c.pcurve.reverse().unwrap();
            }
        }
        for shell in &mut chart.shells {
            for u in &mut shell.faces {
                if u.face == face {
                    u.reversed ^= gauge.count_ones() % 2 != 0;
                }
            }
        }
    }
    let n = chart.faces.len();
    chart.faces.reverse();
    for shell in &mut chart.shells {
        for u in &mut shell.faces {
            u.face = n - 1 - u.face;
        }
    }
    chart.validate().unwrap();
    chart
}

fn scalar(values: [f64; 5], lo: f64, hi: f64) -> Curve {
    Curve {
        degree: 4,
        knots: [vec![lo; 5], vec![hi; 5]].concat(),
        control_points: values.into_iter().map(|x| vec![x, 0., 0.]).collect(),
        weights: vec![1.; 5],
        periodic: false,
    }
}
#[test]
fn authored_affine_closed_material_has_mixed_actual_g1_g2_classes() {
    run(true, false, false, 1, false, false, false)
}
#[test]
fn guided_affine_closed_hollow_material_has_explicit_c0_and_g2_classes() {
    run(false, true, false, 2, false, false, false)
}
#[test]
fn geometric_arc_closed_material_retains_g1_jumps_and_g2_station_seam() {
    run(true, false, false, 0, true, false, false)
}

#[test]
fn nonlinear_line_image_clocks_bind_the_classified_closed_solid() {
    run(false, true, false, 0, false, true, false)
}
#[test]
fn curved_projective_image_clocks_bind_mixed_g1_g2_closed_solid() {
    run(true, false, false, 0, false, true, false)
}

#[test]
fn nonplanar_authored_closed_hollow_material_has_complete_classified_actual_solid() {
    run(false, true, false, 1, false, false, true)
}

#[test]
fn multispan_closed_hollow_material_keeps_internal_g2_joins_and_trim_ownership() {
    run_profiles(false, true, false, 0, false, false, false, true)
}
#[test]
fn multispan_nonplanar_authored_closed_material_keeps_internal_g2_joins() {
    run_profiles(false, true, false, 1, false, false, true, true)
}

#[test]
fn multispan_rational_closed_material_keeps_internal_g1_curvature_jump() {
    run_profiles(true, false, false, 0, false, false, false, true)
}

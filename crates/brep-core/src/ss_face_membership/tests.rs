use super::*;
use crate::{FaceUse, Shell, TopologyIds, trimmed_face_recipe as recipe};
use cad_predicates::ToleranceContext;
use nurbs_core::{
    curve::Curve,
    ss_intersection as ss,
    surface::{Axis, Surface},
};

fn surface(circle: bool, r2: f64) -> Surface {
    let q = [0.25, -0.25, 0.25];
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            i as f64 / 2.,
                            j as f64 / 2.,
                            if circle { q[i] + q[j] - r2 } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
fn wire(s: &Surface, rect: [[f64; 2]; 2], reverse: bool) -> Vec<recipe::Boundary> {
    let [[u0, u1], [v0, v1]] = rect;
    let mut points = vec![[u0, v0], [u1, v0], [u1, v1], [u0, v1]];
    if reverse {
        points.reverse();
    }
    (0..4)
        .map(|i| {
            let [a, b] = [points[i], points[(i + 1) % 4]];
            let (axis, fixed, lo, hi) = if a[0] == b[0] {
                (Axis::U, a[0], a[1], b[1])
            } else {
                (Axis::V, a[1], a[0], b[0])
            };
            let c = s
                .iso(axis, fixed)
                .unwrap()
                .trim(lo.min(hi), lo.max(hi))
                .unwrap();
            let mut c = if lo > hi { c.reverse().unwrap() } else { c };
            // One canonical authored position per shared UV vertex. The face
            // assembler still proves every corrected world edge continuously.
            let last = c.control_points.len() - 1;
            c.control_points[0] = s.evaluate(a[0], a[1]).unwrap().point.to_vec();
            c.control_points[last] = s.evaluate(b[0], b[1]).unwrap().point.to_vec();
            recipe::Boundary {
                curve: c,
                pcurve: Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap(),
                reversed: false,
            }
        })
        .collect()
}
fn model(s: &Surface, hole: Option<[[f64; 2]; 2]>) -> Model {
    let natural = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let mut wires = vec![wire(s, natural, false)];
    if let Some(h) = hole {
        wires.push(wire(s, h, true));
    }
    let context = ToleranceContext::default_valid();
    let report = recipe::assemble(
        &context,
        s,
        &wires,
        1e-8,
        recipe::Limits {
            pairs: 10000,
            region_cells: 100000,
            domain_cells: 100000,
            agreement_cells: 100000,
        },
    )
    .unwrap();
    let face = report
        .face
        .unwrap_or_else(|| panic!("fixture face refused: {}", report.reason));
    let mut model = Model(
        brep_topology::Model {
            vertices: face.vertices().to_vec(),
            edges: face.edges().to_vec(),
            loops: face.loops().to_vec(),
            faces: vec![face.face().clone()],
            shells: vec![Shell {
                faces: vec![FaceUse {
                    face: 0,
                    reversed: false,
                }],
                closed: false,
            }],
            bodies: vec![],
            tolerance_mm: context.spatial_bounds().on_mm,
        },
        TopologyIds::default(),
    );
    model.rebuild_topology_ids();
    model.validate().unwrap();
    model
}
fn curves(r2: f64) -> ss::QualifiedGraphCurves {
    let sources = ss::FiberSources::new(&surface(true, r2), &surface(false, 0.)).unwrap();
    let original = ss::qualify_quadratic_graph_ss(&sources, Default::default()).unwrap();
    ss::construct_graph_curves(
        &original,
        ss::GraphCurveTolerances {
            world: 1e-7,
            uv: 1e-9,
        },
        Default::default(),
    )
    .unwrap()
}
fn fixtures(c: &ss::QualifiedGraphCurves, hole: Option<[[f64; 2]; 2]>) -> [Model; 2] {
    [
        model(c.original().source().first(), hole),
        model(c.original().source().second(), None),
    ]
}
#[test]
fn complete_closed_branch_uses_actual_faces_and_preserves_every_clock() {
    let c = curves(0.0625);
    let models = fixtures(&c, None);
    let saved = models.clone();
    let owner = qualify(
        &c,
        [(&models[0], 0), (&models[1], 0)],
        1e-8,
        Default::default(),
    )
    .unwrap();
    assert_eq!(owner.edge_cells().len(), 4);
    assert_eq!(owner.vertices().len(), 4);
    for (i, cell) in owner.edge_cells().iter().enumerate() {
        assert_eq!(cell.original_edge, i);
        assert_eq!(cell.clock, [0., 1.]);
        assert_eq!(cell.locations, [Location::Inside; 2]);
    }
    owner.reverify().unwrap();
    assert_eq!(models, saved);
}
#[test]
fn hole_inside_loop_requires_whole_arc_refinement_without_excluding_the_loop() {
    let c = curves(0.0625);
    let models = fixtures(&c, Some([[0.4375, 0.5625], [0.4375, 0.5625]]));
    let owner = qualify(
        &c,
        [(&models[0], 0), (&models[1], 0)],
        1e-8,
        Default::default(),
    )
    .unwrap();
    assert!(owner.edge_cells().len() > 4);
    for edge in 0..4 {
        let cells: Vec<_> = owner
            .edge_cells()
            .iter()
            .filter(|x| x.original_edge == edge)
            .collect();
        assert_eq!(cells[0].clock[0], 0.);
        assert_eq!(cells.last().unwrap().clock[1], 1.);
        assert!(cells.windows(2).all(|p| p[0].clock[1] == p[1].clock[0]));
        assert!(cells.iter().all(|p| p.locations == [Location::Inside; 2]));
    }
    owner.reverify().unwrap();
}
#[test]
fn original_branch_in_a_hole_is_excluded_only_from_that_face() {
    let c = curves(0.0625);
    let models = fixtures(&c, Some([[0.125, 0.875], [0.125, 0.875]]));
    let owner = qualify(
        &c,
        [(&models[0], 0), (&models[1], 0)],
        1e-8,
        Default::default(),
    )
    .unwrap();
    assert!(
        owner
            .edge_cells()
            .iter()
            .all(|p| p.locations == [Location::Outside, Location::Inside])
    );
    assert!(
        owner
            .vertices()
            .iter()
            .all(|p| p.locations == [Location::Outside, Location::Inside])
    );
}
#[test]
fn actual_trim_crossings_are_refused_without_partial_clock_ownership() {
    let c = curves(0.0625);
    let models = fixtures(&c, Some([[0.5625, 0.875], [0.4375, 0.5625]]));
    let result = qualify(
        &c,
        [(&models[0], 0), (&models[1], 0)],
        1e-8,
        Limits {
            depth: 8,
            ..Default::default()
        },
    );
    assert!(result.is_err());
}
#[test]
fn unrelated_support_and_mismatched_actual_boundary_do_not_admit_membership() {
    let c = curves(0.0625);
    let mut models = fixtures(&c, None);
    models[0].faces[0].surface.control_points[1][1][2] += 0.125;
    assert!(
        qualify(
            &c,
            [(&models[0], 0), (&models[1], 0)],
            1e-8,
            Default::default()
        )
        .is_err()
    );
    let mut models = fixtures(&c, None);
    models[0].edges[0].curve.control_points[1][2] += 0.125;
    assert!(
        qualify(
            &c,
            [(&models[0], 0), (&models[1], 0)],
            1e-8,
            Default::default()
        )
        .is_err()
    );
}
#[test]
fn shared_work_limits_and_saved_report_tampering_are_checked() {
    let c = curves(0.0625);
    let models = fixtures(&c, None);
    let faces = [(&models[0], 0), (&models[1], 0)];
    for limits in [
        Limits {
            cells: 1,
            ..Default::default()
        },
        Limits {
            pairs: 1,
            ..Default::default()
        },
        Limits {
            range_operations: 1,
            ..Default::default()
        },
        Limits {
            depth: 33,
            ..Default::default()
        },
    ] {
        assert!(qualify(&c, faces, 1e-8, limits).is_err());
    }
    assert!(qualify(&c, faces, f64::NAN, Default::default()).is_err());
    let mut owner = qualify(&c, faces, 1e-8, Default::default()).unwrap();
    let used = owner.range_operations();
    assert!(
        qualify(
            &c,
            faces,
            1e-8,
            Limits {
                range_operations: used,
                ..Default::default()
            }
        )
        .is_ok()
    );
    assert!(
        qualify(
            &c,
            faces,
            1e-8,
            Limits {
                range_operations: used - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    owner.edge_cells[0].locations[0] = Location::Outside;
    assert!(owner.reverify().is_err());
}
#[test]
fn empty_point_and_coincident_strata_keep_their_original_meaning() {
    for r2 in [-0.0625, 0.] {
        let c = curves(r2);
        let models = fixtures(&c, None);
        let owner = qualify(
            &c,
            [(&models[0], 0), (&models[1], 0)],
            1e-8,
            Default::default(),
        )
        .unwrap();
        assert!(owner.edge_cells().is_empty());
        assert_eq!(owner.vertices().len(), usize::from(r2 == 0.));
    }
    let s = surface(false, 0.);
    let source = ss::FiberSources::new(&s, &s).unwrap();
    let o = ss::qualify_quadratic_graph_ss(&source, Default::default()).unwrap();
    let c = ss::construct_graph_curves(
        &o,
        ss::GraphCurveTolerances {
            world: 1e-7,
            uv: 1e-9,
        },
        Default::default(),
    )
    .unwrap();
    let models = fixtures(&c, None);
    assert!(
        qualify(
            &c,
            [(&models[0], 0), (&models[1], 0)],
            1e-8,
            Default::default()
        )
        .is_err()
    );
}
#[test]
fn face_orientation_is_retained_without_reversing_native_uv_loops() {
    let c = curves(0.0625);
    let mut models = fixtures(&c, None);
    models[1].shells[0].faces[0].reversed = true;
    let owner = qualify(
        &c,
        [(&models[0], 0), (&models[1], 0)],
        1e-8,
        Default::default(),
    )
    .unwrap();
    assert_eq!(owner.face_reversed(), [false, true]);
    assert!(
        owner
            .edge_cells()
            .iter()
            .all(|p| p.locations == [Location::Inside; 2])
    );
}

#[test]
fn independent_native_domains_and_support_swap_retain_actual_face_bindings() {
    let mut first = surface(true, 0.0625);
    let mut second = surface(false, 0.);
    first.knots_u = vec![2., 2., 2., 6., 6., 6.];
    first.knots_v = vec![-4., -4., -4., 4., 4., 4.];
    second.knots_u = vec![-3., -3., -3., 5., 5., 5.];
    second.knots_v = vec![7., 7., 7., 11., 11., 11.];
    let build = |a: &Surface, b: &Surface| {
        let source = ss::FiberSources::new(a, b).unwrap();
        let original = ss::qualify_quadratic_graph_ss(&source, Default::default()).unwrap();
        ss::construct_graph_curves(
            &original,
            ss::GraphCurveTolerances {
                world: 1e-7,
                uv: 1e-9,
            },
            Default::default(),
        )
        .unwrap()
    };
    let c = build(&first, &second);
    let models = [
        model(&first, Some([[3.75, 4.25], [-0.5, 0.5]])),
        model(&second, None),
    ];
    let owner = qualify(
        &c,
        [(&models[0], 0), (&models[1], 0)],
        1e-8,
        Default::default(),
    )
    .unwrap();
    assert!(
        owner
            .edge_cells()
            .iter()
            .all(|x| x.locations == [Location::Inside; 2])
    );
    for cell in owner.edge_cells() {
        assert!(cell.uv[0][0][0] >= 2. && cell.uv[0][0][1] <= 6.);
        assert!(cell.uv[1][1][0] >= 7. && cell.uv[1][1][1] <= 11.);
    }
    let swapped = build(&second, &first);
    let reverse = qualify(
        &swapped,
        [(&models[1], 0), (&models[0], 0)],
        1e-8,
        Default::default(),
    )
    .unwrap();
    assert_eq!(
        reverse.face_ids(),
        [owner.face_ids()[1], owner.face_ids()[0]]
    );
    assert!(
        reverse
            .edge_cells()
            .iter()
            .all(|x| x.locations == [Location::Inside; 2])
    );
    assert!(
        qualify(
            &c,
            [(&models[1], 0), (&models[0], 0)],
            1e-8,
            Default::default()
        )
        .is_err()
    );
    reverse.reverify().unwrap();
}

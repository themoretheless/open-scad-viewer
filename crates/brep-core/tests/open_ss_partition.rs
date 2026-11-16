use brep_core::{
    Coedge, Edge, Face, FaceUse, Loop, Model, Shell, Vertex, edge_partition, face_cut,
};
use nurbs_core::{
    curve::Curve,
    ss_intersection as ss,
    surface::{Axis, Surface},
};

#[path = "../examples/support/fiber_cut_fixture.rs"]
mod fixture;

fn curved(z: &[f64]) -> Surface {
    let degree = z.len() - 1;
    let mut knots = vec![0.; degree + 1];
    knots.extend(vec![1.; degree + 1]);
    Surface {
        degree_u: 1,
        degree_v: degree,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: knots,
        control_points: [0.25, 0.5]
            .into_iter()
            .map(|x| {
                z.iter()
                    .enumerate()
                    .map(|(j, &z)| vec![x, j as f64 / degree as f64, z])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; z.len()]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}

fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [0., 1.]
            .into_iter()
            .map(|x| [0., 1.].into_iter().map(|y| vec![x, y, 0.]).collect())
            .collect(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}

fn restricted(curve: &Curve, a: f64, b: f64) -> Curve {
    let right = curve.split(a).unwrap()[1].clone();
    right.split(b).unwrap()[0].clone()
}

fn trimmed_plane() -> Model {
    let surface = plane();
    let corners = [[0.25, 0.], [0.5, 0.], [0.5, 1.], [0.25, 1.]];
    let mut m = Model::empty(1e-7).unwrap();
    m.vertices = corners
        .iter()
        .map(|p| Vertex {
            point: surface.evaluate(p[0], p[1]).unwrap().point,
        })
        .collect();
    let curves = [
        restricted(&surface.iso(Axis::V, 0.).unwrap(), 0.25, 0.5),
        surface.iso(Axis::U, 0.5).unwrap(),
        restricted(&surface.iso(Axis::V, 1.).unwrap(), 0.25, 0.5),
        surface.iso(Axis::U, 0.25).unwrap(),
    ];
    let pairs = [[0, 1], [1, 2], [3, 2], [0, 3]];
    m.edges = curves
        .into_iter()
        .zip(pairs)
        .map(|(curve, vertices)| Edge {
            curve,
            vertices,
            degenerate: false,
        })
        .collect();
    m.loops.push(Loop {
        coedges: (0..4)
            .map(|i| {
                Coedge::new(
                    i,
                    i >= 2,
                    Curve::from_polyline(vec![corners[i].to_vec(), corners[(i + 1) % 4].to_vec()])
                        .unwrap(),
                )
            })
            .collect(),
    });
    m.faces.push(Face {
        surface,
        outer: 0,
        holes: vec![],
    });
    m.shells.push(Shell {
        closed: false,
        faces: vec![FaceUse {
            face: 0,
            reversed: false,
        }],
    });
    m.rebuild_topology_ids();
    m.validate().unwrap();
    m
}

fn pair(z: &[f64]) -> (Model, Model) {
    (fixture::sheet(curved(z)), trimmed_plane())
}

fn single_pair() -> (Model, Model) {
    pair(&[-0.125, -0.125, -0.125, 0.875])
}

fn prebind(
    source: &Model,
    support: usize,
    trace: &ss::QualifiedPhysicalFiberTrace,
) -> (Model, [usize; 2]) {
    let segments = trace.segments();
    let first = segments.first().unwrap();
    let last = segments.last().unwrap();
    let goals: [([f64; 3], [f64; 2]); 2] = [
        (
            first.curve().control_points[0]
                .as_slice()
                .try_into()
                .unwrap(),
            first.pcurves()[support].control_points[0]
                .as_slice()
                .try_into()
                .unwrap(),
        ),
        (
            last.curve()
                .control_points
                .last()
                .unwrap()
                .as_slice()
                .try_into()
                .unwrap(),
            last.pcurves()[support]
                .control_points
                .last()
                .unwrap()
                .as_slice()
                .try_into()
                .unwrap(),
        ),
    ];
    let mut model = source.clone();
    let mut roots = Vec::new();
    for (goal_world, goal_uv) in goals {
        let mut candidates = Vec::new();
        for (wire, l) in model.loops.iter().enumerate() {
            for (coedge, c) in l.coedges.iter().enumerate() {
                assert_eq!(c.pcurve.control_points.len(), 2);
                let p = &c.pcurve.control_points[0];
                let q = &c.pcurve.control_points[1];
                let free = if (p[0] - q[0]).abs() <= 1e-12 && (p[1] - q[1]).abs() > 1e-12 {
                    1
                } else if (p[1] - q[1]).abs() <= 1e-12 && (p[0] - q[0]).abs() > 1e-12 {
                    0
                } else {
                    continue;
                };
                let fixed = 1 - free;
                let fraction = (goal_uv[free] - p[free]) / (q[free] - p[free]);
                if (p[fixed] - goal_uv[fixed]).abs() <= 1e-12
                    && (q[fixed] - goal_uv[fixed]).abs() <= 1e-12
                    && 0. < fraction
                    && fraction < 1.
                {
                    candidates.push((c.edge, wire, coedge, fraction));
                }
            }
        }
        assert_eq!(
            candidates.len(),
            1,
            "support {support}, world {goal_world:?}, UV {goal_uv:?}"
        );
        let (edge, wire, coedge, fraction) = candidates[0];
        let c = &model.loops[wire].coedges[coedge];
        let [pa, pb] = c.pcurve.domain();
        let [ea, eb] = model.edges[edge].curve.domain();
        let edge_fraction = if c.reversed { 1. - fraction } else { fraction };
        let split = edge_partition::partition_to_vertex_and_uv(
            &model,
            edge,
            ea + (eb - ea) * edge_fraction,
            &[edge_partition::PcurveCut {
                wire,
                coedge,
                parameter: pa + (pb - pa) * fraction,
            }],
            goal_world,
            &[goal_uv],
            100000,
        )
        .unwrap();
        roots.push(split.vertex);
        model = split.model;
    }
    assert_eq!(roots.len(), 2);
    roots.sort_by(|&a, &b| model.vertices[a].point[0].total_cmp(&model.vertices[b].point[0]));
    (model, [roots[0], roots[1]])
}

fn limits() -> face_cut::OpenSsLimits {
    face_cut::OpenSsLimits {
        trace: ss::PhysicalFiberTraceLimits {
            tolerance_world: 1e-7,
            tolerance_uv: 1e-7,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn prebind_components(
    source: &Model,
    support: usize,
    traces: &[ss::QualifiedPhysicalFiberTrace],
) -> (Model, Vec<[usize; 2]>) {
    let mut model = source.clone();
    let mut endpoints = Vec::new();
    for trace in traces {
        let (next, ends) = prebind(&model, support, trace);
        model = next;
        endpoints.push(ends);
    }
    (model, endpoints)
}

fn multi_fixture() -> (
    Model,
    Model,
    Vec<ss::QualifiedPhysicalFiberTrace>,
    Model,
    Model,
    Vec<[[usize; 2]; 2]>,
) {
    let (original_a, original_b) = pair(&[0.1875, -0.3125, 0.1875]);
    let source =
        ss::FiberSources::new(&original_a.faces[0].surface, &original_b.faces[0].surface).unwrap();
    let dispatched = ss::construct_qualified_ss_branches(&source, Default::default()).unwrap();
    let ss::QualifiedSurfaceIntersection::RegularOpenArcs(owner) = &dispatched else {
        panic!("expected regular open arcs");
    };
    assert_eq!(owner.branches().len(), 2);
    let traces: Vec<_> = (0..owner.branches().len())
        .map(|branch| ss::construct_physical_fiber_trace(owner, branch, limits().trace).unwrap())
        .collect();
    let (a, ends_a) = prebind_components(&original_a, 0, &traces);
    let (b, ends_b) = prebind_components(&original_b, 1, &traces);
    let endpoints = ends_a
        .into_iter()
        .zip(ends_b)
        .map(|(a, b)| [a, b])
        .collect();
    (original_a, original_b, traces, a, b, endpoints)
}

#[test]
fn open_ss_components_partition_all_prebound_branches_without_refitting() {
    let (original_a, original_b, traces, a, b, endpoints) = multi_fixture();
    let out = face_cut::partition_open_ss_components(&a, 0, &b, 0, &endpoints, limits()).unwrap();
    assert_eq!(out.traces.len(), 2);
    assert_eq!(out.faces[0].len(), 3);
    assert_eq!(out.faces[1].len(), 3);
    for model in &out.models {
        model.validate().unwrap();
        assert!(model.persistent_naming_complete());
    }
    for (component, trace) in out.traces.iter().enumerate() {
        trace.reverify().unwrap();
        assert_eq!(trace.branch(), traces[component].branch());
        assert_eq!(trace.segments().len(), traces[component].segments().len());
        let edge_offset: usize = out.traces[..component]
            .iter()
            .map(|trace| trace.segments().len())
            .sum();
        for (local, segment) in trace.segments().iter().enumerate() {
            for support in 0..2 {
                let edge = out.edges[support][edge_offset + local];
                assert_eq!(out.models[support].edges[edge].curve, *segment.curve());
                let uses: Vec<_> = out.models[support]
                    .loops
                    .iter()
                    .flat_map(|l| &l.coedges)
                    .filter(|c| c.edge == edge)
                    .collect();
                assert_eq!(uses.len(), 2);
                for c in uses {
                    let expected = if c.reversed {
                        segment.pcurves()[support].reverse().unwrap()
                    } else {
                        segment.pcurves()[support].clone()
                    };
                    assert_eq!(c.pcurve, expected);
                }
            }
        }
        for support in 0..2 {
            let first = out.edges[support][edge_offset];
            let last = out.edges[support][edge_offset + trace.segments().len() - 1];
            assert_eq!(
                out.models[support].edges[first].vertices[0],
                endpoints[component][support][0]
            );
            assert_eq!(
                out.models[support].edges[last].vertices[1],
                endpoints[component][support][1]
            );
        }
    }
    for support in 0..2 {
        let expected: usize = traces.iter().map(|trace| trace.segments().len()).sum();
        assert_eq!(out.edges[support].len(), expected);
    }
    assert_eq!(original_a, pair(&[0.1875, -0.3125, 0.1875]).0);
    assert_eq!(original_b, pair(&[0.1875, -0.3125, 0.1875]).1);
}

#[test]
fn open_ss_components_refuse_bad_bindings_and_shared_budget_exhaustion() {
    let (_, _, traces, a, b, endpoints) = multi_fixture();
    let before = (a.clone(), b.clone());
    assert!(
        face_cut::partition_open_ss_components(&a, 0, &b, 0, &endpoints[..1], limits()).is_err()
    );
    let reversed = [endpoints[1], endpoints[0]];
    assert!(face_cut::partition_open_ss_components(&a, 0, &b, 0, &reversed, limits()).is_err());
    let segments: usize = traces.iter().map(|trace| trace.segments().len()).sum();
    assert!(segments > 1);
    let cells: usize = traces
        .iter()
        .map(|trace| {
            trace.cells()
                + trace
                    .reverify_with_budget(limits().trace.max_cells)
                    .unwrap()
        })
        .sum();
    assert!(
        face_cut::partition_open_ss_components(
            &a,
            0,
            &b,
            0,
            &endpoints,
            face_cut::OpenSsLimits {
                trace: ss::PhysicalFiberTraceLimits {
                    max_cells: cells - 1,
                    ..limits().trace
                },
                ..limits()
            },
        )
        .is_err()
    );
    assert!(
        face_cut::partition_open_ss_components(
            &a,
            0,
            &b,
            0,
            &endpoints,
            face_cut::OpenSsLimits {
                trace: ss::PhysicalFiberTraceLimits {
                    max_segments: segments - 1,
                    ..limits().trace
                },
                ..limits()
            },
        )
        .is_err()
    );
    assert_eq!((&a, &b), (&before.0, &before.1));
    assert!(face_cut::partition_open_ss(&a, 0, &b, 0, endpoints[0], limits()).is_err());
}

#[test]
fn open_ss_trace_partitions_two_prebound_sheets_without_refitting() {
    let (original_a, original_b) = single_pair();
    let source =
        ss::FiberSources::new(&original_a.faces[0].surface, &original_b.faces[0].surface).unwrap();
    let dispatched = ss::construct_qualified_ss_branches(&source, Default::default()).unwrap();
    let ss::QualifiedSurfaceIntersection::RegularOpenArcs(owner) = &dispatched else {
        panic!("expected regular open arcs");
    };
    let trace = ss::construct_physical_fiber_trace(owner, 0, limits().trace).unwrap();

    let (a, ends_a) = prebind(&original_a, 0, &trace);
    let (b, ends_b) = prebind(&original_b, 1, &trace);
    let out = face_cut::partition_open_ss(&a, 0, &b, 0, [ends_a, ends_b], limits()).unwrap();
    out.first.model.validate().unwrap();
    out.second.model.validate().unwrap();
    assert!(out.first.model.persistent_naming_complete());
    assert!(out.second.model.persistent_naming_complete());
    assert!(out.trace.segments().len() > 1);
    assert_eq!(out.first.edges.len(), out.trace.segments().len());
    assert_eq!(out.second.edges.len(), out.trace.segments().len());
    for (i, segment) in out.trace.segments().iter().enumerate() {
        let ea = out.first.edges[i];
        let eb = out.second.edges[i];
        assert_eq!(out.first.model.edges[ea].curve, *segment.curve());
        assert_eq!(out.second.model.edges[eb].curve, *segment.curve());
        assert_eq!(
            out.first.model.edges[ea].curve,
            out.second.model.edges[eb].curve
        );
        for (model, edge, support) in [(&out.first.model, ea, 0), (&out.second.model, eb, 1)] {
            let uses: Vec<_> = model
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter(|c| c.edge == edge)
                .collect();
            assert_eq!(uses.len(), 2);
            for c in uses {
                let expected = if c.reversed {
                    segment.pcurves()[support].reverse().unwrap()
                } else {
                    segment.pcurves()[support].clone()
                };
                assert_eq!(c.pcurve, expected);
            }
        }
    }
    assert_eq!(
        out.first.model.edges[out.first.edges[0]].vertices[0],
        ends_a[0]
    );
    assert_eq!(
        out.first.model.edges[*out.first.edges.last().unwrap()].vertices[1],
        ends_a[1]
    );
    assert_eq!(
        out.second.model.edges[out.second.edges[0]].vertices[0],
        ends_b[0]
    );
    assert_eq!(
        out.second.model.edges[*out.second.edges.last().unwrap()].vertices[1],
        ends_b[1]
    );
    out.trace.reverify().unwrap();
    assert_eq!(original_a, single_pair().0);
    assert_eq!(original_b, single_pair().1);
}

#[test]
fn open_ss_refuses_wrong_endpoints_budgets_and_extra_branches() {
    let (original_a, original_b) = single_pair();
    let source =
        ss::FiberSources::new(&original_a.faces[0].surface, &original_b.faces[0].surface).unwrap();
    let dispatched = ss::construct_qualified_ss_branches(&source, Default::default()).unwrap();
    let ss::QualifiedSurfaceIntersection::RegularOpenArcs(owner) = &dispatched else {
        panic!("expected regular open arcs");
    };
    let trace = ss::construct_physical_fiber_trace(owner, 0, limits().trace).unwrap();
    let (a, ends_a) = prebind(&original_a, 0, &trace);
    let (b, ends_b) = prebind(&original_b, 1, &trace);
    let before = (a.clone(), b.clone());
    assert!(
        face_cut::partition_open_ss(&a, 0, &b, 0, [[ends_a[1], ends_a[0]], ends_b], limits(),)
            .is_err()
    );
    assert!(
        face_cut::partition_open_ss(
            &a,
            0,
            &b,
            0,
            [ends_a, ends_b],
            face_cut::OpenSsLimits {
                cut_cells: 0,
                ..limits()
            },
        )
        .is_err()
    );
    assert!(
        face_cut::partition_open_ss(
            &a,
            0,
            &b,
            0,
            [ends_a, ends_b],
            face_cut::OpenSsLimits {
                max_exact: 1000001,
                ..limits()
            },
        )
        .is_err()
    );
    assert!(face_cut::partition_open_ss(&a, 0, &a, 0, [ends_a, ends_a], limits()).is_err());
    assert!(
        face_cut::partition_open_ss(
            &a,
            0,
            &b,
            0,
            [ends_a, ends_b],
            face_cut::OpenSsLimits {
                trace: ss::PhysicalFiberTraceLimits {
                    max_segments: 0,
                    ..limits().trace
                },
                ..limits()
            },
        )
        .is_err()
    );
    assert_eq!((a, b), before);

    let (multi_a, multi_b) = pair(&[0.1875, -0.3125, 0.1875]);
    assert!(face_cut::partition_open_ss(&multi_a, 0, &multi_b, 0, [[0, 1]; 2], limits()).is_err());
}

#[test]
fn open_ss_components_automatically_bind_original_boundaries() {
    let (a, b) = pair(&[0.1875, -0.3125, 0.1875]);
    let before = (a.clone(), b.clone());
    let out = face_cut::partition_open_ss_components_automatic(&a, 0, &b, 0, limits()).unwrap();
    assert_eq!(out.traces.len(), 2);
    assert_eq!(out.endpoints.len(), 2);
    assert_eq!(out.faces[0].len(), 3);
    assert_eq!(out.faces[1].len(), 3);
    assert!(out.binding_cells[0] > 0);
    assert!(out.binding_cells[1] > 0);
    for support in 0..2 {
        out.models[support].validate().unwrap();
        assert!(out.models[support].persistent_naming_complete());
        for (component, trace) in out.traces.iter().enumerate() {
            let first = out.edges[support][out.traces[..component]
                .iter()
                .map(|t| t.segments().len())
                .sum::<usize>()];
            let last = out.edges[support][out.traces[..=component]
                .iter()
                .map(|t| t.segments().len())
                .sum::<usize>()
                - 1];
            let ends = out.endpoints[component][support];
            assert_eq!(out.models[support].edges[first].vertices[0], ends[0]);
            assert_eq!(out.models[support].edges[last].vertices[1], ends[1]);
            assert_eq!(
                out.models[support].vertices[ends[0]].point.as_slice(),
                trace.segments()[0].curve().control_points[0]
            );
            assert_eq!(
                out.models[support].vertices[ends[1]].point.as_slice(),
                trace
                    .segments()
                    .last()
                    .unwrap()
                    .curve()
                    .control_points
                    .last()
                    .unwrap()
            );
        }
    }
    assert_eq!((a, b), before);
}

#[test]
fn open_ss_automatic_binding_supports_nonlinear_pcurves_and_bounds_inversions() {
    let (a, b) = pair(&[0.1875, -0.3125, 0.1875]);
    for binding_cells in [0, 1] {
        assert!(
            face_cut::partition_open_ss_components_automatic(
                &a,
                0,
                &b,
                0,
                face_cut::OpenSsLimits {
                    binding_cells,
                    ..limits()
                },
            )
            .is_err()
        );
    }

    let (_, _, _, bound_a, bound_b, endpoints) = multi_fixture();
    assert!(
        face_cut::partition_open_ss_components(
            &bound_a,
            0,
            &bound_b,
            0,
            &endpoints,
            face_cut::OpenSsLimits {
                binding_cells: 0,
                binding_inversions: 0,
                ..limits()
            },
        )
        .is_ok()
    );
    let rebound = face_cut::partition_open_ss_components_automatic(
        &bound_a,
        0,
        &bound_b,
        0,
        face_cut::OpenSsLimits {
            binding_cells: 1,
            binding_inversions: 1,
            ..limits()
        },
    )
    .unwrap();
    assert_eq!(rebound.binding_cells, [0, 0]);
    assert_eq!(rebound.binding_inversions, [0, 0]);
    assert_eq!(rebound.endpoints, endpoints);

    let (mut nonlinear, plane) = single_pair();
    for wire in &mut nonlinear.loops {
        for c in &mut wire.coedges {
            let old = c.pcurve.clone();
            let [a, b] = old.domain();
            let p = old.control_points[0].clone();
            let q = old.control_points[1].clone();
            let middle = p
                .iter()
                .zip(&q)
                .map(|(&a, &b)| a + (b - a) * 0.5)
                .collect();
            c.pcurve = Curve {
                degree: 2,
                knots: vec![a, a, a, b, b, b],
                control_points: vec![p, middle, q],
                weights: vec![1.; 3],
                periodic: false,
            };
        }
    }
    nonlinear.validate().unwrap();
    let before = nonlinear.clone();
    assert!(
        face_cut::partition_open_ss_components_automatic(
            &nonlinear,
            0,
            &plane,
            0,
            face_cut::OpenSsLimits {
                binding_inversions: 1,
                ..limits()
            },
        )
        .is_err()
    );
    let out = face_cut::partition_open_ss_components_automatic(&nonlinear, 0, &plane, 0, limits())
        .unwrap();
    assert_eq!(out.traces.len(), 1);
    assert_eq!(out.faces[0].len(), 2);
    assert_eq!(out.faces[1].len(), 2);
    assert!(out.binding_inversions[0] >= 2);
    assert_eq!(out.binding_inversions[1], 0);
    for model in &out.models {
        model.validate().unwrap();
        assert!(model.persistent_naming_complete());
    }
    assert_eq!(nonlinear, before);

    let (explicit_windows, plane) = single_pair();
    let mut split_cases = 0;
    for slot in 0..explicit_windows.loops[0].coedges.len() {
        let mut trimmed = explicit_windows.clone();
        let c = trimmed.loops[0].coedges[slot].clone();
        let window = brep_topology::CoedgeTrim {
            curve_parameter: trimmed.edges[c.edge].curve.domain(),
            pcurve_parameter: c.pcurve.domain(),
            periodic_lift: [[0; 2]; 2],
        };
        trimmed.loops[0].coedges[slot] = c.with_trim(window).unwrap();
        trimmed.validate().unwrap();
        let before = trimmed.clone();
        let out = face_cut::partition_open_ss_components_automatic(
            &trimmed, 0, &plane, 0, limits(),
        )
        .unwrap();
        let retained: Vec<_> = out.models[0]
            .loops
            .iter()
            .flat_map(|wire| &wire.coedges)
            .filter(|c| c.trim().is_some())
            .collect();
        match retained.len() {
            1 => {}
            2 => split_cases += 1,
            count => panic!("explicit trim was duplicated or lost: {count} descendants"),
        }
        for c in retained {
            let trim = c.trim().unwrap();
            assert_eq!(
                trim.curve_parameter,
                out.models[0].edges[c.edge].curve.domain()
            );
            assert_eq!(trim.pcurve_parameter, c.pcurve.domain());
        }
        assert!(
            brep_core::boundary_agreement::verify(&out.models[0], 100000)
                .unwrap()
                .complete
        );
        assert_eq!(trimmed, before);
    }
    assert!(split_cases > 0, "test must exercise a real trimmed split");

    let mut partial = explicit_windows.clone();
    let c = partial.loops[0].coedges[0].clone();
    let domain = c.pcurve.domain();
    let mut window = brep_topology::CoedgeTrim {
        curve_parameter: partial.edges[c.edge].curve.domain(),
        pcurve_parameter: domain,
        periodic_lift: [[0; 2]; 2],
    };
    window.pcurve_parameter[1] = domain[0] + (domain[1] - domain[0]) * 0.75;
    let curve = partial.edges[c.edge].curve.domain();
    window.curve_parameter[1] = curve[0] + (curve[1] - curve[0]) * 0.75;
    partial.loops[0].coedges[0] = c.with_trim(window).unwrap();
    partial.validate().unwrap();
    let before = partial.clone();
    assert!(
        face_cut::partition_open_ss_components_automatic(&partial, 0, &plane, 0, limits()).is_err()
    );
    assert_eq!(partial, before);
}

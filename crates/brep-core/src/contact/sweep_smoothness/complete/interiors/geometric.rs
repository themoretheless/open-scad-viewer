//! Exact knot-line transport through a partition of the original tensor chart.
use super::super::{graph_jets, periodic_uses};
use crate::{Result, Surface};
use nurbs_core::{
    continuity::ProjectiveSeam, editing::exact_surface_knot_insertion::tile_tensor_surface,
};

fn cuts(knots: &[f64], degree: usize, count: usize, order: usize) -> Option<Vec<f64>> {
    let [lo, hi] = [knots[degree], knots[count]];
    if knots[..=degree].iter().any(|&k| k != lo) || knots[count..].iter().any(|&k| k != hi) {
        return None;
    }
    let mut cuts = Vec::new();
    for block in knots
        .chunk_by(|a, b| a == b)
        .filter(|k| lo < k[0] && k[0] < hi)
    {
        if block.len() > degree {
            return None;
        }
        if degree < block.len() + order {
            cuts.push(block[0]);
        }
    }
    (cuts.len() <= 254).then_some(cuts)
}

pub(super) struct Report {
    pub(super) certified: bool,
    pub(super) cells: usize,
    pub(super) work: u64,
    pub(super) seams: usize,
}

pub(super) fn inspect(surface: &Surface, order: usize, cells: usize, work: u64) -> Result<Report> {
    let mut report = Report {
        certified: false,
        cells: 0,
        work: 0,
        seams: 0,
    };
    if work == 0 || cells == 0 {
        return Ok(report);
    }
    let wraps = [surface.periodic_u, surface.periodic_v];
    let (surface, used) = periodic_uses::open(surface, work)?;
    report.work = used;
    let Some(surface) = surface else {
        return Ok(report);
    };
    inspect_opened(&surface, wraps, order, cells, work - report.work).map(|mut proof| {
        proof.work += report.work;
        proof
    })
}

pub(super) fn inspect_opened(
    surface: &Surface, wraps: [bool; 2], order: usize, cells: usize, work: u64,
) -> Result<Report> {
    let mut report = Report { certified: false, cells: 0, work: 0, seams: 0 };
    if work == 0 || cells == 0 {
        return Ok(report);
    }
    let (Some(u), Some(v)) = (
        cuts(
            &surface.knots_u,
            surface.degree_u,
            surface.control_points.len(),
            order,
        ),
        cuts(
            &surface.knots_v,
            surface.degree_v,
            surface.control_points[0].len(),
            order,
        ),
    ) else {
        return Ok(report);
    };
    let [columns, rows] = [u.len() + 1, v.len() + 1];
    let count = rows * columns;
    let seam_count =
        rows * (u.len() + usize::from(wraps[0])) + columns * (v.len() + usize::from(wraps[1]));
    if seam_count == 0 || count > cells || count > 4096 || seam_count > 4096 {
        return Ok(report);
    }
    report.cells = count;
    let patches = if count == 1 {
        vec![surface.clone()]
    } else {
        let remaining = (work - report.work).min(cad_predicates::MAX_WORK);
        if remaining == 0 {
            return Ok(report);
        }
        let tiled = tile_tensor_surface(surface, &u, &v, remaining)?;
        report.work += tiled.work_used;
        let Some(patches) = tiled.patches else {
            return Ok(report);
        };
        patches
    };

    let mut seams = Vec::with_capacity(seam_count);
    let mut join = |a, b, ab: &str, bb: &str| {
        seams.push(ProjectiveSeam {
            patches: [a, b],
            boundaries: [ab.into(), bb.into()],
            order,
            normal_scale: 1.,
        });
    };
    for row in 0..rows {
        for column in 0..columns {
            let index = row * columns + column;
            if column + 1 < columns {
                join(index, index + 1, "uMax", "uMin");
            } else if wraps[0] {
                join(index, row * columns, "uMax", "uMin");
            }
            if row + 1 < rows {
                join(index, index + columns, "vMax", "vMin");
            } else if wraps[1] {
                join(index, column, "vMax", "vMin");
            }
        }
    }
    let proof = graph_jets::qualify_with_chart_proof(
        &patches,
        &seams,
        work - report.work,
        graph_jets::ChartProof::Parametric,
    )?;
    report.work += proof.exact_work;
    report.certified = proof.certified;
    if proof.certified {
        report.seams = seams.len();
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saddle() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 0.25, 1., 1.],
            knots_v: vec![0., 0., 0.5, 1., 1.],
            control_points: [0., 1., 3.]
                .into_iter()
                .map(|x| {
                    [0., 2., 3.]
                        .into_iter()
                        .map(|y| vec![x, y, x * y / 8.])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }

    #[test]
    fn crossing_internal_knot_lines_keep_g2_with_exact_nondyadic_speed_ratios() {
        let source = saddle();
        let mut model = crate::cuboid([0.; 3], [3.; 3]).unwrap();
        model.faces[0].surface = source.clone();
        let raw =
            nurbs_core::analysis::surface_continuity::qualify(&source, 2, 1000, 1_000_000).unwrap();
        assert!(raw.certificate.is_none());
        let report = super::super::inspect(&model, 2, 1000, 1_000_000).unwrap();
        assert!(report.certified, "face={:?}", report.unresolved_face);
        assert_eq!(report.geometric_seams, 4);
        assert_eq!(model.faces[0].surface, source);

        for (cells, work) in [
            (report.cells - 1, report.exact_work),
            (report.cells, report.exact_work - 1),
        ] {
            let short = super::super::inspect(&model, 2, cells, work).unwrap();
            assert!(!short.certified);
            assert_eq!(short.unresolved_face, Some(0));
            assert!(short.cells <= cells && short.exact_work <= work);
        }
        model.faces[0].surface.control_points[1][1][2] += 1. / 16.;
        let kink = super::super::inspect(&model, 1, 1000, 1_000_000).unwrap();
        assert!(!kink.certified);
        assert_eq!(kink.geometric_seams, 0);
        assert_eq!(kink.unresolved_face, Some(0));
    }
    #[test]
    fn curved_graph_has_g2_across_a_jump_in_tangential_acceleration() {
        // x(u) is a quadratic spline with a jump in its second derivative.
        // The physical graph z = xy/8 has the same curvature on both sides.
        let mut surface = saddle();
        surface.degree_u = 2;
        surface.knots_u = vec![0., 0., 0., 0.5, 1., 1., 1.];
        surface.knots_v = vec![0., 0., 1., 1.];
        surface.control_points = [0., 0.5, 1.75, 2.]
            .into_iter()
            .map(|x| {
                [0., 2.]
                    .into_iter()
                    .map(|y| vec![x, y, x * y / 8.])
                    .collect()
            })
            .collect();
        surface.weights = vec![vec![1.; 2]; 4];
        let report = inspect(&surface, 2, 100, 1_000_000).unwrap();
        assert!(report.certified);
        assert_eq!(report.seams, 1);
        assert!(
            !inspect(&surface, 2, 100, report.work - 1)
                .unwrap()
                .certified
        );
        // Changing the second-derivative field while preserving the common
        // point and first derivative at the knot removes geometric G2.
        surface.control_points[3][1][2] += 1. / 16.;
        assert!(!inspect(&surface, 2, 100, 1_000_000).unwrap().certified);
    }

    fn periodic_chart(torus: bool) -> Surface {
        let ring = [
            [1., -1.],
            [1., 0.],
            [1., 1.],
            [0., 1.],
            [-1., 1.],
            [-1., 0.],
            [-1., -1.],
            [0., -1.],
            [1., -1.],
            [1., 0.],
        ];
        let weights = [0.5, 1., 0.5, 1., 0.5, 1., 0.5, 1., 0.5, 1.];
        let periodic_knots = vec![-1., -1., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 5.];
        let (control_points, weights_v) = if torus {
            let rows = ring
                .iter()
                .map(|&[x, y]| {
                    ring.iter()
                        .map(|&[c, s]| vec![x * (4. + c), y * (4. + c), s])
                        .collect()
                })
                .collect();
            (rows, weights.to_vec())
        } else {
            let rows = ring
                .iter()
                .map(|&[x, y]| vec![vec![x, y, 0.], vec![x, y, 2.]])
                .collect();
            (rows, vec![1., 1.])
        };
        Surface {
            degree_u: 2,
            degree_v: if torus { 2 } else { 1 },
            knots_u: periodic_knots.clone(),
            knots_v: if torus {
                periodic_knots
            } else {
                vec![0., 0., 1., 1.]
            },
            control_points,
            weights: weights
                .iter()
                .map(|&u| weights_v.iter().map(|v| u * v).collect())
                .collect(),
            periodic_u: true,
            periodic_v: torus,
        }
    }

    #[test]
    fn periodic_original_face_covers_internal_knots_and_every_wrap_join() {
        for (torus, expected) in [(false, 4), (true, 32)] {
            let source = periodic_chart(torus);
            let proof = inspect(&source, 2, 1000, 16_000_000).unwrap();
            assert!(proof.certified, "torus={torus} work={}", proof.work);
            assert_eq!(proof.seams, expected);
            for (cells, work) in [(proof.cells - 1, proof.work), (proof.cells, proof.work - 1)] {
                let short = inspect(&source, 2, cells, work).unwrap();
                assert!(!short.certified);
                assert_eq!(short.seams, 0);
                assert!(short.cells <= cells && short.work <= work);
            }
            let mut model = crate::cuboid([0.; 3], [3.; 3]).unwrap();
            model.faces[0].surface = source;
            let report = super::super::inspect(&model, 2, 1000, 16_000_000).unwrap();
            assert!(report.certified);
            assert_eq!(report.geometric_seams, expected);
        }
    }

    #[test]
    fn periodic_flag_cannot_hide_a_kink_across_the_opened_period() {
        let mut source = periodic_chart(false);
        for row in [0, 8] {
            for point in &mut source.control_points[row] {
                point[0] += 0.125;
            }
        }
        let report = inspect(&source, 1, 1000, 16_000_000).unwrap();
        assert!(!report.certified);
        assert_eq!(report.seams, 0);
        assert!(report.work > 0);
    }
    fn single_face_torus(surface: Surface) -> crate::Model {
        use crate::{Body, Coedge, Curve, Edge, Face, FaceUse, Loop, Shell, Vertex};
        let world_u = Curve {
            degree: surface.degree_u,
            knots: surface.knots_u.clone(),
            control_points: surface
                .control_points
                .iter()
                .map(|row| row[1].clone())
                .collect(),
            weights: surface.weights.iter().map(|row| row[1]).collect(),
            periodic: true,
        };
        let world_v = Curve {
            degree: surface.degree_v,
            knots: surface.knots_v.clone(),
            control_points: surface.control_points[1].clone(),
            weights: surface.weights[1].clone(),
            periodic: true,
        };
        let point = [
            surface.control_points[1][1][0],
            surface.control_points[1][1][1],
            surface.control_points[1][1][2],
        ];
        let curve_use = |edge, reversed, a, b| Coedge::new(edge, reversed, crate::line(a, b));
        let mut model = crate::Model(
            brep_topology::Model {
                vertices: vec![Vertex { point }],
                edges: [world_u, world_v]
                    .into_iter()
                    .map(|curve| Edge {
                        curve,
                        vertices: [0, 0],
                        degenerate: false,
                    })
                    .collect(),
                loops: vec![Loop {
                    coedges: vec![
                        curve_use(0, false, vec![0., 0.], vec![4., 0.]),
                        curve_use(1, false, vec![4., 0.], vec![4., 4.]),
                        curve_use(0, true, vec![4., 4.], vec![0., 4.]),
                        curve_use(1, true, vec![0., 4.], vec![0., 0.]),
                    ],
                }],
                faces: vec![Face {
                    surface,
                    outer: 0,
                    holes: vec![],
                }],
                shells: vec![Shell {
                    faces: vec![FaceUse {
                        face: 0,
                        reversed: false,
                    }],
                    closed: true,
                }],
                bodies: vec![Body {
                    outer_shell: 0,
                    inner_shells: vec![],
                }],
                tolerance_mm: 1e-8,
            },
            crate::TopologyIds::default(),
        );
        model.rebuild_topology_ids();
        model
    }

    #[test]
    fn complete_boundary_owns_both_topological_wrap_edges_of_one_periodic_face() {
        let mut model = single_face_torus(periodic_chart(true));
        let limits = super::super::super::Limits {
            order: 2,
            exact_work: 16_000_000,
            agreement_work: 1_000_000,
            regularity_cells: 100_000,
        };
        let report = super::super::super::inspect(&model, limits).unwrap();
        assert!(
            report.g2_certified,
            "regular={} face={} position={:?} unresolved={:?} links={}",
            report.regularity.all_faces_regular,
            report.face_interiors.certified,
            report
                .edge_correspondence
                .as_ref()
                .map(|r| (r.all_equal, r.all_joins_exact)),
            report.unresolved_edges,
            report.vertex_links_complete,
        );
        assert_eq!(report.periodic_edge_ids, [0, 1]);
        assert!(report.edge_ids.is_empty());
        assert_eq!(report.face_interiors.geometric_seams, 32);
        let short = super::super::super::inspect(
            &model,
            super::super::super::Limits {
                exact_work: report.exact_work - 1,
                ..limits
            },
        )
        .unwrap();
        assert!(!short.g2_certified && short.periodic_edge_ids.is_empty());
        assert_eq!(short.unresolved_edges, [0, 1]);

        let mut trimmed = model.clone();
        let range = trimmed.edges[1].curve.domain();
        let coedge = &mut trimmed.loops[0].coedges[1];
        *coedge = crate::Coedge::new(1, false, crate::Curve {
            degree: 2,
            knots: vec![0., 0., 0., 4., 4., 4.],
            control_points: vec![vec![4., -4.], vec![4., 0.], vec![4., 12.]],
            weights: vec![1., 2., 3.],
            periodic: false,
        }).with_trim(brep_topology::CoedgeTrim {
            curve_parameter: range,
            pcurve_parameter: [1., 2.],
            periodic_lift: [[1, 0], [1, 1]],
        }).unwrap();
        trimmed.validate().unwrap();
        let active = super::super::super::inspect(&trimmed, limits).unwrap();
        assert!(active.g2_certified && active.parameter_work > 0,
            "unresolved={:?} agreement={:?}", active.unresolved_edges,
            active.edge_correspondence.as_ref().map(|r| (r.all_equal, r.all_joins_exact)));
        assert_eq!(active.periodic_edge_ids, [0, 1]);
        use crate::sweep_smoothness::geometric::{Tolerances, inspect_edges_with_exact};
        let tolerances = Tolerances { position: 0., normal: 0., curvature: 0. };
        let edge = inspect_edges_with_exact(&trimmed, &[1], tolerances, 1, 1_000_000).unwrap();
        assert!(edge.within_tolerance && edge.complete);
        assert!(edge.seams[0].positions.as_ref().unwrap().proven);
        assert_eq!(edge.seams[0].inspection.as_ref().unwrap().position_bounds, [0.; 2]);
        let short_edge = inspect_edges_with_exact(
            &trimmed, &[1], tolerances, 1, edge.exact_work - 1,
        ).unwrap();
        assert!(!short_edge.within_tolerance);
        let mut wrong_sheet = trimmed.clone();
        let c = &mut wrong_sheet.loops[0].coedges[1];
        let mut trim = c.trim().unwrap();
        trim.periodic_lift[1][1] -= 1;
        *c = c.clone().with_trim(trim).unwrap();
        let wrong = inspect_edges_with_exact(&wrong_sheet, &[1], tolerances, 1, 1_000_000).unwrap();
        assert!(!wrong.within_tolerance);

        let short = super::super::super::inspect(&trimmed, super::super::super::Limits {
            exact_work: active.exact_work - 1, ..limits
        }).unwrap();
        assert!(!short.g2_certified);
        use super::super::super::classified;
        let classified = classified::inspect(
            &trimmed, &[classified::Class::G1, classified::Class::G2],
            classified::Limits {
                incidence_steps: 100, agreement_work: limits.agreement_work,
                regularity_cells: limits.regularity_cells, exact_work: limits.exact_work,
            },
        ).unwrap();
        assert!(classified.certified, "{}", classified.reason);

        let coedge = &mut model.loops[0].coedges[0];
        *coedge = coedge
            .clone()
            .with_trim(brep_topology::CoedgeTrim {
                curve_parameter: [1., 3.],
                pcurve_parameter: [0.25, 0.75],
                periodic_lift: [[0; 2]; 2],
            })
            .unwrap();
        let partial = super::super::super::inspect(&model, limits).unwrap();
        assert!(!partial.g2_certified);
        assert!(partial.unresolved_edges.contains(&0));
    }
    #[test]
    fn g2_uses_the_exact_active_interval_of_a_bent_periodic_shore() {
        use crate::sweep_smoothness::geometric::{Tolerances, inspect_edges_with_exact};
        let model = single_face_torus(periodic_chart(true));
        let mut trimmed = model.clone();
        let range = trimmed.edges[1].curve.domain();
        let coedge = &mut trimmed.loops[0].coedges[1];
        *coedge = crate::Coedge::new(1, false, crate::Curve {
            degree: 1,
            knots: vec![0., 0., 1., 2., 4., 4.],
            control_points: vec![vec![3., -1.], vec![4., 0.], vec![4., 4.], vec![3., 5.]],
            weights: vec![1.; 4],
            periodic: false,
        }).with_trim(brep_topology::CoedgeTrim {
            curve_parameter: range,
            pcurve_parameter: [1., 2.],
            periodic_lift: [[1, 0], [1, 1]],
        }).unwrap();
        trimmed.validate().unwrap();
        let report = super::super::super::inspect(&trimmed, super::super::super::Limits {
            order: 2, exact_work: 16_000_000, agreement_work: 1_000_000, regularity_cells: 100_000,
        }).unwrap();
        assert!(report.g2_certified, "{:?}", report.unresolved_edges);
        let tolerances = Tolerances { position: 0., normal: 0., curvature: 0. };
        let selected = inspect_edges_with_exact(&trimmed, &[1], tolerances, 1, 1_000_000).unwrap();
        assert!(selected.complete && selected.within_tolerance);
        let limited = inspect_edges_with_exact(&trimmed, &[1], tolerances, 1, selected.exact_work - 1).unwrap();
        assert!(!limited.within_tolerance);

        use super::super::super::classified::{self, Class};
        let mixed = classified::inspect(
            &trimmed, &[Class::C0, Class::G2],
            classified::Limits {
                incidence_steps: 100, exact_work: 16_000_000,
                agreement_work: 1_000_000, regularity_cells: 100_000,
            },
        ).unwrap();
        assert!(mixed.certified, "{}", mixed.reason);
        assert_eq!(mixed.c0_edges, [0]);
        assert_eq!(mixed.smooth_edges, [1]);
    }

    #[test]
    fn selected_g2_joins_two_distinct_periodic_wall_faces() {
        use crate::sweep_smoothness::geometric::{Tolerances, inspect_edges_with_exact};
        use crate::{Coedge, Curve, Edge, Face, FaceUse, Loop, Shell, Vertex};
        let lower = periodic_chart(false);
        let mut upper = lower.clone();
        for p in upper.control_points.iter_mut().flatten() {
            p[2] += 2.;
        }
        let rim = |z: f64| Curve {
            degree: lower.degree_u,
            knots: lower.knots_u.clone(),
            control_points: lower.control_points.iter().map(|row| {
                vec![row[0][0], row[0][1], z]
            }).collect(),
            weights: lower.weights.iter().map(|row| row[0]).collect(),
            periodic: true,
        };
        let stations = [0., 2., 4.];
        let mut edges: Vec<_> = stations.into_iter().enumerate().map(|(i, z)| {
            Edge { curve: rim(z), vertices: [i, i], degenerate: false }
        }).collect();
        for i in 0..2 {
            edges.push(Edge {
                curve: crate::line(vec![1., 0., stations[i]], vec![1., 0., stations[i + 1]]),
                vertices: [i, i + 1],
                degenerate: false,
            });
        }
        let wire = |i| Loop {
            coedges: [
                (i, false, [0., 0.], [4., 0.]),
                (i + 3, false, [4., 0.], [4., 1.]),
                (i + 1, true, [4., 1.], [0., 1.]),
                (i + 3, true, [0., 1.], [0., 0.]),
            ].map(|(edge, rev, a, b)| Coedge::new(edge, rev, crate::line(a.to_vec(), b.to_vec()))).to_vec(),
        };
        let mut model = crate::Model(brep_topology::Model {
            vertices: stations.map(|z| Vertex { point: [1., 0., z] }).to_vec(),
            edges,
            loops: vec![wire(0), wire(1)],
            faces: vec![
                Face { surface: lower, outer: 0, holes: vec![] },
                Face { surface: upper, outer: 1, holes: vec![] },
            ],
            shells: vec![Shell { closed: false, faces: vec![
                FaceUse { face: 0, reversed: false }, FaceUse { face: 1, reversed: false },
            ] }],
            bodies: vec![],
            tolerance_mm: 1e-8,
        }, crate::TopologyIds::default());
        model.rebuild_topology_ids();
        model.validate().unwrap();
        let t = Tolerances { position: 0., normal: 0., curvature: 0. };
        let report = inspect_edges_with_exact(&model, &[1], t, 1, 1_000_000).unwrap();
        assert!(report.within_tolerance && report.seams[0].faces == [0, 1]);
        assert_eq!(report.seams[0].inspection.as_ref().unwrap().orientation,
            Some(nurbs_core::surface_g2::Orientation::Same));
        assert!(!inspect_edges_with_exact(&model, &[1], t, 1, report.exact_work - 1)
            .unwrap().within_tolerance);
        let limits = super::super::super::Limits {
            order: 2, exact_work: 1_000_000, agreement_work: 1_000_000, regularity_cells: 100_000,
        };
        let complete = super::super::super::inspect(&model, limits).unwrap();
        let index = complete.edge_ids.iter().position(|&edge| edge == 1).unwrap();
        assert!(complete.proof.seams[index].certified);
        assert!(complete.parameter_work > 0);
        assert!(!complete.extraction_complete, "the two end rims are open");
        let zero = super::super::super::inspect(&model, super::super::super::Limits {
            exact_work: 0, ..limits
        }).unwrap();
        assert!(zero.unresolved_edges.contains(&1));

        let range = model.edges[1].curve.domain();
        model.loops[1].coedges[0] = Coedge::new(1, false, Curve {
            degree: 1,
            knots: vec![0., 0., 1., 2., 4., 4.],
            control_points: vec![vec![-1., 0.25], vec![0., 0.], vec![4., 0.], vec![5., 0.25]],
            weights: vec![1.; 4],
            periodic: false,
        }).with_trim(brep_topology::CoedgeTrim {
            curve_parameter: range,
            pcurve_parameter: [1., 2.],
            periodic_lift: [[0, 0], [1, 0]],
        }).unwrap();
        model.rebuild_topology_ids();
        model.validate().unwrap();
        let active = inspect_edges_with_exact(&model, &[1], t, 1, 1_000_000).unwrap();
        assert!(active.complete && active.within_tolerance);
        assert_eq!(active.seams[0].faces, [0, 1]);
        assert_eq!(active.seams[0].inspection.as_ref().unwrap().orientation,
            Some(nurbs_core::surface_g2::Orientation::Same));
        let complete = super::super::super::inspect(&model, limits).unwrap();
        let index = complete.edge_ids.iter().position(|&edge| edge == 1).unwrap();
        assert!(complete.proof.seams[index].certified);
        assert!(!complete.extraction_complete);
    }

    #[test]
    fn full_and_classified_g2_join_a_closed_torus_of_two_periodic_bands() {
        use crate::{Body, Coedge, Curve, Edge, Face, FaceUse, Loop, Shell, Vertex};
        use nurbs_core::{editing::{exact_periodic_surface::open_exact_periodic_surface_axis,
            exact_surface_knot_insertion::split_tensor_surface}, surface::Axis};
        let native = periodic_chart(true);
        let opened = open_exact_periodic_surface_axis(&native, Axis::V, 1_000_000)
            .unwrap().surface.unwrap();
        let [lower, upper] = split_tensor_surface(&opened, Axis::V, 2., 1_000_000)
            .unwrap().parts.unwrap();
        for band in [&lower, &upper] {
            let count = band.control_points.len() * band.control_points[0].len();
            let factors = nurbs_core::surface_injectivity::factor_radial_tensor(band, 0, 2, count)
                .unwrap().unwrap();
            assert_eq!(factors.center, [0.; 2]);
            assert_eq!(factors.height_axis, 2);
            assert!(factors.angular.periodic && !factors.meridian.periodic);
            assert!(nurbs_core::surface_injectivity::factor_radial_tensor(band, 0, 2, count - 1)
                .unwrap().is_none());
            let mut perturbation = band.clone();
            let x = &mut perturbation.control_points[2][1][0];
            *x = x.next_up();
            assert!(nurbs_core::surface_injectivity::factor_radial_tensor(&perturbation, 0, 2, count)
                .unwrap().is_none());
        }
        let make_rim = |s: &Surface| Curve {
            degree: s.degree_u, knots: s.knots_u.clone(),
            control_points: s.control_points.iter().map(|r| r[0].clone()).collect(),
            weights: s.weights.iter().map(|r| r[0]).collect(),
            periodic: true,
        };
        let make_wrap = |s: &Surface| Curve {
            degree: s.degree_v, knots: s.knots_v.clone(),
            control_points: s.control_points[1].clone(), weights: s.weights[1].clone(),
            periodic: false,
        };
        let edges: Vec<_> = [
            (make_rim(&lower), [0, 0]), (make_rim(&upper), [1, 1]),
            (make_wrap(&lower), [0, 1]), (make_wrap(&upper), [1, 0]),
        ].into_iter().map(|(curve, vertices)| Edge { curve, vertices, degenerate: false }).collect();
        let wire = |i: usize, lo, hi| Loop {
            coedges: [
                (i, false, [0., lo], [4., lo]), (i + 2, false, [4., lo], [4., hi]),
                (1 - i, true, [4., hi], [0., hi]), (i + 2, true, [0., hi], [0., lo]),
            ].map(|(edge, rev, a, b)| Coedge::new(edge, rev, crate::line(a.to_vec(), b.to_vec()))).to_vec(),
        };
        let vertices = [&lower, &upper].map(|s| {
            let p = &s.control_points[1][0];
            Vertex { point: [p[0], p[1], p[2]] }
        }).to_vec();
        let mut model = crate::Model(brep_topology::Model {
            vertices, edges, loops: vec![wire(0, 0., 2.), wire(1, 2., 4.)],
            faces: vec![
                Face { surface: lower, outer: 0, holes: vec![] },
                Face { surface: upper, outer: 1, holes: vec![] },
            ],
            shells: vec![Shell { closed: true, faces: vec![
                FaceUse { face: 0, reversed: false }, FaceUse { face: 1, reversed: false },
            ] }],
            bodies: vec![Body { outer_shell: 0, inner_shells: vec![] }], tolerance_mm: 1e-8,
        }, crate::TopologyIds::default());
        model.rebuild_topology_ids();
        model.validate().unwrap();
        let limits = super::super::super::Limits {
            order: 2, exact_work: 16_000_000, agreement_work: 1_000_000, regularity_cells: 100_000,
        };
        let complete = super::super::super::inspect(&model, limits).unwrap();
        assert!(complete.g2_certified, "unresolved={:?} seams={:?}", complete.unresolved_edges,
            complete.proof.unresolved_seams);
        assert_eq!(complete.edge_ids, [0, 1]);
        assert_eq!(complete.periodic_edge_ids, [2, 3]);
        let opening_work = model.faces.iter().map(|face| {
            periodic_uses::open(&face.surface, limits.exact_work).unwrap().1
        }).sum::<u64>();
        assert!(opening_work > 0);
        // Each band participates in both shared rims. Its chart is opened once.
        assert_eq!(complete.parameter_work, opening_work);
        let cold = super::super::inspect(
            &model, 2, limits.regularity_cells - complete.regularity.cells, limits.exact_work,
        ).unwrap();
        assert!(cold.certified);
        assert_eq!(cold.geometric_seams, complete.face_interiors.geometric_seams);
        assert_eq!(cold.exact_work, complete.face_interiors.exact_work + opening_work);


        use super::super::super::classified::{self, Class};
        let classes = [Class::G2; 4];
        let classified_limits = classified::Limits {
            incidence_steps: 100, exact_work: limits.exact_work,
            agreement_work: limits.agreement_work, regularity_cells: limits.regularity_cells,
        };
        let classified = classified::inspect(&model, &classes, classified_limits).unwrap();
        assert!(classified.certified, "{}", classified.reason);
        assert_eq!(classified.smooth_edges, [0, 1, 2, 3]);
        assert!(!classified::inspect(&model, &classes, classified::Limits {
            exact_work: classified.exact_work - 1, ..classified_limits
        }).unwrap().certified);

        let range = model.edges[1].curve.domain();
        model.loops[1].coedges[0] = Coedge::new(1, false, Curve {
            degree: 1,
            knots: vec![0., 0., 1., 2., 4., 4.],
            control_points: vec![vec![-1., 2.25], vec![0., 2.], vec![4., 2.], vec![5., 2.25]],
            weights: vec![1.; 4],
            periodic: false,
        }).with_trim(brep_topology::CoedgeTrim {
            curve_parameter: range,
            pcurve_parameter: [1., 2.],
            periodic_lift: [[0, 0], [1, 0]],
        }).unwrap();
        model.rebuild_topology_ids();
        model.validate().unwrap();
        let active = super::super::super::inspect(&model, limits).unwrap();
        assert!(active.g2_certified, "{:?}", active.unresolved_edges);
        assert_eq!(active.edge_ids, [0, 1]);
        assert_eq!(active.periodic_edge_ids, [2, 3]);
        assert!(active.parameter_work > opening_work);
        let mixed = classified::inspect(
            &model, &[Class::G1, Class::G2, Class::G2, Class::C0], classified_limits,
        ).unwrap();
        assert!(mixed.certified, "{}", mixed.reason);
        assert_eq!(mixed.smooth_edges, [0, 1, 2]);
        assert_eq!(mixed.c0_edges, [3]);
    }

    #[test]
    fn classified_wraps_prove_each_requested_order_with_one_budget() {
        use super::super::super::classified::{Class, Limits, inspect};
        let model = single_face_torus(periodic_chart(true));
        let limits = Limits {
            incidence_steps: 100,
            agreement_work: 1_000_000,
            regularity_cells: 100_000,
            exact_work: 16_000_000,
        };
        let classes = [Class::G1, Class::G2];
        let report = inspect(&model, &classes, limits).unwrap();
        assert!(report.certified, "{}", report.reason);
        assert_eq!(report.smooth_edges, [0, 1]);
        assert_eq!(report.classes, classes);
        let short = inspect(
            &model,
            &classes,
            Limits {
                exact_work: report.exact_work - 1,
                ..limits
            },
        )
        .unwrap();
        assert!(!short.certified);
        assert!(short.exact_work < report.exact_work);
        let zero = inspect(
            &model,
            &classes,
            Limits {
                exact_work: 0,
                ..limits
            },
        )
        .unwrap();
        assert!(!zero.certified);
        assert_eq!(zero.uncertain_edge, Some(0));
    }
}

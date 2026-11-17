use brep_core::planar_cap::{
    SectionSheetOptions, build_section_sheets, build_section_sheets_with_support_tolerance,
};
use nurbs_core::curve::Curve;

fn options() -> SectionSheetOptions {
    SectionSheetOptions {
        tolerance: 1e-8,
        max_exact_work: 1_000_000,
        max_verification_cells: 10_000,
    }
}

fn ring(lo: [f64; 2], hi: [f64; 2], z: impl Fn(f64) -> f64) -> Vec<Curve> {
    let [x0, y0] = lo;
    let [x1, y1] = hi;
    let points = [[x0, y0], [x1, y0], [x1, y1], [x0, y1], [x0, y0]];
    points
        .windows(2)
        .map(|pair| {
            Curve::from_polyline(pair.iter().map(|&[x, y]| vec![x, y, z(x)]).collect()).unwrap()
        })
        .collect()
}

#[test]
fn authors_oblique_cap_sheets_with_holes_and_preserves_input_edge_clocks() {
    let outer = ring([0., 0.], [10., 10.], |x| x);
    let hole = ring([2., 2.], [4., 4.], |x| x);
    let separate = ring([20., 20.], [22., 22.], |x| x);
    let report =
        build_section_sheets(&[&hole, &separate, &outer], [-1., 0., 1.], 0., options()).unwrap();
    assert_eq!(report.face_rings, [vec![1], vec![2, 0]]);
    assert_eq!(report.model.faces.len(), 2);
    assert_eq!(report.model.faces[1].holes.len(), 1);
    assert_eq!(report.model.edges.len(), 12);
    assert!(report.model.shells.iter().all(|shell| !shell.closed));
    assert!(
        report
            .model
            .loops
            .iter()
            .flat_map(|l| &l.coedges)
            .any(|c| c.reversed)
    );
    for edge in &report.model.edges {
        assert!(outer.iter().chain(&hole).chain(&separate).any(|curve| {
            curve.control_points == edge.curve.control_points
                && curve.knots == edge.curve.knots
                && curve.weights == edge.curve.weights
        }));
    }
    report.model.validate().unwrap();
    let limited = build_section_sheets(
        &[&hole, &separate, &outer],
        [-1., 0., 1.],
        0.,
        SectionSheetOptions {
            max_exact_work: report.exact_work - 1,
            ..options()
        },
    );
    assert!(limited.is_err());
}

#[test]
fn refuses_a_support_that_cannot_be_lifted_without_rounding() {
    let outline = ring([0., 0.], [1., 1.], |_| 1. / 3.);
    let report = build_section_sheets(&[&outline], [0., 0., 3.], 1., options());
    assert!(report.is_err());
}

#[test]
fn rounds_an_oblique_nonbinary_support_without_warping_it() {
    use num_rational::BigRational as Q;
    let mut wire = ring([0., 0.], [1., 1.], |_| 0.);
    for point in wire.iter_mut().flat_map(|curve| &mut curve.control_points) {
        point[2] = (point[0] + point[1] + 1.) / 3.;
    }
    let normal = [-1., -1., 3.];
    let report =
        build_section_sheets_with_support_tolerance(&[&wire], normal, 1., options(), 1e-14)
            .unwrap();
    let [error] = report.support_error_upper.as_slice() else {
        panic!("{:?}", report.support_error_upper);
    };
    assert!(error > &Q::from_integer(0.into()));
    assert!(error <= &Q::from_float(1e-14).unwrap());
    let cap = &report.sheet.model.faces[0].surface;
    let z = |u: usize, v: usize| Q::from_float(cap.control_points[u][v][2]).unwrap();
    assert_eq!(z(0, 0) + z(1, 1), z(0, 1) + z(1, 0));
    for point in cap.control_points.iter().flatten() {
        let exact = (Q::from_float(point[0]).unwrap()
            + Q::from_float(point[1]).unwrap()
            + Q::from_integer(1.into()))
            / Q::from_integer(3.into());
        let gap = Q::from_float(point[2]).unwrap() - exact;
        assert!(gap.pow(2) <= error.pow(2));
    }
    report.sheet.model.validate().unwrap();
    assert!(
        build_section_sheets_with_support_tolerance(&[&wire], normal, 1., options(), 0.).is_err()
    );
}

#[test]
fn supports_zero_tolerance_for_an_exactly_storable_plane() {
    use num_rational::BigRational as Q;
    let wire = ring([0., 0.], [1., 1.], |_| 0.5);
    let report =
        build_section_sheets_with_support_tolerance(&[&wire], [0., 0., 2.], 1., options(), 0.)
            .unwrap();
    assert_eq!(report.support_error_upper, [Q::from_integer(0.into())]);
    assert_eq!(report.sheet.model.faces.len(), 1);
}

fn open_hollow_prism() -> (brep_core::Model, Vec<Vec<[usize; 2]>>) {
    open_prism(true)
}

fn open_prism(with_hole: bool) -> (brep_core::Model, Vec<Vec<[usize; 2]>>) {
    use brep_core::prism::extrude;
    let outline = |lo, hi| {
        ring(lo, hi, |_| 0.)
            .into_iter()
            .map(|curve| {
                Curve::from_polyline(
                    curve
                        .control_points
                        .iter()
                        .map(|p| p[..2].to_vec())
                        .collect(),
                )
                .unwrap()
            })
            .collect::<Vec<_>>()
    };
    let outer = outline([0., 0.], [4., 4.]);
    let hole: Vec<_> = outline([1., 1.], [2., 2.])
        .into_iter()
        .rev()
        .map(|curve| curve.reverse().unwrap())
        .collect();
    let rings = if with_hole {
        vec![outer, hole]
    } else {
        vec![outer]
    };
    let mut source = extrude(&rings, 0., 2.).unwrap();
    let addresses = remove_prism_cap(&mut source, 2.);
    (source, addresses)
}

fn remove_prism_cap(source: &mut brep_core::Model, height: f64) -> Vec<Vec<[usize; 2]>> {
    let cap = source
        .faces
        .iter()
        .position(|face| {
            face.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == height)
        })
        .unwrap();
    let removed = source.faces.remove(cap);
    let mut removed_loops = vec![removed.outer];
    removed_loops.extend(removed.holes);
    let cap_use = source.shells[0]
        .faces
        .iter()
        .find(|u| u.face == cap)
        .unwrap();
    let edge_rings: Vec<_> = removed_loops
        .iter()
        .map(|&wire| {
            let mut edges: Vec<_> = source.loops[wire].coedges.iter().map(|c| c.edge).collect();
            if !cap_use.reversed {
                edges.reverse();
            }
            edges
        })
        .collect();
    removed_loops.sort_unstable();
    for &wire in removed_loops.iter().rev() {
        source.loops.remove(wire);
    }
    for face in &mut source.faces {
        face.outer -= removed_loops.iter().filter(|&&i| i < face.outer).count();
        for hole in &mut face.holes {
            *hole -= removed_loops.iter().filter(|&&i| i < *hole).count();
        }
    }
    source.shells[0].faces.retain(|u| u.face != cap);
    for face_use in &mut source.shells[0].faces {
        if face_use.face > cap {
            face_use.face -= 1;
        }
    }
    source.shells[0].closed = false;
    source.bodies.clear();
    source.rebuild_topology_ids();
    source.validate().unwrap();
    edge_rings
        .into_iter()
        .map(|edges| {
            edges
                .into_iter()
                .map(|edge| {
                    source
                        .loops
                        .iter()
                        .enumerate()
                        .find_map(|(i, wire)| {
                            wire.coedges
                                .iter()
                                .position(|c| c.edge == edge)
                                .map(|j| [i, j])
                        })
                        .unwrap()
                })
                .collect()
        })
        .collect()
}

#[test]
fn closes_a_hollow_shell_with_one_cap_and_reuses_all_boundary_edges() {
    use brep_core::planar_cap::attach_section_caps;
    let (source, rings) = open_hollow_prism();
    assert_eq!(source.validate().unwrap().boundary_edge_count, 8);
    let wire: Vec<_> = rings.iter().map(Vec::as_slice).collect();
    let report =
        attach_section_caps(&source, 0, &wire, [0., 0., 1.], 2., 1_000_000, 10_000).unwrap();
    assert_eq!(report.cap_faces.len(), 1);
    assert_eq!(report.model.faces[report.cap_faces[0]].holes.len(), 1);
    assert_eq!(report.model.edges, source.edges);
    assert_eq!(report.model.vertices, source.vertices);
    assert_eq!(report.model.validate().unwrap().boundary_edge_count, 0);
    assert!(report.model.shells[0].closed);
    assert!(!source.shells[0].closed);
    assert!(
        attach_section_caps(
            &source,
            0,
            &wire,
            [0., 0., 1.],
            2.,
            report.exact_work - 1,
            10_000
        )
        .is_err()
    );
}

#[test]
fn refuses_duplicate_and_disordered_section_uses_without_mutating_the_source() {
    use brep_core::planar_cap::attach_section_caps;
    let (source, mut rings) = open_hollow_prism();
    let before = source.clone();
    assert!(
        attach_section_caps(
            &source,
            0,
            &[&rings[0], &rings[0]],
            [0., 0., 1.],
            2.,
            1_000_000,
            10_000
        )
        .is_err()
    );
    rings[0].swap(1, 2);
    assert!(
        attach_section_caps(
            &source,
            0,
            &[&rings[0], &rings[1]],
            [0., 0., 1.],
            2.,
            1_000_000,
            10_000
        )
        .is_err()
    );
    assert_eq!(source, before);
}

#[test]
fn discovers_both_free_rings_and_closes_a_hollow_shell() {
    use brep_core::planar_cap::discover_and_attach_section_caps;
    let (source, _) = open_hollow_prism();
    let report =
        discover_and_attach_section_caps(&source, 0, [0., 0., 1.], 2., 1_000_000, 10_000).unwrap();
    assert_eq!(
        report.rings.iter().map(Vec::len).collect::<Vec<_>>(),
        [4, 4]
    );
    let cap = &report.attachment;
    assert_eq!(cap.cap_faces.len(), 1);
    assert_eq!(cap.model.faces[cap.cap_faces[0]].holes.len(), 1);
    assert_eq!(cap.model.edges, source.edges);
    assert_eq!(cap.model.vertices, source.vertices);
    assert!(cap.model.shells[0].closed);
    assert_eq!(cap.model.validate().unwrap().boundary_edge_count, 0);
    let total = report.discovery_work + cap.exact_work;
    assert!(discover_and_attach_section_caps(&source, 0, [0., 0., 1.], 2., total, 10_000).is_ok());
    assert!(
        discover_and_attach_section_caps(&source, 0, [0., 0., 1.], 2., total - 1, 10_000).is_err()
    );
}

#[test]
fn refuses_empty_and_partial_section_cycles_atomically() {
    use brep_core::planar_cap::discover_and_attach_section_caps;
    let (source, _) = open_hollow_prism();
    let before = source.clone();
    assert!(
        discover_and_attach_section_caps(&source, 0, [0., 0., 1.], 1., 1_000_000, 10_000).is_err()
    );
    assert!(
        discover_and_attach_section_caps(&source, 0, [1., 0., 0.], 0., 1_000_000, 10_000).is_err()
    );
    assert_eq!(source, before);
}

#[test]
fn closes_both_ends_of_a_shell_with_a_shared_budget() {
    use brep_core::planar_cap::{SectionPlane, close_shell_with_section_caps};
    let (mut source, _) = open_prism(false);
    remove_prism_cap(&mut source, 0.);
    assert_eq!(source.validate().unwrap().boundary_edge_count, 8);
    let planes = [2., 0.].map(|offset| SectionPlane {
        normal: [0., 0., 1.],
        offset,
    });
    let report = close_shell_with_section_caps(&source, 0, &planes, 1_000_000, 10_000).unwrap();
    assert!(report.model.shells[0].closed);
    assert_eq!(report.cap_faces.len(), 2);
    assert_eq!(report.model.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(report.model.edges, source.edges);
    assert_eq!(report.model.vertices, source.vertices);
    for (&face, plane) in report.cap_faces.iter().zip(planes) {
        let cap = &report.model.faces[face];
        assert!(cap.holes.is_empty());
        assert!(
            cap.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == plane.offset)
        );
    }
    assert!(
        close_shell_with_section_caps(
            &source,
            0,
            &planes,
            report.exact_work,
            report.verification_cells
        )
        .is_ok()
    );
    assert!(
        close_shell_with_section_caps(
            &source,
            0,
            &planes,
            report.exact_work - 1,
            report.verification_cells
        )
        .is_err()
    );
    assert!(
        close_shell_with_section_caps(
            &source,
            0,
            &planes,
            report.exact_work,
            report.verification_cells - 1
        )
        .is_err()
    );
}

#[test]
fn refuses_partial_closure_and_a_late_invalid_plane_atomically() {
    use brep_core::planar_cap::{SectionPlane, close_shell_with_section_caps};
    let (mut source, _) = open_prism(false);
    remove_prism_cap(&mut source, 0.);
    let before = source.clone();
    let planes = [2., 1.].map(|offset| SectionPlane {
        normal: [0., 0., 1.],
        offset,
    });
    assert!(close_shell_with_section_caps(&source, 0, &planes[..1], 1_000_000, 10_000).is_err());
    assert!(close_shell_with_section_caps(&source, 0, &planes, 1_000_000, 10_000).is_err());
    assert_eq!(source, before);
}

#[path = "support/closed_solid_policy.rs"]
mod closed_solid_policy;

fn section_solid_options() -> brep_core::planar_cap::SectionSolidOptions {
    brep_core::planar_cap::SectionSolidOptions {
        max_exact_work: 1_000_000,
        max_verification_cells: 10_000,
        admission: closed_solid_policy::policy(),
        volume_tolerance_mm3: 1e-4,
        max_volume_cells: 10_000,
    }
}

#[test]
fn qualifies_a_capped_hollow_solid_and_encloses_its_material_volume() {
    use brep_core::planar_cap::{SectionPlane, build_section_solid};
    let (source, _) = open_hollow_prism();
    let planes = [SectionPlane {
        normal: [0., 0., 1.],
        offset: 2.,
    }];
    let report = build_section_solid(&source, &planes, section_solid_options())
        .unwrap_or_else(|error| panic!("{error:?}"));
    let result = report.solid.model();
    assert_eq!(result.bodies.len(), 1);
    assert_eq!(result.edges, source.edges);
    assert_eq!(result.vertices, source.vertices);
    assert_eq!(result.faces[report.cap_faces[0]].holes.len(), 1);
    assert!(report.solid.geometry().proven);
    let volume = report.volume.volume_mm3;
    assert!(volume.lower > 0. && volume.lower <= 30. && volume.upper >= 30.);
    assert!(volume.upper - volume.lower < 1e-4);
}

#[test]
fn refuses_an_inward_capped_boundary_without_mutating_the_source() {
    use brep_core::planar_cap::{SectionPlane, build_section_solid};
    let (mut source, _) = open_prism(false);
    for face in &mut source.shells[0].faces {
        face.reversed = !face.reversed;
    }
    let before = source.clone();
    let planes = [SectionPlane {
        normal: [0., 0., 1.],
        offset: 2.,
    }];
    let error = build_section_solid(&source, &planes, section_solid_options())
        .err()
        .unwrap();
    assert_eq!(error.code, "BREP_CLOSED_SOLID_UNPROVED");
    assert_eq!(source, before);
}

fn open_third_height_prism() -> (brep_core::Model, Vec<Vec<[usize; 2]>>) {
    let (mut source, rings) = open_hollow_prism();
    scale_prism_to_third_height(&mut source);
    (source, rings)
}

fn scale_prism_to_third_height(source: &mut brep_core::Model) {
    for vertex in &mut source.vertices {
        vertex.point[2] /= 6.;
    }
    for edge in &mut source.edges {
        for pole in &mut edge.curve.control_points {
            pole[2] /= 6.;
        }
    }
    for face in &mut source.faces {
        for pole in face.surface.control_points.iter_mut().flatten() {
            pole[2] /= 6.;
        }
    }
    source.rebuild_topology_ids();
    source.validate().unwrap();
}

#[test]
fn attaches_a_rounded_planar_support_to_retained_boundary_edges() {
    use brep_core::planar_cap::{
        SectionAttachmentOptions, attach_section_caps_with_support_tolerance,
    };
    use num_rational::BigRational as Q;
    use num_traits::Zero;
    let (source, rings) = open_third_height_prism();
    let addresses: Vec<_> = rings.iter().map(Vec::as_slice).collect();
    let report = attach_section_caps_with_support_tolerance(
        &source,
        0,
        &addresses,
        [0., 0., 3.],
        1.,
        SectionAttachmentOptions {
            max_exact_work: 1_000_000,
            max_verification_cells: 10_000,
            support_tolerance_mm: 1e-14,
        },
    )
    .unwrap();
    let [error] = report.support_error_upper.as_slice() else {
        panic!("expected one support bound");
    };
    assert!(error > &Q::zero() && error <= &Q::from_float(1e-14).unwrap());
    let result = report.attachment;
    assert!(result.model.shells[0].closed);
    assert_eq!(result.model.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(result.model.edges, source.edges);
    assert_eq!(result.model.vertices, source.vertices);
    assert_eq!(result.model.faces[result.cap_faces[0]].holes.len(), 1);
}

#[test]
fn refuses_exact_and_too_small_support_budgets_for_the_nonbinary_plane() {
    use brep_core::planar_cap::{
        SectionAttachmentOptions, attach_section_caps, attach_section_caps_with_support_tolerance,
    };
    let (source, rings) = open_third_height_prism();
    let before = source.clone();
    let addresses: Vec<_> = rings.iter().map(Vec::as_slice).collect();
    assert!(
        attach_section_caps(&source, 0, &addresses, [0., 0., 3.], 1., 1_000_000, 10_000).is_err()
    );
    let result = attach_section_caps_with_support_tolerance(
        &source,
        0,
        &addresses,
        [0., 0., 3.],
        1.,
        SectionAttachmentOptions {
            max_exact_work: 1_000_000,
            max_verification_cells: 10_000,
            support_tolerance_mm: 1e-18,
        },
    );
    assert!(result.is_err());
    assert_eq!(source, before);
}

#[test]
fn discovers_rounded_boundary_cycles_with_normal_scale_invariant_distance() {
    use brep_core::planar_cap::{
        SectionAttachmentOptions, SectionPlane,
        discover_and_attach_section_caps_with_support_tolerance,
    };
    let (source, _) = open_third_height_prism();
    let options = SectionAttachmentOptions {
        max_exact_work: 1_000_000,
        max_verification_cells: 10_000,
        support_tolerance_mm: 1e-14,
    };
    for scale in [1., -8.] {
        let plane = SectionPlane {
            normal: [0., 0., 3. * scale],
            offset: scale,
        };
        let report = discover_and_attach_section_caps_with_support_tolerance(
            &source, 0, plane, 2e-17, options,
        )
        .unwrap();
        assert_eq!(
            report.rings.iter().map(Vec::len).collect::<Vec<_>>(),
            [4, 4]
        );
        let result = report.bounded.attachment;
        assert!(result.model.shells[0].closed);
        assert_eq!(result.model.edges, source.edges);
        assert_eq!(result.model.faces[result.cap_faces[0]].holes.len(), 1);
        assert_eq!(report.bounded.support_error_upper.len(), 1);
        assert!(
            discover_and_attach_section_caps_with_support_tolerance(
                &source, 0, plane, 1e-17, options,
            )
            .is_err()
        );
    }
}

#[test]
fn rounded_cycle_selection_cannot_override_the_support_error_limit() {
    use brep_core::planar_cap::{
        SectionAttachmentOptions, SectionPlane,
        discover_and_attach_section_caps_with_support_tolerance,
    };
    let (source, _) = open_third_height_prism();
    let before = source.clone();
    let plane = SectionPlane {
        normal: [0., 0., 3.],
        offset: 1.,
    };
    let options = SectionAttachmentOptions {
        max_exact_work: 1_000_000,
        max_verification_cells: 10_000,
        support_tolerance_mm: 1e-18,
    };
    assert!(
        discover_and_attach_section_caps_with_support_tolerance(&source, 0, plane, 2e-17, options,)
            .is_err()
    );
    assert_eq!(source, before);
}

fn mixed_section_planes() -> [brep_core::planar_cap::BoundedSectionPlane; 2] {
    use brep_core::planar_cap::{BoundedSectionPlane, SectionPlane};
    [
        BoundedSectionPlane {
            plane: SectionPlane {
                normal: [0., 0., 1.],
                offset: 0.,
            },
            selection_tolerance_mm: 0.,
            support_tolerance_mm: 0.,
        },
        BoundedSectionPlane {
            plane: SectionPlane {
                normal: [0., 0., 3.],
                offset: 1.,
            },
            selection_tolerance_mm: 2e-17,
            support_tolerance_mm: 1e-14,
        },
    ]
}

#[test]
fn closes_mixed_exact_and_rounded_sections_under_shared_budgets() {
    use brep_core::planar_cap::close_shell_with_bounded_section_caps;
    use num_traits::Zero;
    let (mut source, _) = open_prism(false);
    remove_prism_cap(&mut source, 0.);
    scale_prism_to_third_height(&mut source);
    let planes = mixed_section_planes();
    let report =
        close_shell_with_bounded_section_caps(&source, 0, &planes, 1_000_000, 10_000).unwrap();
    let result = &report.attachment;
    assert!(result.model.shells[0].closed);
    assert_eq!(result.model.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(result.model.edges, source.edges);
    assert_eq!(result.model.vertices, source.vertices);
    assert_eq!(result.cap_faces.len(), 2);
    assert!(report.support_error_upper[0].is_zero());
    assert!(!report.support_error_upper[1].is_zero());
    assert!(
        close_shell_with_bounded_section_caps(
            &source,
            0,
            &planes,
            result.exact_work,
            result.verification_cells
        )
        .is_ok()
    );
    assert!(
        close_shell_with_bounded_section_caps(
            &source,
            0,
            &planes,
            result.exact_work - 1,
            result.verification_cells
        )
        .is_err()
    );
    assert!(
        close_shell_with_bounded_section_caps(
            &source,
            0,
            &planes,
            result.exact_work,
            result.verification_cells - 1
        )
        .is_err()
    );
}

#[test]
fn refuses_a_late_support_failure_without_committing_the_first_cap() {
    use brep_core::planar_cap::close_shell_with_bounded_section_caps;
    let (mut source, _) = open_prism(false);
    remove_prism_cap(&mut source, 0.);
    scale_prism_to_third_height(&mut source);
    let before = source.clone();
    let mut planes = mixed_section_planes();
    planes[1].support_tolerance_mm = 0.;
    assert!(close_shell_with_bounded_section_caps(&source, 0, &planes, 1_000_000, 10_000).is_err());
    assert_eq!(source, before);
}

#[test]
fn qualifies_a_mixed_section_solid_and_bounds_its_stored_volume() {
    use brep_core::planar_cap::build_bounded_section_solid;
    use num_rational::BigRational as Q;
    use num_traits::Zero;
    let (mut source, _) = open_prism(false);
    remove_prism_cap(&mut source, 0.);
    scale_prism_to_third_height(&mut source);
    let planes = mixed_section_planes();
    let report = build_bounded_section_solid(&source, &planes, section_solid_options())
        .unwrap_or_else(|error| panic!("{error:?}"));
    assert!(report.support_error_upper[0].is_zero());
    assert!(report.support_error_upper[1] > Q::zero());
    let section = report.section;
    assert_eq!(section.cap_faces.len(), 2);
    assert!(section.solid.geometry().proven);
    let model = section.solid.model();
    assert_eq!(model.edges, source.edges);
    assert_eq!(model.vertices, source.vertices);
    assert_eq!(model.bodies.len(), 1);
    let expected = 16. * (1. / 3.);
    let volume = section.volume.volume_mm3;
    assert!(0. < volume.lower && volume.lower <= expected && expected <= volume.upper);
    assert!(volume.upper - volume.lower < 1e-4);
}

#[test]
fn rejects_a_bounded_section_solid_when_final_material_proof_fails() {
    use brep_core::planar_cap::build_bounded_section_solid;
    let (mut source, _) = open_prism(false);
    remove_prism_cap(&mut source, 0.);
    scale_prism_to_third_height(&mut source);
    for face in &mut source.shells[0].faces {
        face.reversed = !face.reversed;
    }
    let before = source.clone();
    let error =
        build_bounded_section_solid(&source, &mixed_section_planes(), section_solid_options())
            .err()
            .unwrap();
    assert_eq!(error.code, "BREP_CLOSED_SOLID_UNPROVED");
    assert_eq!(source, before);
}

#[test]
fn infers_exact_section_planes_and_groups_coplanar_hole_rings() {
    use brep_core::planar_cap::infer_section_planes;
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let report = infer_section_planes(&source, 0, 1_000_000).unwrap();
    let [plane] = report.planes.as_slice() else {
        panic!("expected one plane")
    };
    assert_eq!(plane.normal, [0, 0, 1].map(|x| Q::from_integer(x.into())));
    assert_eq!(plane.offset, Q::from_integer(2.into()));
    assert_eq!(plane.rings.iter().map(Vec::len).collect::<Vec<_>>(), [4, 4]);
    assert!(infer_section_planes(&source, 0, report.exact_work - 1).is_err());

    let (mut source, _) = open_prism(false);
    remove_prism_cap(&mut source, 0.);
    let report = infer_section_planes(&source, 0, 1_000_000).unwrap();
    assert_eq!(report.planes.len(), 2);
    assert_eq!(report.planes[0].offset, Q::from_integer(0.into()));
    assert_eq!(report.planes[1].offset, Q::from_integer(2.into()));
}

#[test]
fn refuses_to_infer_a_plane_from_a_warped_free_rim() {
    use brep_core::planar_cap::infer_section_planes;
    let (mut source, _) = open_prism(false);
    let warp = |point: &mut [f64]| {
        if point == [4., 4., 2.] {
            point[2] = 3.;
        }
    };
    for vertex in &mut source.vertices {
        warp(&mut vertex.point);
    }
    for edge in &mut source.edges {
        for point in &mut edge.curve.control_points {
            warp(point);
        }
    }
    for face in &mut source.faces {
        for point in face.surface.control_points.iter_mut().flatten() {
            warp(point);
        }
    }
    source.rebuild_topology_ids();
    assert!(source.validate().is_ok());
    let error = infer_section_planes(&source, 0, 1_000_000).unwrap_err();
    assert_eq!(error.message, "Free section cycle has nonplanar controls");
}

#[test]
fn automatically_caps_two_oblique_rims_with_exact_equations() {
    use brep_core::planar_cap::close_shell_with_inferred_section_caps;
    let (mut source, _) = open_prism(false);
    remove_prism_cap(&mut source, 0.);
    let shear = |point: &mut [f64]| point[2] += 0.75 * point[0];
    for vertex in &mut source.vertices {
        shear(&mut vertex.point);
    }
    for edge in &mut source.edges {
        for point in &mut edge.curve.control_points {
            shear(point);
        }
    }
    for face in &mut source.faces {
        for point in face.surface.control_points.iter_mut().flatten() {
            shear(point);
        }
    }
    source.rebuild_topology_ids();
    let report = close_shell_with_inferred_section_caps(&source, 0, 1_000_000, 10_000).unwrap();
    assert!(report.model.shells[0].closed);
    assert_eq!(report.cap_faces.len(), 2);
    assert_eq!(report.model.edges, source.edges);
    assert_eq!(report.model.vertices, source.vertices);
    assert_eq!(report.model.validate().unwrap().boundary_edge_count, 0);
    assert!(
        close_shell_with_inferred_section_caps(
            &source,
            0,
            report.exact_work,
            report.verification_cells
        )
        .is_ok()
    );
    assert!(
        close_shell_with_inferred_section_caps(
            &source,
            0,
            report.exact_work - 1,
            report.verification_cells
        )
        .is_err()
    );
}

#[test]
fn automatically_caps_nested_rims_as_one_face_with_a_hole() {
    use brep_core::planar_cap::close_shell_with_inferred_section_caps;
    let (source, _) = open_hollow_prism();
    let report = close_shell_with_inferred_section_caps(&source, 0, 1_000_000, 10_000).unwrap();
    assert_eq!(report.cap_faces.len(), 1);
    assert_eq!(report.model.faces[report.cap_faces[0]].holes.len(), 1);
    assert!(report.model.shells[0].closed);
    assert!(!source.shells[0].closed);
}

fn open_prism_with_nonbinary_cap_support() -> brep_core::Model {
    let (mut source, _) = open_prism(false);
    map_prism_to_nonbinary_cap_support(&mut source);
    source
}

fn map_prism_to_nonbinary_cap_support(source: &mut brep_core::Model) {
    let map = |point: &mut [f64]| {
        let (x, y, z) = (point[0], point[1], point[2]);
        point.copy_from_slice(&[0.5 * x + 0.25 * y, -0.5 * x + 0.5 * y, z + 0.25 * y]);
    };
    for vertex in &mut source.vertices {
        map(&mut vertex.point);
    }
    for edge in &mut source.edges {
        for point in &mut edge.curve.control_points {
            map(point);
        }
    }
    for face in &mut source.faces {
        for point in face.surface.control_points.iter_mut().flatten() {
            map(point);
        }
    }
    source.rebuild_topology_ids();
    source.validate().unwrap();
}

#[test]
fn caps_an_exact_inferred_oblique_rim_with_a_rounded_support() {
    use brep_core::planar_cap::{
        SectionAttachmentOptions, close_shell_with_bounded_inferred_section_caps,
        close_shell_with_inferred_section_caps,
    };
    use num_rational::BigRational as Q;
    use num_traits::Zero;
    let source = open_prism_with_nonbinary_cap_support();
    assert!(close_shell_with_inferred_section_caps(&source, 0, 1_000_000, 10_000).is_err());
    let options = SectionAttachmentOptions {
        support_tolerance_mm: 1e-14,
        max_exact_work: 1_000_000,
        max_verification_cells: 10_000,
    };
    let report = close_shell_with_bounded_inferred_section_caps(&source, 0, options).unwrap();
    let [error] = report.support_error_upper.as_slice() else {
        panic!("expected one bound")
    };
    assert!(error > &Q::zero() && error <= &Q::from_float(options.support_tolerance_mm).unwrap());
    assert_eq!(
        report
            .attachment
            .model
            .validate()
            .unwrap()
            .boundary_edge_count,
        0
    );
    assert!(report.attachment.model.shells[0].closed);
    assert_eq!(report.attachment.model.edges, source.edges);
    assert_eq!(report.attachment.model.vertices, source.vertices);
    let tight = SectionAttachmentOptions {
        max_exact_work: report.attachment.exact_work,
        max_verification_cells: report.attachment.verification_cells,
        ..options
    };
    assert!(close_shell_with_bounded_inferred_section_caps(&source, 0, tight).is_ok());
    assert!(
        close_shell_with_bounded_inferred_section_caps(
            &source,
            0,
            SectionAttachmentOptions {
                max_exact_work: tight.max_exact_work - 1,
                ..tight
            }
        )
        .is_err()
    );
}

#[test]
fn refuses_zero_rounding_allowance_for_the_inferred_oblique_support() {
    use brep_core::planar_cap::{
        SectionAttachmentOptions, close_shell_with_bounded_inferred_section_caps,
    };
    let source = open_prism_with_nonbinary_cap_support();
    let before = source.clone();
    assert!(
        close_shell_with_bounded_inferred_section_caps(
            &source,
            0,
            SectionAttachmentOptions {
                support_tolerance_mm: 0.,
                max_exact_work: 1_000_000,
                max_verification_cells: 10_000,
            }
        )
        .is_err()
    );
    assert_eq!(source, before);
}

#[test]
fn builds_a_certified_hollow_solid_from_inferred_cap_planes() {
    use brep_core::planar_cap::build_inferred_section_solid;
    use num_traits::Zero;
    let (source, _) = open_hollow_prism();
    let report = build_inferred_section_solid(&source, 0., section_solid_options())
        .unwrap_or_else(|error| panic!("{error:?}"));
    let [error] = report.support_error_upper.as_slice() else {
        panic!("expected one cap")
    };
    assert!(error.is_zero());
    assert!(report.section.solid.geometry().proven);
    let model = report.section.solid.model();
    assert_eq!(model.edges, source.edges);
    assert_eq!(model.vertices, source.vertices);
    assert_eq!(model.bodies.len(), 1);
    assert_eq!(model.faces[report.section.cap_faces[0]].holes.len(), 1);
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    let volume = report.section.volume.volume_mm3;
    assert!(volume.lower > 0. && volume.lower <= 30. && volume.upper >= 30.);
    assert!(volume.upper - volume.lower < 1e-4);
}

#[test]
fn refuses_a_rounded_inferred_cap_when_embedding_is_unproved() {
    use brep_core::planar_cap::build_inferred_section_solid;
    let source = open_prism_with_nonbinary_cap_support();
    let before = source.clone();
    let error = build_inferred_section_solid(&source, 1e-14, section_solid_options())
        .err()
        .unwrap();
    assert_eq!(error.code, "BREP_CLOSED_SOLID_UNPROVED");
    assert_eq!(source, before);
}

#[test]
fn refuses_an_inward_inferred_solid_after_capping() {
    use brep_core::planar_cap::build_inferred_section_solid;
    let (mut source, _) = open_hollow_prism();
    for face in &mut source.shells[0].faces {
        face.reversed = !face.reversed;
    }
    let before = source.clone();
    let error = build_inferred_section_solid(&source, 0., section_solid_options())
        .err()
        .unwrap();
    assert_eq!(error.code, "BREP_CLOSED_SOLID_UNPROVED");
    assert_eq!(source, before);
}

#[test]
fn reports_unproved_boundary_of_the_capped_candidate() {
    use brep_core::planar_cap::{SectionAttachmentOptions, inspect_inferred_section_solid};
    let source = open_prism_with_nonbinary_cap_support();
    let report = inspect_inferred_section_solid(
        &source,
        SectionAttachmentOptions {
            max_exact_work: 1_000_000,
            max_verification_cells: 10_000,
            support_tolerance_mm: 1e-14,
        },
        closed_solid_policy::policy(),
    )
    .unwrap();
    assert_eq!(report.cap_faces.len(), 1);
    assert_eq!(report.support_error_upper.len(), 1);
    let boundary = &report.admission.geometry().boundary;
    assert!(!boundary.proven);
    assert!(boundary.agreement.has_exact_counterexample());
    assert!(report.admission.geometry().nesting.is_none());
    assert!(report.admission.solid().is_none());
    assert!(report.admission.candidate().shells[0].closed);
    assert_eq!(report.admission.candidate().edges, source.edges);
}

#[test]
fn reports_inward_orientation_after_proving_the_boundary() {
    use brep_core::planar_cap::{SectionAttachmentOptions, inspect_inferred_section_solid};
    let (mut source, _) = open_hollow_prism();
    for face in &mut source.shells[0].faces {
        face.reversed = !face.reversed;
    }
    let report = inspect_inferred_section_solid(
        &source,
        SectionAttachmentOptions {
            max_exact_work: 1_000_000,
            max_verification_cells: 10_000,
            support_tolerance_mm: 0.,
        },
        closed_solid_policy::policy(),
    )
    .unwrap();
    let geometry = report.admission.geometry();
    assert!(geometry.boundary.proven);
    assert!(geometry.nesting.is_some());
    assert_eq!(geometry.orientations[0].outward, Some(false));
    assert!(report.admission.solid().is_none());
}

#[test]
fn expands_the_support_to_keep_an_oblique_rim_exact() {
    use brep_core::planar_cap::build_section_sheets_with_expanded_exact_support;
    let points = [
        [0., 0., 2.],
        [2., -2., 2.],
        [3., 0., 3.],
        [1., 2., 3.],
        [0., 0., 2.],
    ];
    let ring: Vec<_> = points
        .windows(2)
        .map(|p| Curve::from_polyline(p.iter().map(|p| p.to_vec()).collect()).unwrap())
        .collect();
    let normal = [-1., -1., 3.];
    assert!(build_section_sheets(&[&ring], normal, 6., options()).is_err());
    let report = build_section_sheets_with_expanded_exact_support(&[&ring], normal, 6., options())
        .unwrap_or_else(|error| panic!("{error:?}"));
    let agreement = brep_core::boundary_agreement::verify_exact(&report.model, 1_000_000).unwrap();
    assert!(agreement.all_equal);
    assert!(agreement.all_joins_exact);
    for edge in &report.model.edges {
        assert!(ring.contains(&edge.curve));
    }
    assert!(
        build_section_sheets_with_expanded_exact_support(
            &[&ring],
            normal,
            6.,
            SectionSheetOptions {
                max_exact_work: report.exact_work - 1,
                ..options()
            }
        )
        .is_err()
    );
}

#[test]
fn expanded_exact_support_refuses_a_nearby_rim_off_the_requested_plane() {
    use brep_core::planar_cap::build_section_sheets_with_expanded_exact_support;
    let outline = ring([0., 0.], [1., 1.], |_| 1. / 3.);
    assert!(
        build_section_sheets_with_expanded_exact_support(&[&outline], [0., 0., 3.], 1., options())
            .is_err()
    );
}

#[test]
fn qualifies_an_inferred_oblique_solid_with_expanded_exact_support() {
    use brep_core::planar_cap::build_expanded_inferred_section_solid;
    let source = open_prism_with_nonbinary_cap_support();
    let report = build_expanded_inferred_section_solid(&source, section_solid_options())
        .unwrap_or_else(|error| panic!("{error:?}"));
    assert!(report.solid.geometry().proven);
    assert!(report.solid.geometry().boundary.agreement.all_equal);
    assert_eq!(report.cap_faces.len(), 1);
    let model = report.solid.model();
    assert_eq!(model.bodies.len(), 1);
    assert_eq!(model.edges, source.edges);
    assert_eq!(model.vertices, source.vertices);
    let volume = report.volume.volume_mm3;
    assert!(volume.lower > 0. && volume.lower <= 12. && 12. <= volume.upper);
    assert!(volume.upper - volume.lower < 1e-4);
}

#[test]
fn expanded_inferred_solid_refuses_budget_exhaustion_atomically() {
    use brep_core::planar_cap::build_expanded_inferred_section_solid;
    let source = open_prism_with_nonbinary_cap_support();
    let before = source.clone();
    let mut options = section_solid_options();
    let success = build_expanded_inferred_section_solid(&source, options).unwrap();
    options.max_exact_work = success.attachment_exact_work - 1;
    assert!(build_expanded_inferred_section_solid(&source, options).is_err());
    assert_eq!(source, before);
}

#[test]
fn refines_exact_oblique_support_for_submillimeter_rims() {
    use brep_core::planar_cap::build_section_sheets_with_expanded_exact_support;
    let shape = [
        [0., 0., 2.],
        [2., -2., 2.],
        [3., 0., 3.],
        [1., 2., 3.],
        [0., 0., 2.],
    ];
    for scale in [1. / 32., 1. / 1024.] {
        let ring: Vec<_> = shape
            .windows(2)
            .map(|pair| {
                Curve::from_polyline(
                    pair.iter()
                        .map(|p| p.iter().map(|x| x * scale).collect())
                        .collect(),
                )
                .unwrap()
            })
            .collect();
        let report = build_section_sheets_with_expanded_exact_support(
            &[&ring],
            [-1., -1., 3.],
            6. * scale,
            options(),
        )
        .unwrap();
        let face = &report.model.faces[0].surface;
        for (knots, width) in [(&face.knots_u, 3. * scale), (&face.knots_v, 4. * scale)] {
            let expanded = knots[3] - knots[0];
            assert!(expanded >= width && expanded <= 3. * width);
        }
        assert!(
            brep_core::boundary_agreement::verify_exact(&report.model, 1_000_000)
                .unwrap()
                .all_equal
        );
    }
}

#[test]
fn compact_exact_cap_support_stays_inside_the_vertex_coordinate_range() {
    use brep_core::planar_cap::build_section_sheets_with_expanded_exact_support;
    let y = 999_997.75;
    let points = [
        [0., y, 2.],
        [2., y - 2., 2.],
        [3., y, 3.],
        [1., y + 2., 3.],
        [0., y, 2.],
    ];
    let wire: Vec<_> = points
        .windows(2)
        .map(|pair| Curve::from_polyline(pair.iter().map(|p| p.to_vec()).collect()).unwrap())
        .collect();
    let report = build_section_sheets_with_expanded_exact_support(
        &[&wire],
        [-1., -1., 3.],
        6. - y,
        options(),
    )
    .unwrap();
    let support = &report.model.faces[0].surface;
    assert!(support.knots_v[3] >= y + 2.);
    assert!(support.knots_v[3] <= 1e6);
    assert!(
        brep_core::boundary_agreement::verify_exact(&report.model, 1_000_000)
            .unwrap()
            .all_equal
    );
}

#[test]
fn attaches_an_expanded_exact_cap_to_only_the_selected_rim() {
    use brep_core::planar_cap::{
        attach_section_caps_with_expanded_exact_support, infer_section_planes,
    };
    use num_rational::BigRational as Q;
    let (mut source, _) = open_prism(false);
    remove_prism_cap(&mut source, 0.);
    map_prism_to_nonbinary_cap_support(&mut source);
    assert_eq!(source.validate().unwrap().boundary_edge_count, 8);
    let inferred = infer_section_planes(&source, 0, 1_000_000).unwrap();
    let top = inferred
        .planes
        .iter()
        .find(|plane| plane.offset == Q::from_integer((-6).into()))
        .unwrap();
    let rings: Vec<_> = top.rings.iter().map(Vec::as_slice).collect();
    let result = attach_section_caps_with_expanded_exact_support(
        &source,
        0,
        &rings,
        [-1., -1., 3.],
        6.,
        1_000_000,
        10_000,
    )
    .unwrap();
    assert_eq!(result.cap_faces.len(), 1);
    assert!(!result.model.shells[0].closed);
    assert_eq!(result.model.validate().unwrap().boundary_edge_count, 4);
    assert_eq!(result.model.edges, source.edges);
    assert_eq!(result.model.vertices, source.vertices);
    assert!(
        brep_core::boundary_agreement::verify_exact(&result.model, 1_000_000)
            .unwrap()
            .all_equal
    );
    assert_eq!(source.validate().unwrap().boundary_edge_count, 8);
}

#[test]
fn expanded_exact_attachment_refuses_a_plane_offset_within_model_tolerance() {
    use brep_core::planar_cap::{
        attach_section_caps_with_expanded_exact_support, infer_section_planes,
    };
    let source = open_prism_with_nonbinary_cap_support();
    let inferred = infer_section_planes(&source, 0, 1_000_000).unwrap();
    let rings: Vec<_> = inferred.planes[0].rings.iter().map(Vec::as_slice).collect();
    assert!(
        attach_section_caps_with_expanded_exact_support(
            &source,
            0,
            &rings,
            [-1., -1., 3.],
            6. + source.tolerance_mm / 8.,
            1_000_000,
            10_000,
        )
        .is_err()
    );
}

#[test]
fn discovers_and_caps_only_the_requested_oblique_opening_exactly() {
    use brep_core::planar_cap::{
        SectionPlane, discover_and_attach_section_caps_with_expanded_exact_support,
    };
    let (mut source, _) = open_prism(false);
    remove_prism_cap(&mut source, 0.);
    map_prism_to_nonbinary_cap_support(&mut source);
    let plane = SectionPlane {
        normal: [-1., -1., 3.],
        offset: 6.,
    };
    let report = discover_and_attach_section_caps_with_expanded_exact_support(
        &source, 0, plane, 1_000_000, 10_000,
    )
    .unwrap();
    assert_eq!(report.rings[0].len(), 4);
    assert_eq!(report.rings.len(), 1);
    let cap = &report.attachment;
    assert_eq!(cap.model.validate().unwrap().boundary_edge_count, 4);
    assert_eq!(cap.model.edges, source.edges);
    assert_eq!(cap.model.vertices, source.vertices);
    assert!(
        brep_core::boundary_agreement::verify_exact(&cap.model, 1_000_000)
            .unwrap()
            .all_equal
    );
    let work = cap.exact_work + report.discovery_work;
    assert!(
        discover_and_attach_section_caps_with_expanded_exact_support(
            &source, 0, plane, work, 10_000
        )
        .is_ok()
    );
    assert!(
        discover_and_attach_section_caps_with_expanded_exact_support(
            &source,
            0,
            plane,
            work - 1,
            10_000
        )
        .is_err()
    );
}

#[test]
fn expanded_exact_discovery_rejects_a_partial_oblique_rim() {
    use brep_core::planar_cap::{
        SectionPlane, discover_and_attach_section_caps_with_expanded_exact_support,
    };
    let source = open_prism_with_nonbinary_cap_support();
    let before = source.clone();
    let plane = SectionPlane {
        normal: [1., 1., 0.],
        offset: 0.,
    };
    let err = discover_and_attach_section_caps_with_expanded_exact_support(
        &source, 0, plane, 1_000_000, 10_000,
    )
    .unwrap_err();
    assert_eq!(err.message, "Section boundary has an open chain");
    assert_eq!(source, before);
}

#[test]
fn closes_both_oblique_rims_with_exact_expanded_caps_under_shared_budgets() {
    use brep_core::planar_cap::{SectionPlane, close_shell_with_expanded_exact_section_caps};
    let (mut source, _) = open_prism(false);
    remove_prism_cap(&mut source, 0.);
    map_prism_to_nonbinary_cap_support(&mut source);
    let planes = [6., 0.].map(|offset| SectionPlane {
        normal: [-1., -1., 3.],
        offset,
    });
    let report =
        close_shell_with_expanded_exact_section_caps(&source, 0, &planes, 1_000_000, 10_000)
            .unwrap();
    assert_eq!(report.cap_faces.len(), 2);
    assert!(report.model.shells[0].closed);
    assert_eq!(report.model.edges, source.edges);
    assert_eq!(report.model.vertices, source.vertices);
    for (&face, plane) in report.cap_faces.iter().zip(planes) {
        let pole = &report.model.faces[face].surface.control_points[0][0];
        assert_eq!(-pole[0] - pole[1] + 3. * pole[2], plane.offset);
    }
    assert!(
        brep_core::boundary_agreement::verify_exact(&report.model, 1_000_000)
            .unwrap()
            .all_equal
    );
    assert!(
        close_shell_with_expanded_exact_section_caps(
            &source,
            0,
            &planes,
            report.exact_work,
            report.verification_cells
        )
        .is_ok()
    );
    assert!(
        close_shell_with_expanded_exact_section_caps(
            &source,
            0,
            &planes,
            report.exact_work - 1,
            report.verification_cells
        )
        .is_err()
    );
    assert!(
        close_shell_with_expanded_exact_section_caps(
            &source,
            0,
            &planes,
            report.exact_work,
            report.verification_cells - 1
        )
        .is_err()
    );
}

#[test]
fn refuses_to_commit_an_expanded_cap_when_a_later_plane_misses_its_rim() {
    use brep_core::planar_cap::{SectionPlane, close_shell_with_expanded_exact_section_caps};
    let (mut source, _) = open_prism(false);
    remove_prism_cap(&mut source, 0.);
    map_prism_to_nonbinary_cap_support(&mut source);
    let before = source.clone();
    let planes = [6., 1.].map(|offset| SectionPlane {
        normal: [-1., -1., 3.],
        offset,
    });
    assert!(
        close_shell_with_expanded_exact_section_caps(&source, 0, &planes, 1_000_000, 10_000)
            .is_err()
    );
    assert_eq!(source, before);
}

#[test]
fn qualifies_a_solid_from_two_supplied_oblique_section_planes() {
    use brep_core::planar_cap::{SectionPlane, build_expanded_section_solid};
    let (mut source, _) = open_prism(false);
    remove_prism_cap(&mut source, 0.);
    map_prism_to_nonbinary_cap_support(&mut source);
    let planes = [6., 0.].map(|offset| SectionPlane {
        normal: [-1., -1., 3.],
        offset,
    });
    let report = build_expanded_section_solid(&source, &planes, section_solid_options())
        .unwrap_or_else(|error| panic!("{error:?}"));
    let model = report.solid.model();
    assert!(report.solid.geometry().proven);
    assert!(report.solid.geometry().boundary.agreement.all_equal);
    assert_eq!(report.cap_faces.len(), 2);
    assert_eq!(model.edges, source.edges);
    assert_eq!(model.vertices, source.vertices);
    assert_eq!(model.bodies.len(), 1);
    let interval = report.volume.volume_mm3;
    assert!(0. < interval.lower && interval.lower <= 12. && 12. <= interval.upper);
    assert!(interval.upper - interval.lower < 1e-4);
}

#[test]
fn rejects_a_closed_but_inward_solid_from_supplied_exact_caps() {
    use brep_core::planar_cap::{SectionPlane, build_expanded_section_solid};
    let mut source = open_prism_with_nonbinary_cap_support();
    for face in &mut source.shells[0].faces {
        face.reversed = !face.reversed;
    }
    let before = source.clone();
    let planes = [SectionPlane {
        normal: [-1., -1., 3.],
        offset: 6.,
    }];
    let error = build_expanded_section_solid(&source, &planes, section_solid_options())
        .err()
        .unwrap();
    assert_eq!(error.code, "BREP_CLOSED_SOLID_UNPROVED");
    assert_eq!(source, before);
}

#[test]
fn corrects_proven_inward_orientation_after_exact_section_closure() {
    use brep_core::planar_cap::{SectionPlane, build_outward_section_solid};
    let mut source = open_prism_with_nonbinary_cap_support();
    let planes = [SectionPlane {
        normal: [-1., -1., 3.],
        offset: 6.,
    }];
    let options = section_solid_options();
    let outward = build_outward_section_solid(&source, &planes, options).unwrap();
    assert!(!outward.orientation_reversed);
    for face in &mut source.shells[0].faces {
        face.reversed = !face.reversed;
    }
    let before = source.clone();
    let corrected = build_outward_section_solid(&source, &planes, options).unwrap();
    assert!(corrected.orientation_reversed);
    let solid = corrected.section.solid;
    assert!(solid.geometry().proven);
    assert_eq!(solid.geometry().orientations[0].outward, Some(true));
    assert_eq!(
        solid.model().shells[0].faces,
        outward.section.solid.model().shells[0].faces
    );
    assert_eq!(solid.model().edges, source.edges);
    assert_eq!(solid.model().vertices, source.vertices);
    let volume = corrected.section.volume.volume_mm3;
    assert!(volume.lower > 0. && volume.lower <= 12. && 12. <= volume.upper);
    assert_eq!(source, before);
}

#[test]
fn outward_capping_refuses_when_orientation_cannot_be_proven() {
    use brep_core::planar_cap::{SectionPlane, build_outward_section_solid};
    let mut source = open_prism_with_nonbinary_cap_support();
    for face in &mut source.shells[0].faces {
        face.reversed = !face.reversed;
    }
    let before = source.clone();
    let mut options = section_solid_options();
    options.admission.geometry.orientation_cells = 1;
    let planes = [SectionPlane {
        normal: [-1., -1., 3.],
        offset: 6.,
    }];
    let err = build_outward_section_solid(&source, &planes, options)
        .err()
        .unwrap();
    assert_eq!(err.code, "BREP_CLOSED_SOLID_UNPROVED");
    assert_eq!(source, before);
}

#[test]
fn infers_two_oblique_caps_and_corrects_the_inward_shell() {
    use brep_core::planar_cap::build_outward_inferred_section_solid;
    let (mut source, _) = open_prism(false);
    remove_prism_cap(&mut source, 0.);
    map_prism_to_nonbinary_cap_support(&mut source);
    let options = section_solid_options();
    let outward = build_outward_inferred_section_solid(&source, options).unwrap();
    assert!(!outward.orientation_reversed);
    for face in &mut source.shells[0].faces {
        face.reversed = !face.reversed;
    }
    let before = source.clone();
    let corrected = build_outward_inferred_section_solid(&source, options).unwrap();
    assert!(corrected.orientation_reversed);
    assert_eq!(corrected.section.cap_faces.len(), 2);
    let solid = corrected.section.solid;
    assert!(solid.geometry().boundary.agreement.all_equal);
    assert_eq!(solid.geometry().orientations[0].outward, Some(true));
    assert_eq!(solid.model().edges, source.edges);
    assert_eq!(solid.model().vertices, source.vertices);
    let volume = corrected.section.volume.volume_mm3;
    assert!(volume.lower > 0. && volume.lower <= 12. && 12. <= volume.upper);
    assert_eq!(source, before);
}

#[test]
fn refuses_inferred_orientation_repair_without_a_boundary_proof() {
    use brep_core::planar_cap::build_outward_inferred_section_solid;
    let mut source = open_prism_with_nonbinary_cap_support();
    for face in &mut source.shells[0].faces {
        face.reversed = !face.reversed;
    }
    let before = source.clone();
    let mut options = section_solid_options();
    options.admission.geometry.boundary.exact_work = 1;
    let error = build_outward_inferred_section_solid(&source, options)
        .err()
        .unwrap();
    assert_eq!(error.code, "BREP_CLOSED_SOLID_UNPROVED");
    assert_eq!(source, before);
}

#[test]
fn measures_exact_volume_of_a_hollow_prism_from_its_oriented_rims() {
    use brep_core::{
        analysis::exact_polyhedral_volume, planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let report = exact_polyhedral_volume(&result.section.solid, 1_000_000).unwrap();
    assert_eq!(report.volume_mm3, Q::from_integer(30.into()));
    assert!(exact_polyhedral_volume(&result.section.solid, report.exact_work).is_ok());
    assert!(exact_polyhedral_volume(&result.section.solid, report.exact_work - 1).is_err());
}

#[test]
fn measures_exact_volume_from_oblique_caps_with_expanded_supports() {
    use brep_core::{
        analysis::exact_polyhedral_volume, planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let source = open_prism_with_nonbinary_cap_support();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let report = exact_polyhedral_volume(&result.section.solid, 1_000_000).unwrap();
    assert_eq!(report.volume_mm3, Q::from_integer(12.into()));
}

#[test]
fn exact_centroid_accounts_for_an_off_center_hole() {
    use brep_core::{
        analysis::exact_polyhedral_centroid, planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let report = exact_polyhedral_centroid(&result.section.solid, 1_000_000).unwrap();
    assert_eq!(report.volume_mm3, Q::from_integer(30.into()));
    let xy = Q::new(61.into(), 30.into());
    assert_eq!(
        report.centroid_mm,
        [xy.clone(), xy, Q::from_integer(1.into())]
    );
    assert!(exact_polyhedral_centroid(&result.section.solid, report.exact_work).is_ok());
    assert!(exact_polyhedral_centroid(&result.section.solid, report.exact_work - 1).is_err());
}

#[test]
fn exact_centroid_follows_the_affine_map_of_an_oblique_solid() {
    use brep_core::{
        analysis::exact_polyhedral_centroid, planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let source = open_prism_with_nonbinary_cap_support();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let report = exact_polyhedral_centroid(&result.section.solid, 1_000_000).unwrap();
    assert_eq!(report.volume_mm3, Q::from_integer(12.into()));
    let coordinate = Q::new(3.into(), 2.into());
    assert_eq!(
        report.centroid_mm,
        [coordinate.clone(), Q::from_integer(0.into()), coordinate]
    );
}

#[test]
fn exact_inertia_includes_the_products_of_inertia_from_a_shifted_hole() {
    use brep_core::{
        analysis::exact_polyhedral_inertia, planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let report = exact_polyhedral_inertia(solid, 1_000_000).unwrap();
    let q = |a: i64, b: i64| Q::new(a.into(), b.into());
    assert_eq!(report.volume_mm3, q(30, 1));
    assert_eq!(report.centroid_mm, [q(61, 30), q(61, 30), q(1, 1)]);
    assert_eq!(
        report.inertia_mm5,
        [
            [q(1559, 30), q(8, 15), q(0, 1)],
            [q(8, 15), q(1559, 30), q(0, 1)],
            [q(0, 1), q(0, 1), q(1259, 15)],
        ]
    );
    assert!(exact_polyhedral_inertia(solid, report.exact_work).is_ok());
    assert!(exact_polyhedral_inertia(solid, report.exact_work - 1).is_err());
}

#[test]
fn exact_inertia_matches_the_affine_covariance_of_an_oblique_prism() {
    use brep_core::{
        analysis::exact_polyhedral_inertia, planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let source = open_prism_with_nonbinary_cap_support();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let report = exact_polyhedral_inertia(&result.section.solid, 1_000_000).unwrap();
    let expected = [[13, 2, -1], [2, 10, -2], [-1, -2, 13]];
    assert_eq!(
        report.inertia_mm5,
        expected.map(|row| row.map(|n| Q::from_integer(n.into())))
    );
}

#[test]
fn exact_area_subtracts_cap_holes_and_counts_their_walls() {
    use brep_core::{
        analysis::exact_polyhedral_area, planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let report = exact_polyhedral_area(solid, 1_000_000).unwrap();
    let mut expected = [4, 4, 4, 4, 64, 64, 64, 64, 225, 225].map(|n| Q::from_integer(n.into()));
    expected.sort();
    let mut actual = report.face_area_squared_mm4;
    actual.sort();
    assert_eq!(actual, expected);
    assert!(exact_polyhedral_area(solid, report.exact_work).is_ok());
    assert!(exact_polyhedral_area(solid, report.exact_work - 1).is_err());
}

#[test]
fn exact_oblique_area_retains_irrational_face_areas_as_square_roots() {
    use brep_core::{
        analysis::exact_polyhedral_area, planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let source = open_prism_with_nonbinary_cap_support();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let report = exact_polyhedral_area(&result.section.solid, 1_000_000).unwrap();
    let mut actual = report.face_area_squared_mm4;
    actual.sort();
    assert_eq!(
        actual,
        [20, 20, 32, 32, 44, 44].map(|n| Q::from_integer(n.into()))
    );
}

#[test]
fn rational_total_area_collapses_to_a_single_value_for_a_hollow_prism() {
    use brep_core::{
        analysis::bounded_polyhedral_area, planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let report = bounded_polyhedral_area(solid, 1e-10, 1_000_000).unwrap();
    assert_eq!(
        report.bounds_mm2,
        [70, 70].map(|n| Q::from_integer(n.into()))
    );
    assert!(bounded_polyhedral_area(solid, 1e-10, report.exact_work).is_ok());
    assert!(bounded_polyhedral_area(solid, 1e-10, report.exact_work - 1).is_err());
    assert!(bounded_polyhedral_area(solid, 0., 1_000_000).is_err());
}

#[test]
fn bounds_the_sum_of_irrational_areas_at_the_requested_width() {
    use brep_core::{
        analysis::bounded_polyhedral_area, planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let source = open_prism_with_nonbinary_cap_support();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let coarse = bounded_polyhedral_area(solid, 1e-5, 1_000_000).unwrap();
    let fine = bounded_polyhedral_area(solid, 1e-12, 1_000_000).unwrap();
    let [lower, upper] = fine.bounds_mm2;
    assert!(lower > coarse.bounds_mm2[0] && upper < coarse.bounds_mm2[1]);
    assert!(&upper - &lower <= Q::from_float(1e-12).unwrap());
    // An independent interval from exact square tests of decimal endpoints.
    let denominator = 10_i64.pow(15);
    let mut check_lower = Q::from_integer(0.into());
    let mut check_upper = check_lower.clone();
    for (square, floor) in [
        (20, 4_472_135_954_999_579_i64),
        (32, 5_656_854_249_492_380),
        (44, 6_633_249_580_710_799),
    ] {
        let lo = Q::new(floor.into(), denominator.into());
        let hi = Q::new((floor + 1).into(), denominator.into());
        let square = Q::from_integer(square.into());
        assert!(&lo * &lo < square && &hi * &hi > square);
        check_lower += &lo + &lo;
        check_upper += &hi + &hi;
    }
    assert!(lower <= check_lower && upper >= check_upper);
}

#[test]
fn counts_shared_polyhedral_edges_once_and_returns_exact_rational_lengths() {
    use brep_core::{
        analysis::bounded_polyhedral_edge_length, planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let lengths = bounded_polyhedral_edge_length(solid, 1e-9, 1_000_000).unwrap();
    assert_eq!(lengths.edge_count, 24);
    assert_eq!(
        lengths.bounds_mm,
        [56, 56].map(|n| Q::from_integer(n.into()))
    );
    assert!(bounded_polyhedral_edge_length(solid, 1e-9, lengths.exact_work).is_ok());
    assert!(bounded_polyhedral_edge_length(solid, 1e-9, lengths.exact_work - 1).is_err());
}

#[test]
fn bounds_irrational_oblique_edge_lengths() {
    use brep_core::{
        analysis::bounded_polyhedral_edge_length, planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let source = open_prism_with_nonbinary_cap_support();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let report = bounded_polyhedral_edge_length(&result.section.solid, 1e-9, 1_000_000).unwrap();
    let expected = Q::from_float(8. + 4. * (8_f64.sqrt() + 6_f64.sqrt())).unwrap();
    let [lower, upper] = report.bounds_mm;
    assert_eq!(report.edge_count, 12);
    assert!(lower < expected && expected < upper);
    assert!(upper - lower <= Q::from_float(1e-9).unwrap());
}

#[test]
fn exact_directional_bounds_ignore_the_unused_expanded_cap_support() {
    use brep_core::{
        analysis::exact_polyhedral_projection, planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let source = open_prism_with_nonbinary_cap_support();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let section = &result.section;
    let solid = &section.solid;
    let cap = &solid.model().faces[section.cap_faces[0]];
    assert!(
        cap.surface
            .control_points
            .iter()
            .flatten()
            .any(|p| p[1].abs() > 2.)
    );
    let report = exact_polyhedral_projection(solid, [0., 1., 0.], 1_000_000).unwrap();
    let expected = [-2, 2].map(|n| Q::from_integer(n.into()));
    assert_eq!(report.dot_bounds, expected);
    assert_eq!(report.witnesses_mm.map(|p| p[1].clone()), expected);
    assert!(exact_polyhedral_projection(solid, [0., 1., 0.], report.exact_work).is_ok());
    assert!(exact_polyhedral_projection(solid, [0., 1., 0.], report.exact_work - 1).is_err());
}

#[test]
fn exact_projection_scales_and_reverses_an_oblique_direction() {
    use brep_core::{
        analysis::exact_polyhedral_projection, planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let source = open_prism_with_nonbinary_cap_support();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let direction = [0.5, 0.5, -1.5];
    let report = exact_polyhedral_projection(solid, direction, 1_000_000).unwrap();
    assert_eq!(
        report.dot_bounds,
        [-3, 0].map(|n| Q::from_integer(n.into()))
    );
    for (point, bound) in report.witnesses_mm.iter().zip(report.dot_bounds) {
        let value: Q = point
            .iter()
            .zip(direction)
            .map(|(p, d)| p * Q::from_float(d).unwrap())
            .sum();
        assert_eq!(value, bound);
    }
    assert!(exact_polyhedral_projection(solid, [0.; 3], 1_000_000).is_err());
}

#[test]
fn exact_affine_frame_bounds_recover_dimensions_of_the_sheared_prism() {
    use brep_core::{
        analysis::exact_polyhedral_frame_bounds, planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let source = open_prism_with_nonbinary_cap_support();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let origin = [0.75, 0., 1.25];
    let basis = [[0.5, -0.5, 0.], [0.25, 0.5, 0.25], [0., 0., 1.]];
    let report = exact_polyhedral_frame_bounds(solid, origin, basis, 1_000_000).unwrap();
    assert_eq!(
        report.bounds,
        [[-1, 3], [-1, 3], [-1, 1]].map(|pair| pair.map(|n| Q::from_integer(n.into())))
    );
    for (axis, witnesses) in report.witnesses_mm.iter().enumerate() {
        for (point, bound) in witnesses.iter().zip(&report.bounds[axis]) {
            let exact = |n| Q::from_float(n).unwrap();
            let x = &point[0] - exact(origin[0]);
            let y = &point[1] - exact(origin[1]);
            let z = &point[2] - exact(origin[2]);
            let v = (&x + &y) / exact(0.75);
            let u = (&x - exact(0.25) * &v) / exact(0.5);
            let w = &z - exact(0.25) * &v;
            assert_eq!(&[u, v, w][axis], bound);
        }
    }
    assert!(exact_polyhedral_frame_bounds(solid, origin, basis, report.exact_work).is_ok());
    assert!(exact_polyhedral_frame_bounds(solid, origin, basis, report.exact_work - 1).is_err());
}

#[test]
fn exact_frame_projection_rejects_singular_or_nonfinite_frames() {
    use brep_core::{
        analysis::exact_polyhedral_frame_bounds, planar_cap::build_outward_inferred_section_solid,
    };
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    assert!(exact_polyhedral_frame_bounds(solid, [0.; 3], [[1., 0., 0.]; 3], 1_000_000).is_err());
    assert!(
        exact_polyhedral_frame_bounds(solid, [f64::INFINITY, 0., 0.], [[1., 0., 0.]; 3], 1_000_000)
            .is_err()
    );
}

fn shifted_oblique_solid(dx: f64) -> brep_core::closed_solid::QualifiedClosedSolid {
    use brep_core::planar_cap::build_outward_inferred_section_solid;
    let mut source = open_prism_with_nonbinary_cap_support();
    for vertex in &mut source.vertices {
        vertex.point[0] += dx;
    }
    for edge in &mut source.edges {
        for point in &mut edge.curve.control_points {
            point[0] += dx;
        }
    }
    for face in &mut source.faces {
        for point in face.surface.control_points.iter_mut().flatten() {
            point[0] += dx;
        }
    }
    source.rebuild_topology_ids();
    build_outward_inferred_section_solid(&source, section_solid_options())
        .unwrap()
        .section
        .solid
}

#[test]
fn certifies_exact_axis_clearance_and_preserves_witness_input_order() {
    use brep_core::analysis::exact_polyhedral_axis_separation;
    use num_rational::BigRational as Q;
    let a = shifted_oblique_solid(0.);
    let b = shifted_oblique_solid(5.);
    let q = |x: i64| Q::from_integer(x.into());
    let report = exact_polyhedral_axis_separation(&a, &b, [2., 0., 0.], 1_000_000).unwrap();
    assert_eq!(report.signed_gap, q(4));
    assert_eq!(report.separating_plane_distance_squared_mm2, Some(q(4)));
    assert_eq!(
        report.witnesses_mm.each_ref().map(|p| p[0].clone()),
        [q(3), q(5)]
    );
    let swapped =
        exact_polyhedral_axis_separation(&b, &a, [-2., 0., 0.], report.exact_work).unwrap();
    assert_eq!(swapped.signed_gap, q(4));
    assert_eq!(
        swapped.witnesses_mm.each_ref().map(|p| p[0].clone()),
        [q(5), q(3)]
    );
    assert!(exact_polyhedral_axis_separation(&a, &b, [2., 0., 0.], report.exact_work - 1).is_err());
}

#[test]
fn distinguishes_touching_and_overlapping_projections() {
    use brep_core::analysis::exact_polyhedral_axis_separation;
    use num_rational::BigRational as Q;
    let a = shifted_oblique_solid(0.);
    for (shift, gap) in [(3., 0), (2., -1)] {
        let b = shifted_oblique_solid(shift);
        let report = exact_polyhedral_axis_separation(&a, &b, [1., 0., 0.], 1_000_000).unwrap();
        assert_eq!(report.signed_gap, Q::from_integer(gap.into()));
        assert_eq!(
            report.separating_plane_distance_squared_mm2,
            (gap == 0).then(|| Q::from_integer(0.into()))
        );
    }
}

#[test]
fn classifies_planes_against_exact_trimmed_polyhedral_extrema() {
    use brep_core::analysis::{PolyhedralPlaneRelation::*, exact_polyhedral_plane_relation};
    use num_rational::BigRational as Q;
    let solid = shifted_oblique_solid(0.);
    let normal = [-1., -1., 3.];
    for (offset, expected, residuals) in [
        (-2., Positive, [2, 8]),
        (0., TouchesFromPositive, [0, 6]),
        (3., Crosses, [-3, 3]),
        (6., TouchesFromNegative, [-6, 0]),
        (8., Negative, [-8, -2]),
    ] {
        let report = exact_polyhedral_plane_relation(&solid, normal, offset, 1_000_000).unwrap();
        assert_eq!(report.relation, expected);
        assert_eq!(
            report.residual_bounds,
            residuals.map(|n| Q::from_integer(n.into()))
        );
        let distance = if matches!(expected, Positive | Negative) {
            4
        } else {
            0
        };
        assert_eq!(
            report.distance_squared_mm2,
            Q::new(distance.into(), 11.into())
        );
        for (point, residual) in report.witnesses_mm.iter().zip(report.residual_bounds) {
            assert_eq!(
                -&point[0] - &point[1] + &point[2] * Q::from_integer(3.into())
                    - Q::from_float(offset).unwrap(),
                residual
            );
        }
    }
}

#[test]
fn exact_plane_distance_is_invariant_under_rescaling_and_orientation() {
    use brep_core::analysis::{PolyhedralPlaneRelation::*, exact_polyhedral_plane_relation};
    let solid = shifted_oblique_solid(0.);
    let report = exact_polyhedral_plane_relation(&solid, [-1., -1., 3.], 8., 1_000_000).unwrap();
    let reversed =
        exact_polyhedral_plane_relation(&solid, [2., 2., -6.], -16., report.exact_work).unwrap();
    assert_eq!(report.relation, Negative);
    assert_eq!(reversed.relation, Positive);
    assert_eq!(report.distance_squared_mm2, reversed.distance_squared_mm2);
    assert!(
        exact_polyhedral_plane_relation(&solid, [-1., -1., 3.], 8., report.exact_work - 1).is_err()
    );
    assert!(exact_polyhedral_plane_relation(&solid, [0.; 3], 8., 1_000_000).is_err());
}

#[test]
fn exact_edge_plane_intersection_preserves_nonbinary_crossing_coordinates() {
    use brep_core::analysis::{
        PolyhedralEdgePlaneContact, exact_polyhedral_edge_plane_intersection,
    };
    use num_rational::BigRational as Q;
    let solid = shifted_oblique_solid(0.);
    let report =
        exact_polyhedral_edge_plane_intersection(&solid, [-1., -1., 3.], 2., 1_000_000).unwrap();
    assert_eq!(report.hits.len(), 4);
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    for hit in &report.hits {
        let PolyhedralEdgePlaneContact::Point { fraction, point_mm } = &hit.contact else {
            panic!("expected a crossing point");
        };
        assert!(*fraction == q(1, 3) || *fraction == q(2, 3));
        assert_eq!(
            -&point_mm[0] - &point_mm[1] + &point_mm[2] * q(3, 1),
            q(2, 1)
        );
    }
    assert!(
        exact_polyhedral_edge_plane_intersection(&solid, [-1., -1., 3.], 2., report.exact_work)
            .is_ok()
    );
    assert!(
        exact_polyhedral_edge_plane_intersection(&solid, [-1., -1., 3.], 2., report.exact_work - 1)
            .is_err()
    );
}

#[test]
fn reports_coplanar_spans_alongside_incident_endpoint_hits() {
    use brep_core::analysis::{
        PolyhedralEdgePlaneContact, exact_polyhedral_edge_plane_intersection,
    };
    use num_traits::Zero;
    let solid = shifted_oblique_solid(0.);
    let report =
        exact_polyhedral_edge_plane_intersection(&solid, [-1., -1., 3.], 6., 1_000_000).unwrap();
    let mut counts = [0; 2];
    for hit in report.hits {
        match hit.contact {
            PolyhedralEdgePlaneContact::Coplanar { .. } => counts[0] += 1,
            PolyhedralEdgePlaneContact::Point { fraction, .. } => {
                assert!(fraction.is_zero() || fraction.is_integer());
                counts[1] += 1;
            }
        }
    }
    assert_eq!(counts, [4, 4]);
    assert!(
        exact_polyhedral_edge_plane_intersection(&solid, [-1., -1., 3.], 7., 1_000_000)
            .unwrap()
            .hits
            .is_empty()
    );
}

#[test]
fn transverse_section_preserves_the_gap_where_a_cap_is_trimmed_by_a_hole() {
    use brep_core::{
        analysis::exact_polyhedral_transverse_section,
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let section = &result.section;
    let cut =
        exact_polyhedral_transverse_section(&section.solid, [1., 0., 0.], 1.5, 1_000_000).unwrap();
    let cap = cut
        .faces
        .iter()
        .find(|f| f.face == section.cap_faces[0])
        .unwrap();
    let point = |y: i64| {
        [
            Q::new(3.into(), 2.into()),
            Q::from_integer(y.into()),
            Q::from_integer(2.into()),
        ]
    };
    assert_eq!(
        cap.segments_mm,
        [[point(0), point(1)], [point(2), point(4)]]
    );
    assert_eq!(
        cut.faces.iter().map(|f| f.segments_mm.len()).sum::<usize>(),
        8
    );
    assert!(
        exact_polyhedral_transverse_section(&section.solid, [1., 0., 0.], 1.5, cut.exact_work)
            .is_ok()
    );
    assert!(
        exact_polyhedral_transverse_section(&section.solid, [1., 0., 0.], 1.5, cut.exact_work - 1)
            .is_err()
    );
}

#[test]
fn transverse_section_preserves_nonbinary_positions_and_refuses_endpoint_contacts() {
    use brep_core::analysis::exact_polyhedral_transverse_section;
    use num_rational::BigRational as Q;
    let solid = shifted_oblique_solid(0.);
    let cut = exact_polyhedral_transverse_section(&solid, [-1., -1., 3.], 2., 1_000_000).unwrap();
    assert_eq!(cut.faces.len(), 4);
    for face in cut.faces {
        assert_eq!(face.segments_mm.len(), 1);
        assert!(face.segments_mm[0].iter().all(|p| -&p[0] - &p[1]
            + &p[2] * Q::from_integer(3.into())
            == Q::from_integer(2.into())));
    }
    assert!(exact_polyhedral_transverse_section(&solid, [-1., -1., 3.], 6., 1_000_000).is_err());
}

#[test]
fn assembles_exact_outer_and_inner_section_cycles_with_face_ownership() {
    use brep_core::{
        analysis::{exact_polyhedral_section_loops, exact_polyhedral_transverse_section},
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let direction = [0., 0., 1.];
    let segments = exact_polyhedral_transverse_section(solid, direction, 1., 1_000_000).unwrap();
    let report = exact_polyhedral_section_loops(solid, direction, 1., 1_000_000).unwrap();
    let mut areas = Vec::new();
    for ring in &report.loops {
        assert_eq!(ring.points_mm.len(), 4);
        let mut twice_area = Q::from_integer(0.into());
        for (i, &face) in ring.faces.iter().enumerate() {
            let a = &ring.points_mm[i];
            let b = &ring.points_mm[(i + 1) % 4];
            twice_area += &a[0] * &b[1] - &a[1] * &b[0];
            assert!(
                segments
                    .faces
                    .iter()
                    .find(|entry| entry.face == face)
                    .unwrap()
                    .segments_mm
                    .iter()
                    .any(|segment| segment == &[a.clone(), b.clone()]
                        || segment == &[b.clone(), a.clone()])
            );
        }
        areas.push(num_traits::Signed::abs(&twice_area));
    }
    areas.sort();
    assert_eq!(areas, [2, 32].map(|n| Q::from_integer(n.into())));
    assert!(exact_polyhedral_section_loops(solid, direction, 1., report.exact_work).is_ok());
    assert!(exact_polyhedral_section_loops(solid, direction, 1., report.exact_work - 1).is_err());
}

#[test]
fn builds_two_disjoint_vertical_section_loops_and_allows_empty_sections() {
    use brep_core::{
        analysis::exact_polyhedral_section_loops, planar_cap::build_outward_inferred_section_solid,
    };
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let direction = [1., 0., 0.];
    let report = exact_polyhedral_section_loops(solid, direction, 1.5, 1_000_000).unwrap();
    assert_eq!(
        report
            .loops
            .iter()
            .map(|ring| ring.points_mm.len())
            .collect::<Vec<_>>(),
        [4, 4]
    );
    assert!(
        exact_polyhedral_section_loops(solid, direction, 5., 1_000_000)
            .unwrap()
            .loops
            .is_empty()
    );
}

#[test]
fn exact_section_area_subtracts_nested_holes_and_adds_disjoint_components() {
    use brep_core::{
        analysis::exact_polyhedral_section_area, planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let mut horizontal = exact_polyhedral_section_area(solid, [0., 0., 1.], 1., 1_000_000).unwrap();
    horizontal.depths.sort();
    assert_eq!(horizontal.depths, [0, 1]);
    assert_eq!(horizontal.area_squared_mm4, Q::from_integer(225.into()));
    let vertical = exact_polyhedral_section_area(solid, [1., 0., 0.], 1.5, 1_000_000).unwrap();
    assert_eq!(vertical.depths, [0, 0]);
    assert_eq!(vertical.area_squared_mm4, Q::from_integer(36.into()));
    assert!(exact_polyhedral_section_area(solid, [1., 0., 0.], 1.5, vertical.exact_work).is_ok());
    assert!(
        exact_polyhedral_section_area(solid, [1., 0., 0.], 1.5, vertical.exact_work - 1).is_err()
    );
}

#[test]
fn exact_oblique_section_area_has_the_correct_metric_scale() {
    use brep_core::analysis::exact_polyhedral_section_area;
    use num_rational::BigRational as Q;
    let solid = shifted_oblique_solid(0.);
    for (normal, offset) in [([-1., -1., 3.], 2.), ([2., 2., -6.], -4.)] {
        let report = exact_polyhedral_section_area(&solid, normal, offset, 1_000_000).unwrap();
        assert_eq!(report.depths, [0]);
        assert_eq!(report.area_squared_mm4, Q::from_integer(44.into()));
    }
}

#[test]
fn exact_section_centroid_accounts_for_an_off_center_hole() {
    use brep_core::{
        analysis::exact_polyhedral_section_centroid,
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let normal = [0., 0., 1.];
    let report = exact_polyhedral_section_centroid(solid, normal, 1., 1_000_000).unwrap();
    let q = |a: i64, b: i64| Q::new(a.into(), b.into());
    assert_eq!(report.centroid_mm, Some([q(61, 30), q(61, 30), q(1, 1)]));
    assert_eq!(report.section.area_squared_mm4, q(225, 1));
    assert!(
        exact_polyhedral_section_centroid(solid, normal, 1., report.section.exact_work).is_ok()
    );
    assert!(
        exact_polyhedral_section_centroid(solid, normal, 1., report.section.exact_work - 1)
            .is_err()
    );
    assert!(
        exact_polyhedral_section_centroid(solid, normal, 3., 1_000_000)
            .unwrap()
            .centroid_mm
            .is_none()
    );
}

#[test]
fn exact_section_centroid_lifts_a_nonbinary_point_to_the_oblique_plane() {
    use brep_core::analysis::exact_polyhedral_section_centroid;
    use num_rational::BigRational as Q;
    let solid = shifted_oblique_solid(0.);
    let report = exact_polyhedral_section_centroid(&solid, [-1., -1., 3.], 2., 1_000_000).unwrap();
    let q = |a: i64, b: i64| Q::new(a.into(), b.into());
    assert_eq!(report.centroid_mm, Some([q(3, 2), q(0, 1), q(7, 6)]));
    assert_eq!(report.section.area_squared_mm4, q(44, 1));
}

#[test]
fn section_perimeter_includes_holes_and_disjoint_contours() {
    use brep_core::{
        analysis::bounded_polyhedral_section_perimeter,
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    for (normal, offset, length, count) in [
        ([0., 0., 1.], 1., 20, 2),
        ([1., 0., 0.], 1.5, 14, 2),
        ([1., 0., 0.], 5., 0, 0),
    ] {
        let report =
            bounded_polyhedral_section_perimeter(solid, normal, offset, 1e-10, 1_000_000).unwrap();
        assert_eq!(report.loops.len(), count);
        assert_eq!(
            report.bounds_mm,
            [length; 2].map(|n| Q::from_integer(n.into()))
        );
    }
}

#[test]
fn bounds_exact_oblique_section_length_at_the_requested_precision() {
    use brep_core::analysis::bounded_polyhedral_section_perimeter;
    use num_rational::BigRational as Q;
    let solid = shifted_oblique_solid(0.);
    let normal = [-1., -1., 3.];
    let report =
        bounded_polyhedral_section_perimeter(&solid, normal, 2., 1e-10, 1_000_000).unwrap();
    let [lower, upper] = report.bounds_mm;
    let expected = Q::from_float(2. * (8_f64.sqrt() + 6_f64.sqrt())).unwrap();
    assert!(lower < expected && expected < upper);
    assert!(upper - lower <= Q::from_float(1e-10).unwrap());
    assert!(
        bounded_polyhedral_section_perimeter(&solid, normal, 2., 1e-10, report.exact_work).is_ok()
    );
    assert!(
        bounded_polyhedral_section_perimeter(&solid, normal, 2., 1e-10, report.exact_work - 1)
            .is_err()
    );
    assert!(bounded_polyhedral_section_perimeter(&solid, normal, 2., f64::NAN, 1_000_000).is_err());
}

#[test]
fn classifies_material_hole_boundary_and_off_plane_points_in_exact_sections() {
    use brep_core::{
        analysis::{
            PolyhedralSectionPointLocation::*, exact_polyhedral_section_loops,
            exact_polyhedral_section_point,
        },
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let normal = [0., 0., 1.];
    let q = |n: i64| Q::new(n.into(), 2.into());
    for (p, expected) in [
        ([1, 1, 2], Material),
        ([3, 3, 2], Outside),
        ([9, 3, 2], Outside),
        ([1, 1, 3], OffPlane),
    ] {
        let result =
            exact_polyhedral_section_point(solid, normal, 1., &p.map(q), 1_000_000).unwrap();
        assert_eq!(result.location, expected);
        assert!(
            exact_polyhedral_section_point(solid, normal, 1., &p.map(q), result.exact_work).is_ok()
        );
        assert!(
            exact_polyhedral_section_point(solid, normal, 1., &p.map(q), result.exact_work - 1)
                .is_err()
        );
    }
    let point = [2, 3, 2].map(q);
    let boundary = exact_polyhedral_section_point(solid, normal, 1., &point, 1_000_000).unwrap();
    let Boundary {
        ring,
        segment,
        face,
    } = boundary.location
    else {
        panic!("expected hole boundary")
    };
    let contours = exact_polyhedral_section_loops(solid, normal, 1., 1_000_000).unwrap();
    assert_eq!(face, contours.loops[ring].faces[segment]);
    let [a, b] = [
        segment,
        (segment + 1) % contours.loops[ring].points_mm.len(),
    ]
    .map(|i| &contours.loops[ring].points_mm[i]);
    assert_eq!(a[0], point[0]);
    assert_eq!(b[0], point[0]);
}

#[test]
fn locates_a_nonbinary_point_in_an_oblique_section() {
    use brep_core::analysis::{
        PolyhedralSectionPointLocation::Material, exact_polyhedral_section_point,
    };
    use num_rational::BigRational as Q;
    let solid = shifted_oblique_solid(0.);
    let point = [
        Q::new(3.into(), 2.into()),
        Q::from_integer(0.into()),
        Q::new(7.into(), 6.into()),
    ];
    let result =
        exact_polyhedral_section_point(&solid, [-1., -1., 3.], 2., &point, 1_000_000).unwrap();
    assert_eq!(result.location, Material);
}

#[test]
fn finds_the_exact_nearest_point_on_a_hole_from_off_plane() {
    use brep_core::{
        analysis::{exact_polyhedral_section_distance, exact_polyhedral_section_loops},
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |a: i64, b: i64| Q::new(a.into(), b.into());
    let normal = [0., 0., 1.];
    let point = [q(5, 4), q(3, 2), q(2, 1)];
    let report = exact_polyhedral_section_distance(solid, normal, 1., &point, 1_000_000).unwrap();
    let nearest = report.nearest.unwrap();
    assert_eq!(nearest.distance_squared_mm2, q(17, 16));
    assert_eq!(nearest.point_mm, [q(1, 1), q(3, 2), q(1, 1)]);
    let section = exact_polyhedral_section_loops(solid, normal, 1., 1_000_000).unwrap();
    assert_eq!(
        section.loops[nearest.ring].faces[nearest.segment],
        nearest.face
    );
    assert!(
        exact_polyhedral_section_distance(solid, normal, 1., &point, report.exact_work).is_ok()
    );
    assert!(
        exact_polyhedral_section_distance(solid, normal, 1., &point, report.exact_work - 1)
            .is_err()
    );
}

#[test]
fn clamps_to_a_section_vertex_and_reports_empty_sections() {
    use brep_core::{
        analysis::exact_polyhedral_section_distance,
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64| Q::from_integer(n.into());
    let query = [-1, -2, 1].map(q);
    let normal = [0., 0., 1.];
    let report = exact_polyhedral_section_distance(solid, normal, 1., &query, 1_000_000).unwrap();
    let nearest = report.nearest.unwrap();
    assert_eq!(nearest.point_mm, [0, 0, 1].map(q));
    assert_eq!(nearest.distance_squared_mm2, q(5));
    assert!(
        exact_polyhedral_section_distance(solid, normal, 5., &query, 1_000_000)
            .unwrap()
            .nearest
            .is_none()
    );
}

#[test]
fn finds_projected_interior_and_hole_boundary_witnesses_for_material_distance() {
    use brep_core::{
        analysis::{PolyhedralSectionMaterialWitness, exact_polyhedral_section_material_distance},
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |a: i64, b: i64| Q::new(a.into(), b.into());
    let normal = [0., 0., 1.];
    let point = [q(1, 2), q(1, 2), q(2, 1)];
    let report =
        exact_polyhedral_section_material_distance(solid, normal, 1., &point, 1_000_000).unwrap();
    assert_eq!(
        report.nearest,
        Some(PolyhedralSectionMaterialWitness::Interior {
            point_mm: [q(1, 2), q(1, 2), q(1, 1)],
            distance_squared_mm2: q(1, 1),
        })
    );
    assert!(
        exact_polyhedral_section_material_distance(solid, normal, 1., &point, report.exact_work)
            .is_ok()
    );
    assert!(
        exact_polyhedral_section_material_distance(
            solid,
            normal,
            1.,
            &point,
            report.exact_work - 1
        )
        .is_err()
    );

    for (x, distance) in [(q(5, 4), q(17, 16)), (q(1, 1), q(1, 1))] {
        let point = [x, q(3, 2), q(2, 1)];
        let report =
            exact_polyhedral_section_material_distance(solid, normal, 1., &point, 1_000_000)
                .unwrap();
        let Some(PolyhedralSectionMaterialWitness::Boundary(hit)) = report.nearest else {
            panic!("expected hole boundary")
        };
        assert_eq!(hit.point_mm, [q(1, 1), q(3, 2), q(1, 1)]);
        assert_eq!(hit.distance_squared_mm2, distance);
    }
    assert!(
        exact_polyhedral_section_material_distance(solid, normal, 5., &point, 1_000_000)
            .unwrap()
            .nearest
            .is_none()
    );
}

#[test]
fn uses_true_orthogonal_projection_on_an_oblique_section() {
    use brep_core::analysis::{
        PolyhedralSectionMaterialWitness, exact_polyhedral_section_material_distance,
    };
    use num_rational::BigRational as Q;
    let solid = shifted_oblique_solid(0.);
    let q = |a: i64, b: i64| Q::new(a.into(), b.into());
    let projected = [q(3, 2), q(0, 1), q(7, 6)];
    let query = [q(1, 2), q(-1, 1), q(25, 6)];
    let report =
        exact_polyhedral_section_material_distance(&solid, [-1., -1., 3.], 2., &query, 1_000_000)
            .unwrap();
    assert_eq!(
        report.nearest,
        Some(PolyhedralSectionMaterialWitness::Interior {
            point_mm: projected,
            distance_squared_mm2: q(11, 1),
        })
    );
}

#[test]
fn clips_section_material_into_exact_intervals_around_a_hole() {
    use brep_core::{
        analysis::exact_polyhedral_section_clip, planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let normal = [0., 0., 1.];
    let segment = [[q(-1, 1), q(3, 2), q(1, 1)], [q(5, 1), q(3, 2), q(1, 1)]];
    let report = exact_polyhedral_section_clip(solid, normal, 1., &segment, 1_000_000).unwrap();
    assert_eq!(report.intervals, [[q(1, 6), q(1, 3)], [q(1, 2), q(5, 6)]]);
    assert!(exact_polyhedral_section_clip(solid, normal, 1., &segment, report.exact_work).is_ok());
    assert!(
        exact_polyhedral_section_clip(solid, normal, 1., &segment, report.exact_work - 1).is_err()
    );
}

#[test]
fn section_clip_preserves_boundary_overlap_and_isolated_vertex_contact() {
    use brep_core::{
        analysis::exact_polyhedral_section_clip, planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let normal = [0., 0., 1.];
    let overlap = [[q(1, 2), q(1, 1), q(1, 1)], [q(5, 2), q(1, 1), q(1, 1)]];
    let tangent = [[q(-1, 1), q(1, 1), q(1, 1)], [q(1, 1), q(-1, 1), q(1, 1)]];
    assert_eq!(
        exact_polyhedral_section_clip(solid, normal, 1., &overlap, 1_000_000)
            .unwrap()
            .intervals,
        [[q(0, 1), q(1, 1)]]
    );
    assert_eq!(
        exact_polyhedral_section_clip(solid, normal, 1., &tangent, 1_000_000)
            .unwrap()
            .intervals,
        [[q(1, 2), q(1, 2)]]
    );
    assert!(exact_polyhedral_section_clip(solid, normal, 2.5, &tangent, 1_000_000).is_err());
}

#[test]
fn clips_multiple_polyline_spans_with_one_section_and_exact_local_parameters() {
    use brep_core::{
        analysis::{exact_polyhedral_section_clip, exact_polyhedral_section_clip_polyline},
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |a: i64, b: i64| Q::new(a.into(), b.into());
    let point = |x: i64, y: i64| [q(x, 2), q(y, 2), q(1, 1)];
    let path = [point(-2, 3), point(10, 3), point(10, 3), point(-2, 3)];
    let normal = [0., 0., 1.];
    let report =
        exact_polyhedral_section_clip_polyline(solid, normal, 1., &path, 1_000_000).unwrap();
    assert_eq!(
        report.span_intervals,
        [
            vec![[q(1, 6), q(1, 3)], [q(1, 2), q(5, 6)]],
            vec![],
            vec![[q(1, 6), q(1, 2)], [q(2, 3), q(5, 6)]],
        ]
    );
    let separate_work: u64 = path
        .windows(2)
        .map(|pair| {
            let endpoints = [pair[0].clone(), pair[1].clone()];
            exact_polyhedral_section_clip(solid, normal, 1., &endpoints, 1_000_000)
                .unwrap()
                .exact_work
        })
        .sum();
    assert!(report.exact_work < separate_work);
    assert!(
        exact_polyhedral_section_clip_polyline(solid, normal, 1., &path, report.exact_work).is_ok()
    );
    assert!(
        exact_polyhedral_section_clip_polyline(solid, normal, 1., &path, report.exact_work - 1)
            .is_err()
    );
}

#[test]
fn polyline_clip_preserves_repeated_inside_vertices_and_rejects_off_plane_tails() {
    use brep_core::{
        analysis::exact_polyhedral_section_clip_polyline,
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |a: i64| Q::from_integer(a.into());
    let point = [0, 0, 1].map(q);
    let mut path = [point.clone(), point.clone(), [4, 0, 1].map(q)];
    let normal = [0., 0., 1.];
    let report =
        exact_polyhedral_section_clip_polyline(solid, normal, 1., &path, 1_000_000).unwrap();
    assert_eq!(
        report.span_intervals,
        [vec![[q(0), q(1)]], vec![[q(0), q(1)]]]
    );
    path[2][2] = q(2);
    assert!(exact_polyhedral_section_clip_polyline(solid, normal, 1., &path, 1_000_000).is_err());
}

#[test]
fn section_path_length_counts_repeated_traversals_and_excludes_holes() {
    use brep_core::{
        analysis::bounded_polyhedral_section_path_length,
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let p = |x: i64| {
        [
            Q::from_integer(x.into()),
            Q::new(3.into(), 2.into()),
            Q::from_integer(1.into()),
        ]
    };
    let path = [p(-1), p(5), p(5), p(-1)];
    let normal = [0., 0., 1.];
    let report =
        bounded_polyhedral_section_path_length(solid, normal, 1., &path, 1e-10, 1_000_000).unwrap();
    assert_eq!(
        report.bounds_mm,
        [Q::from_integer(6.into()), Q::from_integer(6.into())]
    );
    assert!(
        bounded_polyhedral_section_path_length(solid, normal, 1., &path, 1e-10, report.exact_work)
            .is_ok()
    );
    assert!(
        bounded_polyhedral_section_path_length(
            solid,
            normal,
            1.,
            &path,
            1e-10,
            report.exact_work - 1
        )
        .is_err()
    );
}

#[test]
fn section_path_length_encloses_an_irrational_oblique_chord() {
    use brep_core::analysis::bounded_polyhedral_section_path_length;
    use num_rational::BigRational as Q;
    let solid = shifted_oblique_solid(0.);
    let q = |a: i64, b: i64| Q::new(a.into(), b.into());
    let path = [[q(0, 1), q(3, 2), q(7, 6)], [q(3, 1), q(-3, 2), q(7, 6)]];
    let report =
        bounded_polyhedral_section_path_length(&solid, [-1., -1., 3.], 2., &path, 1e-12, 1_000_000)
            .unwrap();
    let [lower, upper] = report.bounds_mm;
    assert!(lower.pow(2) < q(8, 1) && q(8, 1) < upper.pow(2));
    assert!(upper - lower <= Q::from_float(1e-12).unwrap());
}

#[test]
fn spatial_line_clip_recovers_material_intervals_and_an_inside_start() {
    use brep_core::{
        analysis::exact_polyhedral_segment_clip, planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |a: i64, b: i64| Q::new(a.into(), b.into());
    let point = |x: i64| [q(x, 2), q(3, 2), q(1, 1)];
    let endpoints = [point(-2), point(10)];
    let report = exact_polyhedral_segment_clip(solid, &endpoints, 1_000_000).unwrap();
    assert_eq!(report.intervals, [[q(1, 6), q(1, 3)], [q(1, 2), q(5, 6)]]);
    assert!(exact_polyhedral_segment_clip(solid, &endpoints, report.exact_work).is_ok());
    assert!(exact_polyhedral_segment_clip(solid, &endpoints, report.exact_work - 1).is_err());
    let inside = exact_polyhedral_segment_clip(solid, &[point(1), point(3)], 1_000_000).unwrap();
    assert_eq!(inside.intervals, [[q(0, 1), q(1, 2)]]);
}

#[test]
fn spatial_line_clip_preserves_oblique_fractions_and_refuses_edge_contacts() {
    use brep_core::analysis::exact_polyhedral_segment_clip;
    use num_rational::BigRational as Q;
    let solid = shifted_oblique_solid(0.);
    let q = |a: i64, b: i64| Q::new(a.into(), b.into());
    let path = [[q(3, 2), q(0, 1), q(-1, 2)], [q(3, 2), q(0, 1), q(7, 2)]];
    let report = exact_polyhedral_segment_clip(&solid, &path, 1_000_000).unwrap();
    assert_eq!(report.intervals, [[q(1, 4), q(3, 4)]]);
    let edge_path = [[q(0, 1), q(0, 1), q(-1, 1)], [q(0, 1), q(0, 1), q(3, 1)]];
    assert!(exact_polyhedral_segment_clip(&solid, &edge_path, 1_000_000).is_err());
}

#[test]
fn spatial_clip_ignores_coplanar_support_outside_the_retained_wall() {
    use brep_core::{
        analysis::exact_polyhedral_segment_clip, planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let p = |x, y, z| [x, y, z].map(|n: i64| Q::from_integer(n.into()));
    let line = [p(3, 1, -1), p(3, 1, 3)];
    let report = exact_polyhedral_segment_clip(solid, &line, 1_000_000).unwrap();
    let q = |n: i64| Q::new(n.into(), 4.into());
    assert_eq!(report.intervals, [[q(1), q(3)]]);
    assert!(exact_polyhedral_segment_clip(solid, &line, report.exact_work).is_ok());
    assert!(exact_polyhedral_segment_clip(solid, &line, report.exact_work - 1).is_err());
    assert!(
        exact_polyhedral_segment_clip(solid, &[p(5, 0, -1), p(5, 0, 3)], 1_000_000)
            .unwrap()
            .intervals
            .is_empty()
    );
}

#[test]
fn spatial_clip_still_refuses_contact_with_a_retained_coplanar_wall() {
    use brep_core::{
        analysis::exact_polyhedral_segment_clip, planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let p = |z: i64| {
        [
            Q::new(3.into(), 2.into()),
            Q::from_integer(1.into()),
            Q::from_integer(z.into()),
        ]
    };
    assert!(exact_polyhedral_segment_clip(solid, &[p(-1), p(3)], 1_000_000).is_err());
}

#[test]
fn exact_spatial_classification_distinguishes_material_hole_and_boundary() {
    use brep_core::{
        analysis::{PolyhedralPointLocation::*, exact_polyhedral_point_location},
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64| Q::new(n.into(), 2.into());
    for (p, expected) in [
        ([1, 1, 1], Material),
        ([3, 3, 2], Exterior),
        ([10, 3, 2], Exterior),
    ] {
        let p = p.map(q);
        let report = exact_polyhedral_point_location(solid, &p, 1_000_000).unwrap();
        assert_eq!(report.location, expected);
        assert!(exact_polyhedral_point_location(solid, &p, report.exact_work).is_ok());
        assert!(exact_polyhedral_point_location(solid, &p, report.exact_work - 1).is_err());
    }
    for p in [[2, 3, 2], [0, 0, 0], [8, 8, 2]] {
        let report = exact_polyhedral_point_location(solid, &p.map(q), 1_000_000).unwrap();
        assert!(matches!(report.location, Boundary { face } if face < solid.model().faces.len()));
    }
}

#[test]
fn exact_spatial_classification_admits_rational_coordinates_on_an_oblique_solid() {
    use brep_core::analysis::{PolyhedralPointLocation::Material, exact_polyhedral_point_location};
    use num_rational::BigRational as Q;
    let solid = shifted_oblique_solid(0.);
    let point = [
        Q::new(3.into(), 2.into()),
        Q::from_integer(0.into()),
        Q::new(7.into(), 6.into()),
    ];
    assert_eq!(
        exact_polyhedral_point_location(&solid, &point, 1_000_000)
            .unwrap()
            .location,
        Material
    );
}

#[test]
fn exact_boundary_distance_respects_the_hole_in_a_cap() {
    use brep_core::{
        analysis::exact_polyhedral_boundary_distance,
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |a: i64, b: i64| Q::new(a.into(), b.into());
    let query = [q(5, 4), q(3, 2), q(3, 1)];
    let report = exact_polyhedral_boundary_distance(solid, &query, 1_000_000).unwrap();
    assert_eq!(report.point_mm, [q(1, 1), q(3, 2), q(2, 1)]);
    assert_eq!(report.distance_squared_mm2, q(17, 16));
    assert!(report.face < solid.model().faces.len());
    assert!(exact_polyhedral_boundary_distance(solid, &query, report.exact_work).is_ok());
    assert!(exact_polyhedral_boundary_distance(solid, &query, report.exact_work - 1).is_err());
}

#[test]
fn exact_boundary_distance_projects_orthogonally_onto_an_oblique_cap() {
    use brep_core::analysis::exact_polyhedral_boundary_distance;
    use num_rational::BigRational as Q;
    let solid = shifted_oblique_solid(0.);
    let q = |a: i64, b: i64| Q::new(a.into(), b.into());
    let query = [q(23, 16), q(-1, 16), q(43, 16)];
    let report = exact_polyhedral_boundary_distance(&solid, &query, 1_000_000).unwrap();
    assert_eq!(report.point_mm, [q(3, 2), q(0, 1), q(5, 2)]);
    assert_eq!(report.distance_squared_mm2, q(11, 256));
    assert_eq!(
        exact_polyhedral_boundary_distance(&solid, &report.point_mm, 1_000_000)
            .unwrap()
            .distance_squared_mm2,
        q(0, 1)
    );
}

#[test]
fn signed_distance_distinguishes_material_from_a_hole_and_its_wall() {
    use brep_core::{
        analysis::exact_polyhedral_signed_distance,
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    for (x, signed) in [(3, -1), (5, 1), (4, 0)] {
        let point = [q(x, 4), q(3, 2), q(1, 1)];
        let report = exact_polyhedral_signed_distance(solid, &point, 1_000_000).unwrap();
        assert_eq!(report.signed_distance_squared_mm2, q(signed, 16));
        assert_eq!(report.nearest_mm, [q(1, 1), q(3, 2), q(1, 1)]);
        assert!(exact_polyhedral_signed_distance(solid, &point, report.exact_work).is_ok());
        assert!(exact_polyhedral_signed_distance(solid, &point, report.exact_work - 1).is_err());
    }
}

#[test]
fn signed_distance_uses_exact_oblique_face_projection() {
    use brep_core::analysis::{
        exact_polyhedral_boundary_distance, exact_polyhedral_point_location,
        exact_polyhedral_signed_distance,
    };
    use num_rational::BigRational as Q;
    let solid = shifted_oblique_solid(0.);
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    for direction in [-1, 1] {
        let point = [
            q(24 - direction, 16),
            q(-direction, 16),
            q(40 + 3 * direction, 16),
        ];
        let report = exact_polyhedral_signed_distance(&solid, &point, 1_000_000).unwrap();
        assert_eq!(report.signed_distance_squared_mm2, q(11 * direction, 256));
        assert_eq!(report.nearest_mm, [q(3, 2), q(0, 1), q(5, 2)]);
        let nearest = exact_polyhedral_boundary_distance(&solid, &point, 1_000_000).unwrap();
        let location = exact_polyhedral_point_location(&solid, &point, 1_000_000).unwrap();
        assert!(report.exact_work < nearest.exact_work + location.exact_work);
    }
}

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

#[test]
fn batches_exact_signed_distances_with_order_and_shared_preparation() {
    use brep_core::{
        analysis::{exact_polyhedral_signed_distance, exact_polyhedral_signed_distance_batch},
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |a: i64, b: i64| Q::new(a.into(), b.into());
    let points = [5, 3, 4, 5].map(|x| [q(x, 4), q(3, 2), q(1, 1)]);
    let report = exact_polyhedral_signed_distance_batch(solid, &points, 1_000_000).unwrap();
    let mut separate_work = 0;
    for ((point, sample), sign) in points.iter().zip(&report.samples).zip([1, -1, 0, 1]) {
        let individual = exact_polyhedral_signed_distance(solid, point, 1_000_000).unwrap();
        assert_eq!(sample.signed_distance_squared_mm2, q(sign, 16));
        assert_eq!(sample.nearest_mm, individual.nearest_mm);
        assert_eq!(sample.face, individual.face);
        assert!(sample.exact_work < individual.exact_work);
        separate_work += individual.exact_work;
    }
    assert!(report.exact_work < separate_work);
    assert!(exact_polyhedral_signed_distance_batch(solid, &points, report.exact_work).is_ok());
    assert!(exact_polyhedral_signed_distance_batch(solid, &points, report.exact_work - 1).is_err());
}

#[test]
fn signed_distance_batch_refuses_out_of_budget_coordinates_and_accepts_empty_input() {
    use brep_core::analysis::exact_polyhedral_signed_distance_batch;
    use num_rational::BigRational as Q;
    let solid = shifted_oblique_solid(0.);
    let zero = Q::from_integer(0.into());
    let origin = std::array::from_fn(|_| zero.clone());
    let large = Q::from_integer(num_bigint::BigInt::from(1) << 8192);
    let points = [origin, [large, zero.clone(), zero]];
    assert!(exact_polyhedral_signed_distance_batch(&solid, &points, 1_000_000).is_err());
    assert!(
        exact_polyhedral_signed_distance_batch(&solid, &[], 1_000_000)
            .unwrap()
            .samples
            .is_empty()
    );
}

#[test]
fn exact_ball_clearance_distinguishes_both_sides_of_a_hole_wall() {
    use brep_core::{
        analysis::{PolyhedralBallRelation::*, exact_polyhedral_ball_clearance},
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    for (x, expected) in [
        (3, [Inside, InsideTangent, CrossesBoundary]),
        (5, [Outside, OutsideTangent, CrossesBoundary]),
    ] {
        let center = [q(x, 4), q(3, 2), q(1, 1)];
        for (radius, expected) in [q(1, 8), q(1, 4), q(3, 8)].into_iter().zip(expected) {
            let report =
                exact_polyhedral_ball_clearance(solid, &center, &radius, 1_000_000).unwrap();
            assert_eq!(report.relation, expected);
            assert_eq!(report.nearest_mm, [q(1, 1), q(3, 2), q(1, 1)]);
            assert!(
                exact_polyhedral_ball_clearance(solid, &center, &radius, report.exact_work).is_ok()
            );
            assert!(
                exact_polyhedral_ball_clearance(solid, &center, &radius, report.exact_work - 1)
                    .is_err()
            );
        }
    }
}

#[test]
fn exact_ball_clearance_handles_zero_radius_on_a_retained_face() {
    use brep_core::analysis::{PolyhedralBallRelation::*, exact_polyhedral_ball_clearance};
    use num_rational::BigRational as Q;
    let solid = shifted_oblique_solid(0.);
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let center = [q(3, 2), q(0, 1), q(5, 2)];
    let report = exact_polyhedral_ball_clearance(&solid, &center, &q(0, 1), 1_000_000).unwrap();
    assert_eq!(report.relation, BoundaryPoint);
    assert_eq!(report.nearest_mm, center);
    assert_eq!(
        exact_polyhedral_ball_clearance(&solid, &center, &q(1, 8), 1_000_000)
            .unwrap()
            .relation,
        CrossesBoundary
    );
    assert!(exact_polyhedral_ball_clearance(&solid, &center, &q(-1, 8), 1_000_000).is_err());
}

#[test]
fn swept_ball_detects_boundary_contact_between_clear_endpoints() {
    use brep_core::{
        analysis::{
            PolyhedralBallRelation, exact_polyhedral_ball_clearance,
            exact_polyhedral_swept_ball_clearance,
        },
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    use std::cmp::Ordering;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let p = |x, y, z| [x, y, z].map(|n: i64| Q::from_integer(n.into()));
    let path = [p(-1, 3, 1), p(5, 3, 1)];
    let radius = Q::new(1.into(), 2.into());
    for center in &path {
        assert_eq!(
            exact_polyhedral_ball_clearance(solid, center, &radius, 1_000_000)
                .unwrap()
                .relation,
            PolyhedralBallRelation::Outside
        );
    }
    let report = exact_polyhedral_swept_ball_clearance(solid, &path, &radius, 1_000_000).unwrap();
    assert_eq!(report.clearance, Ordering::Less);
    assert_eq!(report.distance_squared_mm2, Q::from_integer(0.into()));
    assert_eq!(report.center_mm, report.boundary_mm);
    assert!(
        exact_polyhedral_swept_ball_clearance(solid, &path, &radius, report.exact_work).is_ok()
    );
    assert!(
        exact_polyhedral_swept_ball_clearance(solid, &path, &radius, report.exact_work - 1)
            .is_err()
    );
}

#[test]
fn swept_ball_finds_exact_tangency_above_a_trimmed_cap() {
    use brep_core::{
        analysis::exact_polyhedral_swept_ball_clearance,
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    use std::cmp::Ordering;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let p = |x, y, z| [x, y, z].map(|n: i64| Q::from_integer(n.into()));
    let report = exact_polyhedral_swept_ball_clearance(
        solid,
        &[p(-1, 3, 3), p(5, 3, 3)],
        &Q::from_integer(1.into()),
        1_000_000,
    )
    .unwrap();
    assert_eq!(report.clearance, Ordering::Equal);
    assert_eq!(report.distance_squared_mm2, Q::from_integer(1.into()));
    assert_eq!(&report.center_mm[..2], &report.boundary_mm[..2]);
    assert_eq!(report.boundary_mm[2], Q::from_integer(2.into()));
}

#[test]
fn swept_path_finds_the_first_minimizing_span_and_reuses_faces() {
    use brep_core::analysis::{
        exact_polyhedral_swept_ball_clearance, exact_polyhedral_swept_path_clearance,
    };
    use num_rational::BigRational as Q;
    use std::cmp::Ordering;
    let solid = shifted_oblique_solid(0.);
    let p = |x, y, z| [x, y, z].map(|n: i64| Q::from_integer(n.into()));
    let path = [p(-2, 0, 5), p(2, 0, 5), p(2, 0, 4), p(2, 0, 4)];
    let radius = Q::from_integer(1.into());
    let report = exact_polyhedral_swept_path_clearance(&solid, &path, &radius, 1_000_000).unwrap();
    assert_eq!(report.span, 1);
    assert_eq!(report.nearest.clearance, Ordering::Greater);
    let mut work = 0;
    for (i, pair) in path.windows(2).enumerate() {
        let single = exact_polyhedral_swept_ball_clearance(
            &solid,
            &[pair[0].clone(), pair[1].clone()],
            &radius,
            1_000_000,
        )
        .unwrap();
        assert!(single.distance_squared_mm2 >= report.nearest.distance_squared_mm2);
        if i == report.span {
            assert_eq!(single.center_mm, report.nearest.center_mm);
            assert_eq!(single.boundary_mm, report.nearest.boundary_mm);
        }
        work += single.exact_work;
    }
    assert!(report.exact_work < work);
    assert!(
        exact_polyhedral_swept_path_clearance(&solid, &path, &radius, report.exact_work).is_ok()
    );
    assert!(
        exact_polyhedral_swept_path_clearance(&solid, &path, &radius, report.exact_work - 1)
            .is_err()
    );
}

#[test]
fn swept_path_reports_interior_contact_and_validates_the_entire_input() {
    use brep_core::analysis::exact_polyhedral_swept_path_clearance;
    use num_rational::BigRational as Q;
    use std::cmp::Ordering;
    let solid = shifted_oblique_solid(0.);
    let p = |x, y, z| [x, y, z].map(|n: i64| Q::from_integer(n.into()));
    let radius = Q::from_integer(1.into());
    let mut path = [p(1, 0, 5), p(1, 0, 4), p(1, 0, -1), p(1, 0, 5)];
    let report = exact_polyhedral_swept_path_clearance(&solid, &path, &radius, 1_000_000).unwrap();
    assert_eq!(report.span, 1);
    assert_eq!(report.nearest.clearance, Ordering::Less);
    assert_eq!(report.nearest.center_mm, report.nearest.boundary_mm);
    path[3][0] = Q::from_integer(num_bigint::BigInt::from(1) << 8192);
    assert!(exact_polyhedral_swept_path_clearance(&solid, &path, &radius, 1_000_000).is_err());
}

#[test]
fn swept_material_relation_distinguishes_hole_clearance_and_containment() {
    use brep_core::{
        analysis::{PolyhedralSweptPathRelation::*, exact_polyhedral_swept_path_material},
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    for (x, expected) in [
        (3, [Inside, InsideTangent, CrossesBoundary]),
        (5, [Outside, OutsideTangent, CrossesBoundary]),
    ] {
        let path = [[q(x, 4), q(3, 2), q(1, 2)], [q(x, 4), q(3, 2), q(3, 2)]];
        for (radius, expected) in [q(1, 8), q(1, 4), q(3, 8)].into_iter().zip(expected) {
            let report =
                exact_polyhedral_swept_path_material(solid, &path, &radius, 1_000_000).unwrap();
            assert_eq!(report.relation, expected);
            assert_eq!(report.clearance.nearest.distance_squared_mm2, q(1, 16));
            assert!(
                exact_polyhedral_swept_path_material(
                    solid,
                    &path,
                    &radius,
                    report.clearance.exact_work
                )
                .is_ok()
            );
            assert!(
                exact_polyhedral_swept_path_material(
                    solid,
                    &path,
                    &radius,
                    report.clearance.exact_work - 1
                )
                .is_err()
            );
        }
    }
}

#[test]
fn swept_material_relation_flags_zero_radius_boundary_contact() {
    use brep_core::analysis::{
        PolyhedralSweptPathRelation::*, exact_polyhedral_swept_path_material,
    };
    use num_rational::BigRational as Q;
    let solid = shifted_oblique_solid(0.);
    let q = |n: i64| Q::from_integer(n.into());
    let path = [[q(1), q(0), q(4)], [q(1), q(0), q(-1)]];
    for (r, expected) in [(0, ZeroRadiusBoundaryContact), (1, CrossesBoundary)] {
        let report = exact_polyhedral_swept_path_material(&solid, &path, &q(r), 1_000_000).unwrap();
        assert_eq!(report.relation, expected);
        assert_eq!(report.clearance.nearest.distance_squared_mm2, q(0));
    }
}

#[test]
fn first_contact_encloses_an_irrational_parameter_on_the_first_hit_span() {
    use brep_core::{
        analysis::bounded_polyhedral_swept_path_first_contact,
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let path = [
        [q(-2, 1), q(3, 1), q(5, 2)],
        [q(-1, 1), q(3, 1), q(5, 2)],
        [q(1, 1), q(3, 1), q(5, 2)],
    ];
    let report =
        bounded_polyhedral_swept_path_first_contact(solid, &path, &q(1, 1), 1e-7, 1_000_000)
            .unwrap();
    let contact = report.contact.unwrap();
    assert_eq!(contact.span, 1);
    let [lo, hi] = contact.parameter_bounds;
    let squared_gap = |t: &Q| (q(1, 1) - q(2, 1) * t).pow(2) + q(1, 4);
    assert!(squared_gap(&lo) > q(1, 1) && squared_gap(&hi) <= q(1, 1));
    assert!(hi - lo <= Q::from_float(1e-7).unwrap());
    assert!(
        bounded_polyhedral_swept_path_first_contact(
            solid,
            &path,
            &q(1, 1),
            1e-7,
            report.exact_work
        )
        .is_ok()
    );
    assert!(
        bounded_polyhedral_swept_path_first_contact(
            solid,
            &path,
            &q(1, 1),
            1e-7,
            report.exact_work - 1
        )
        .is_err()
    );
}

#[test]
fn first_contact_includes_tangency_and_initial_contact() {
    use brep_core::{
        analysis::bounded_polyhedral_swept_path_first_contact,
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x| [q(x, 1), q(3, 1), q(3, 1)];
    let path = [p(-1), p(5)];
    let report =
        bounded_polyhedral_swept_path_first_contact(solid, &path, &q(1, 1), 1e-6, 1_000_000)
            .unwrap();
    let [lo, hi] = report.contact.unwrap().parameter_bounds;
    assert!(lo <= q(1, 6) && q(1, 6) <= hi);
    assert!(
        bounded_polyhedral_swept_path_first_contact(solid, &path, &q(1, 2), 1e-6, 1_000_000)
            .unwrap()
            .contact
            .is_none()
    );
    let isolated = [[q(-2, 1), q(0, 1), q(15, 4)], [q(0, 1), q(-2, 1), q(15, 4)]];
    let hit =
        bounded_polyhedral_swept_path_first_contact(solid, &isolated, &q(9, 4), 1e-6, 1_000_000)
            .unwrap()
            .contact
            .unwrap();
    assert_eq!(hit.parameter_bounds[1], q(1, 2));
    assert!(hit.parameter_bounds[0] < q(1, 2));
    let initial = bounded_polyhedral_swept_path_first_contact(
        solid,
        &[p(1), p(5)],
        &q(1, 1),
        1e-6,
        1_000_000,
    )
    .unwrap();
    assert_eq!(
        initial.contact.unwrap().parameter_bounds,
        [q(0, 1), q(0, 1)]
    );
}

#[test]
fn swept_face_ranges_enclose_extremes_of_disconnected_cap_contacts() {
    use brep_core::{
        analysis::bounded_polyhedral_swept_face_contact_ranges,
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let point = |x| [q(x, 1), q(3, 2), q(2, 1)];
    let path = [point(-1), point(5)];
    let report =
        bounded_polyhedral_swept_face_contact_ranges(solid, &path, &q(0, 1), 1e-6, 1_000_000)
            .unwrap();
    let cap = report
        .contacts
        .iter()
        .find(|entry| entry.face == result.section.cap_faces[0])
        .unwrap();
    assert_eq!(cap.span, 0);
    for ([lo, hi], expected) in [
        (&cap.first_parameter_bounds, q(1, 6)),
        (&cap.last_parameter_bounds, q(5, 6)),
    ] {
        assert!(*lo <= expected && expected <= *hi);
        assert!(hi - lo <= Q::from_float(1e-6).unwrap());
    }
    assert!(
        bounded_polyhedral_swept_face_contact_ranges(
            solid,
            &path,
            &q(0, 1),
            1e-6,
            report.exact_work
        )
        .is_ok()
    );
    assert!(
        bounded_polyhedral_swept_face_contact_ranges(
            solid,
            &path,
            &q(0, 1),
            1e-6,
            report.exact_work - 1
        )
        .is_err()
    );
}

#[test]
fn swept_face_ranges_retain_full_span_contacts_at_path_endpoints() {
    use brep_core::analysis::bounded_polyhedral_swept_face_contact_ranges;
    use num_rational::BigRational as Q;
    let solid = shifted_oblique_solid(0.);
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = [q(3, 2), q(0, 1), q(5, 2)];
    let path = [p.clone(), p.clone(), p];
    let report =
        bounded_polyhedral_swept_face_contact_ranges(&solid, &path, &q(0, 1), 1e-6, 1_000_000)
            .unwrap();
    assert_eq!(
        report
            .contacts
            .iter()
            .map(|hit| hit.span)
            .collect::<Vec<_>>(),
        [0, 1]
    );
    for hit in report.contacts {
        assert_eq!(hit.first_parameter_bounds, [q(0, 1), q(0, 1)]);
        assert_eq!(hit.last_parameter_bounds, [q(1, 1), q(1, 1)]);
    }
}

#[test]
fn exact_zero_radius_face_contacts_preserve_the_gap_over_a_hole() {
    use brep_core::{
        analysis::exact_polyhedral_path_boundary_contacts,
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let path = [[q(-1, 1), q(3, 2), q(2, 1)], [q(5, 1), q(3, 2), q(2, 1)]];
    let report = exact_polyhedral_path_boundary_contacts(solid, &path, 1_000_000).unwrap();
    let cap = report
        .contacts
        .iter()
        .find(|entry| entry.face == result.section.cap_faces[0])
        .unwrap();
    assert_eq!(cap.intervals, [[q(1, 6), q(1, 3)], [q(1, 2), q(5, 6)]]);
    assert!(exact_polyhedral_path_boundary_contacts(solid, &path, report.exact_work).is_ok());
    assert!(exact_polyhedral_path_boundary_contacts(solid, &path, report.exact_work - 1).is_err());
}

#[test]
fn exact_zero_radius_face_contacts_include_transverse_points_and_stationary_spans() {
    use brep_core::analysis::exact_polyhedral_path_boundary_contacts;
    use num_rational::BigRational as Q;
    let solid = shifted_oblique_solid(0.);
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let point = |z| [q(3, 2), q(0, 1), z];
    let path = [
        point(q(7, 2)),
        point(q(5, 2)),
        point(q(5, 2)),
        point(q(-1, 2)),
    ];
    let report = exact_polyhedral_path_boundary_contacts(&solid, &path, 1_000_000).unwrap();
    assert_eq!(
        report
            .contacts
            .iter()
            .map(|hit| (hit.span, hit.intervals.clone()))
            .collect::<Vec<_>>(),
        [
            (0, vec![[q(1, 1), q(1, 1)]]),
            (1, vec![[q(0, 1), q(1, 1)]]),
            (2, vec![[q(2, 3), q(2, 3)]]),
            (2, vec![[q(0, 1), q(0, 1)]]),
        ]
    );
}

#[test]
fn exact_path_material_clip_keeps_voids_interior_and_coplanar() {
    use brep_core::{
        analysis::exact_polyhedral_path_material_clip,
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let point = |x, z| [q(x, 1), q(3, 2), q(z, 1)];
    let path = [point(-1, 1), point(5, 1), point(5, 2), point(-1, 2)];
    let report = exact_polyhedral_path_material_clip(solid, &path, 1_000_000).unwrap();
    assert_eq!(
        report.span_intervals,
        [
            vec![[q(1, 6), q(1, 3)], [q(1, 2), q(5, 6)]],
            vec![],
            vec![[q(1, 6), q(1, 2)], [q(2, 3), q(5, 6)]],
        ]
    );
    assert_eq!(
        exact_polyhedral_path_material_clip(solid, &path, report.exact_work).unwrap(),
        report
    );
    assert!(exact_polyhedral_path_material_clip(solid, &path, report.exact_work - 1).is_err());
}

#[test]
fn exact_path_material_clip_preserves_isolated_and_stationary_contacts() {
    use brep_core::{
        analysis::exact_polyhedral_path_material_clip,
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let point = |x, y| [q(x, 1), q(y, 1), q(1, 1)];
    let inside = [q(1, 2), q(1, 2), q(1, 1)];
    let boundary = point(0, 0);
    let full = vec![[q(0, 1), q(1, 1)]];
    for (path, expected) in [
        ([point(-1, 1), point(1, -1)], vec![[q(1, 2), q(1, 2)]]),
        ([point(-1, -1), boundary.clone()], vec![[q(1, 1), q(1, 1)]]),
        ([inside.clone(), inside], full.clone()),
        ([boundary.clone(), boundary], full),
        ([point(-1, -1), point(-1, -1)], vec![]),
    ] {
        let report = exact_polyhedral_path_material_clip(solid, &path, 1_000_000).unwrap();
        assert_eq!(report.span_intervals, [expected]);
    }
}

#[test]
fn bounded_path_material_length_counts_boundary_travel_and_retracing() {
    use brep_core::{
        analysis::bounded_polyhedral_path_material_length,
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let point = |x| [q(x, 1), q(3, 2), q(2, 1)];
    let path = [point(-1), point(5), point(5), point(-1)];
    let report = bounded_polyhedral_path_material_length(solid, &path, 1e-6, 1_000_000).unwrap();
    assert_eq!(report.length_bounds_mm, [q(6, 1), q(6, 1)]);
    assert_eq!(
        bounded_polyhedral_path_material_length(solid, &path, 1e-6, report.exact_work).unwrap(),
        report
    );
    assert!(
        bounded_polyhedral_path_material_length(solid, &path, 1e-6, report.exact_work - 1).is_err()
    );
}

#[test]
fn bounded_path_material_length_encloses_irrational_distance() {
    use brep_core::{
        analysis::bounded_polyhedral_path_material_length,
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64| Q::from_integer(n.into());
    let point = |x| [q(x), q(x), q(1)];
    let path = [point(-1), point(5)];
    let tolerance = 1e-9;
    let [lo, hi] = bounded_polyhedral_path_material_length(solid, &path, tolerance, 1_000_000)
        .unwrap()
        .length_bounds_mm;
    assert!(lo >= q(0) && lo.pow(2) < q(18) && hi.pow(2) > q(18));
    assert!(hi - lo <= Q::from_float(tolerance).unwrap());
    assert!(bounded_polyhedral_path_material_length(solid, &path, f64::NAN, 1_000_000).is_err());
    let tangent = [[q(-1), q(1), q(1)], [q(1), q(-1), q(1)]];
    assert_eq!(
        bounded_polyhedral_path_material_length(solid, &tangent, tolerance, 1_000_000)
            .unwrap()
            .length_bounds_mm,
        [q(0), q(0)]
    );
}

#[test]
fn exact_path_exterior_clip_preserves_open_ends_at_hole_and_face_contacts() {
    use brep_core::{
        analysis::{PolyhedralExteriorInterval, exact_polyhedral_path_exterior_clip},
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let point = |x, z| [q(x, 1), q(3, 2), q(z, 1)];
    let expected = vec![
        PolyhedralExteriorInterval {
            parameters: [q(0, 1), q(1, 6)],
            endpoints_included: [true, false],
        },
        PolyhedralExteriorInterval {
            parameters: [q(1, 3), q(1, 2)],
            endpoints_included: [false, false],
        },
        PolyhedralExteriorInterval {
            parameters: [q(5, 6), q(1, 1)],
            endpoints_included: [false, true],
        },
    ];
    for z in [1, 2] {
        let path = [point(-1, z), point(5, z)];
        let report = exact_polyhedral_path_exterior_clip(solid, &path, 1_000_000).unwrap();
        assert_eq!(report.span_intervals, [expected.clone()]);
        assert_eq!(
            exact_polyhedral_path_exterior_clip(solid, &path, report.exact_work).unwrap(),
            report
        );
        assert!(exact_polyhedral_path_exterior_clip(solid, &path, report.exact_work - 1).is_err());
    }
}

#[test]
fn exact_path_exterior_clip_splits_at_isolated_contact_and_handles_stationary_spans() {
    use brep_core::{
        analysis::{PolyhedralExteriorInterval, exact_polyhedral_path_exterior_clip},
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let point = |x, y| [q(x, 1), q(y, 1), q(1, 1)];
    let interval = |lo, hi, endpoints_included| PolyhedralExteriorInterval {
        parameters: [lo, hi],
        endpoints_included,
    };
    let stationary = vec![interval(q(0, 1), q(1, 1), [true, true])];
    let path = [
        point(-1, 1),
        point(-1, 1),
        point(1, -1),
        point(0, 0),
        point(0, 0),
    ];
    let report = exact_polyhedral_path_exterior_clip(solid, &path, 1_000_000).unwrap();
    assert_eq!(
        report.span_intervals,
        [
            stationary,
            vec![
                interval(q(0, 1), q(1, 2), [true, false]),
                interval(q(1, 2), q(1, 1), [false, true]),
            ],
            vec![interval(q(0, 1), q(1, 1), [true, false])],
            vec![],
        ]
    );
}

#[test]
fn exact_path_interior_clip_excludes_coplanar_runs_and_keeps_through_hole_gap() {
    use brep_core::{
        analysis::{PolyhedralInteriorInterval, exact_polyhedral_path_interior_clip},
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let point = |x, z| [q(x, 1), q(3, 2), q(z, 1)];
    let path = [point(-1, 1), point(5, 1), point(5, 2), point(-1, 2)];
    let report = exact_polyhedral_path_interior_clip(solid, &path, 1_000_000).unwrap();
    assert_eq!(
        report.span_intervals,
        [
            vec![
                PolyhedralInteriorInterval {
                    parameters: [q(1, 6), q(1, 3)],
                    endpoints_included: [false, false],
                },
                PolyhedralInteriorInterval {
                    parameters: [q(1, 2), q(5, 6)],
                    endpoints_included: [false, false],
                },
            ],
            vec![],
            vec![],
        ]
    );
    assert_eq!(
        exact_polyhedral_path_interior_clip(solid, &path, report.exact_work).unwrap(),
        report
    );
    assert!(exact_polyhedral_path_interior_clip(solid, &path, report.exact_work - 1).is_err());
}

#[test]
fn exact_path_interior_clip_splits_at_inner_wall_tangency_and_keeps_interior_ends() {
    use brep_core::{
        analysis::{PolyhedralInteriorInterval, exact_polyhedral_path_interior_clip},
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let a = [q(1, 2), q(3, 2), q(1, 1)];
    let b = [q(3, 2), q(1, 2), q(1, 1)];
    let interval = |lo, hi, endpoints_included| PolyhedralInteriorInterval {
        parameters: [lo, hi],
        endpoints_included,
    };
    let path = [a.clone(), b.clone(), b, [q(3, 2), q(1, 2), q(2, 1)]];
    let report = exact_polyhedral_path_interior_clip(solid, &path, 1_000_000).unwrap();
    assert_eq!(
        report.span_intervals,
        [
            vec![
                interval(q(0, 1), q(1, 2), [true, false]),
                interval(q(1, 2), q(1, 1), [false, true]),
            ],
            vec![interval(q(0, 1), q(1, 1), [true, true])],
            vec![interval(q(0, 1), q(1, 1), [true, false])],
        ]
    );
}

#[test]
fn exact_path_partition_orders_transverse_contacts_and_classifies_each_gap() {
    use brep_core::{
        analysis::{
            PolyhedralPathLocation::*, PolyhedralPathPart, exact_polyhedral_path_partition,
        },
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let point = |x| [q(x, 1), q(3, 2), q(1, 1)];
    let path = [point(-1), point(5)];
    let report = exact_polyhedral_path_partition(solid, &path, 1_000_000).unwrap();
    let part = |location, lo, hi, endpoints_included| PolyhedralPathPart {
        location,
        parameters: [lo, hi],
        endpoints_included,
    };
    let contact = |t: Q| part(Boundary, t.clone(), t, [true, true]);
    assert_eq!(
        report.spans,
        [vec![
            part(Exterior, q(0, 1), q(1, 6), [true, false]),
            contact(q(1, 6)),
            part(Interior, q(1, 6), q(1, 3), [false, false]),
            contact(q(1, 3)),
            part(Exterior, q(1, 3), q(1, 2), [false, false]),
            contact(q(1, 2)),
            part(Interior, q(1, 2), q(5, 6), [false, false]),
            contact(q(5, 6)),
            part(Exterior, q(5, 6), q(1, 1), [false, true]),
        ]]
    );
    assert_eq!(
        exact_polyhedral_path_partition(solid, &path, report.exact_work).unwrap(),
        report
    );
    assert!(exact_polyhedral_path_partition(solid, &path, report.exact_work - 1).is_err());
}

#[test]
fn exact_path_partition_coalesces_shared_edge_contacts_and_stationary_boundary() {
    use brep_core::{
        analysis::{
            PolyhedralPathLocation::*, PolyhedralPathPart, exact_polyhedral_path_partition,
        },
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let point = |x| [q(x, 1), q(0, 1), q(2, 1)];
    let path = [point(-1), point(2), point(2)];
    let report = exact_polyhedral_path_partition(solid, &path, 1_000_000).unwrap();
    let part = |location, lo, hi, endpoints_included| PolyhedralPathPart {
        location,
        parameters: [lo, hi],
        endpoints_included,
    };
    assert_eq!(
        report.spans,
        [
            vec![
                part(Exterior, q(0, 1), q(1, 3), [true, false]),
                part(Boundary, q(1, 3), q(1, 1), [true, true]),
            ],
            vec![part(Boundary, q(0, 1), q(1, 1), [true, true])],
        ]
    );
}

#[test]
fn exact_path_boundary_events_distinguish_entries_exits_and_tangencies() {
    use brep_core::{
        analysis::{
            PolyhedralPathBoundaryEvent, PolyhedralPathLocation::*,
            exact_polyhedral_path_boundary_events,
        },
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let path = [[q(-1, 1), q(3, 2), q(1, 1)], [q(5, 1), q(3, 2), q(1, 1)]];
    let event = |t: Q, before, after| PolyhedralPathBoundaryEvent {
        parameters: [t.clone(), t],
        before: Some(before),
        after: Some(after),
    };
    let report = exact_polyhedral_path_boundary_events(solid, &path, 1_000_000).unwrap();
    assert_eq!(
        report.events,
        [
            event(q(1, 6), Exterior, Interior),
            event(q(1, 3), Interior, Exterior),
            event(q(1, 2), Exterior, Interior),
            event(q(5, 6), Interior, Exterior),
        ]
    );
    assert_eq!(
        exact_polyhedral_path_boundary_events(solid, &path, report.exact_work).unwrap(),
        report
    );
    assert!(exact_polyhedral_path_boundary_events(solid, &path, report.exact_work - 1).is_err());
    let tangent = [[q(-1, 1), q(1, 1), q(1, 1)], [q(1, 1), q(-1, 1), q(1, 1)]];
    assert_eq!(
        exact_polyhedral_path_boundary_events(solid, &tangent, 1_000_000)
            .unwrap()
            .events,
        [event(q(1, 2), Exterior, Exterior)],
    );
}

#[test]
fn exact_path_boundary_events_join_across_vertices_and_preserve_endpoints() {
    use brep_core::{
        analysis::{
            PolyhedralPathBoundaryEvent, PolyhedralPathLocation::*,
            exact_polyhedral_path_boundary_events,
        },
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64| Q::from_integer(n.into());
    let point = |x, z| [q(x), q(1), q(z)];
    let a = point(0, 2);
    let b = [Q::new(1.into(), 2.into()), q(1), q(2)];
    let c = [b[0].clone(), q(1), q(1)];
    let path = [point(-1, 2), a, b.clone(), b.clone(), c, b];
    let report = exact_polyhedral_path_boundary_events(solid, &path, 1_000_000).unwrap();
    assert_eq!(
        report.events,
        [
            PolyhedralPathBoundaryEvent {
                parameters: [q(1), q(3)],
                before: Some(Exterior),
                after: Some(Interior),
            },
            PolyhedralPathBoundaryEvent {
                parameters: [q(5), q(5)],
                before: Some(Interior),
                after: None,
            },
        ]
    );
    let reversed = path.into_iter().rev().collect::<Vec<_>>();
    let reverse = exact_polyhedral_path_boundary_events(solid, &reversed, 1_000_000).unwrap();
    assert_eq!(reverse.events[0].parameters, [q(0), q(0)]);
    assert_eq!(reverse.events[0].before, None);
    assert_eq!(reverse.events[0].after, Some(Interior));
    assert_eq!(reverse.events[1].parameters, [q(2), q(4)]);
    assert_eq!(reverse.events[1].before, Some(Interior));
    assert_eq!(reverse.events[1].after, Some(Exterior));
}

#[test]
fn bounded_path_length_breakdown_counts_each_location_and_stationary_boundary() {
    use brep_core::{
        analysis::bounded_polyhedral_path_length_breakdown,
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64| Q::from_integer(n.into());
    let point = |x, z| [q(x), Q::new(3.into(), 2.into()), q(z)];
    let path = [
        point(-1, 1),
        point(5, 1),
        point(5, 2),
        point(4, 2),
        point(4, 2),
        point(-1, 2),
    ];
    let report = bounded_polyhedral_path_length_breakdown(solid, &path, 1e-6, 1_000_000).unwrap();
    assert_eq!(report.interior_bounds_mm, [q(3), q(3)]);
    assert_eq!(report.boundary_bounds_mm, [q(3), q(3)]);
    assert_eq!(report.exterior_bounds_mm, [q(7), q(7)]);
    assert_eq!(
        bounded_polyhedral_path_length_breakdown(solid, &path, 1e-6, report.exact_work).unwrap(),
        report
    );
    assert!(
        bounded_polyhedral_path_length_breakdown(solid, &path, 1e-6, report.exact_work - 1)
            .is_err()
    );
}

#[test]
fn bounded_path_length_breakdown_certifies_irrational_lengths() {
    use brep_core::{
        analysis::bounded_polyhedral_path_length_breakdown,
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64| Q::from_integer(n.into());
    let point = |x, z| [q(x), q(x), q(z)];
    let path = [point(-1, 1), point(5, 1), point(5, 2), point(-1, 2)];
    let tol = 1e-9;
    let report = bounded_polyhedral_path_length_breakdown(solid, &path, tol, 1_000_000).unwrap();
    for ([lo, hi], offset, radicand) in [
        (report.interior_bounds_mm, q(0), q(18)),
        (report.boundary_bounds_mm, q(0), q(18)),
        (report.exterior_bounds_mm, q(1), q(72)),
    ] {
        assert!(lo >= offset && (&lo - &offset).pow(2) < radicand);
        assert!((&hi - &offset).pow(2) > radicand);
        assert!(hi - lo <= Q::from_float(tol).unwrap());
    }
    assert!(bounded_polyhedral_path_length_breakdown(solid, &path, 0.0, 1_000_000).is_err());
}

#[test]
fn exact_material_path_components_join_spans_and_preserve_hole_and_turn() {
    use brep_core::{
        analysis::{PolyhedralMaterialPathComponent, exact_polyhedral_material_path_components},
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let point = |x, z| [q(x, 1), q(3, 2), q(z, 1)];
    let path = [
        point(-1, 1),
        point(3, 1),
        point(3, 1),
        point(3, 2),
        point(5, 2),
    ];
    let report = exact_polyhedral_material_path_components(solid, &path, 1_000_000).unwrap();
    assert_eq!(
        report.components,
        [
            PolyhedralMaterialPathComponent {
                parameters: [q(1, 4), q(1, 2)],
                points_mm: vec![point(0, 1), point(1, 1)],
            },
            PolyhedralMaterialPathComponent {
                parameters: [q(3, 4), q(7, 2)],
                points_mm: vec![point(2, 1), point(3, 1), point(3, 2), point(4, 2)],
            },
        ]
    );
    assert_eq!(
        exact_polyhedral_material_path_components(solid, &path, report.exact_work).unwrap(),
        report
    );
    assert!(
        exact_polyhedral_material_path_components(solid, &path, report.exact_work - 1).is_err()
    );
}

#[test]
fn exact_material_path_components_keep_separate_visits_to_the_same_tangent_point() {
    use brep_core::{
        analysis::{PolyhedralMaterialPathComponent, exact_polyhedral_material_path_components},
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let point = |x, y| [q(x, 1), q(y, 1), q(1, 1)];
    let path = [point(-1, 1), point(1, -1), point(-1, 1)];
    let report = exact_polyhedral_material_path_components(solid, &path, 1_000_000).unwrap();
    assert_eq!(
        report.components,
        [q(1, 2), q(3, 2)].map(|t| PolyhedralMaterialPathComponent {
            parameters: [t.clone(), t],
            points_mm: vec![point(0, 0)],
        }),
    );
}

#[test]
fn exact_path_common_material_clip_retains_only_the_shared_part_of_three_solids() {
    use brep_core::analysis::exact_polyhedral_path_common_material_clip;
    use num_rational::BigRational as Q;
    let a = shifted_oblique_solid(0.);
    let b = shifted_oblique_solid(1.);
    let c = shifted_oblique_solid(2.);
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let point = |x| [q(x, 1), q(0, 1), q(1, 1)];
    let path = [point(-1), point(5), point(-1)];
    let inputs = [&a, &b, &c];
    let report = exact_polyhedral_path_common_material_clip(&inputs, &path, 1_000_000).unwrap();
    assert_eq!(
        report.span_intervals,
        [vec![[q(1, 2), q(2, 3)]], vec![[q(1, 3), q(1, 2)]]],
    );
    assert_eq!(
        exact_polyhedral_path_common_material_clip(&[&c, &b, &a], &path, 1_000_000)
            .unwrap()
            .span_intervals,
        report.span_intervals
    );
    assert_eq!(
        exact_polyhedral_path_common_material_clip(&inputs, &path, report.exact_work).unwrap(),
        report
    );
    assert!(
        exact_polyhedral_path_common_material_clip(&inputs, &path, report.exact_work - 1).is_err()
    );
}

#[test]
fn exact_path_common_material_clip_distinguishes_touching_and_disjoint_solids() {
    use brep_core::analysis::exact_polyhedral_path_common_material_clip;
    use num_rational::BigRational as Q;
    let a = shifted_oblique_solid(0.);
    let touching = shifted_oblique_solid(3.);
    let disjoint = shifted_oblique_solid(4.);
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let path = [[q(0, 1), q(0, 1), q(1, 1)], [q(6, 1), q(0, 1), q(1, 1)]];
    assert_eq!(
        exact_polyhedral_path_common_material_clip(&[&a, &touching], &path, 1_000_000)
            .unwrap()
            .span_intervals,
        [vec![[q(1, 2), q(1, 2)]]],
    );
    assert!(
        exact_polyhedral_path_common_material_clip(&[&a, &disjoint], &path, 1_000_000)
            .unwrap()
            .span_intervals[0]
            .is_empty()
    );
    assert!(exact_polyhedral_path_common_material_clip(&[], &path, 1_000_000).is_err());
}

#[test]
fn exact_path_union_material_clip_merges_overlap_and_contact_but_keeps_free_gaps() {
    use brep_core::analysis::exact_polyhedral_path_union_material_clip;
    use num_rational::BigRational as Q;
    let a = shifted_oblique_solid(0.);
    let overlap = shifted_oblique_solid(2.);
    let touching = shifted_oblique_solid(5.);
    let separated = shifted_oblique_solid(9.);
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let point = |x| [q(x, 1), q(0, 1), q(1, 1)];
    let path = [point(-1), point(13), point(-1)];
    let solids = [&touching, &separated, &a, &overlap];
    let report = exact_polyhedral_path_union_material_clip(&solids, &path, 1_000_000).unwrap();
    assert_eq!(
        report.span_intervals,
        [
            vec![[q(1, 14), q(9, 14)], [q(5, 7), q(13, 14)]],
            vec![[q(1, 14), q(2, 7)], [q(5, 14), q(13, 14)]],
        ]
    );
    assert_eq!(
        exact_polyhedral_path_union_material_clip(&solids, &path, report.exact_work).unwrap(),
        report
    );
    assert!(
        exact_polyhedral_path_union_material_clip(&solids, &path, report.exact_work - 1).is_err()
    );
}

#[test]
fn exact_path_union_material_clip_keeps_isolated_contacts() {
    use brep_core::{
        analysis::exact_polyhedral_path_union_material_clip,
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let solid = &result.section.solid;
    let distant = shifted_oblique_solid(4.);
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let path = [[q(-1, 1), q(1, 1), q(1, 1)], [q(1, 1), q(-1, 1), q(1, 1)]];
    let report =
        exact_polyhedral_path_union_material_clip(&[&distant, solid, solid], &path, 1_000_000)
            .unwrap();
    assert_eq!(report.span_intervals, [vec![[q(1, 2), q(1, 2)]]]);
    assert!(exact_polyhedral_path_union_material_clip(&[], &path, 1_000_000).is_err());
}

#[test]
fn exact_path_material_difference_removes_touching_and_overlapping_cutters() {
    use brep_core::analysis::{
        PolyhedralPathDifferenceInterval, exact_polyhedral_path_material_difference,
    };
    use num_rational::BigRational as Q;
    let stock = shifted_oblique_solid(0.);
    let left = shifted_oblique_solid(-3.);
    let right = shifted_oblique_solid(2.);
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let point = |x| [q(x, 1), q(0, 1), q(1, 1)];
    let path = [point(-1), point(5), point(-1)];
    let cutters = [&left, &right];
    let report =
        exact_polyhedral_path_material_difference(&stock, &cutters, &path, 1_000_000).unwrap();
    assert_eq!(
        report.span_intervals,
        [
            vec![PolyhedralPathDifferenceInterval {
                parameters: [q(1, 6), q(1, 2)],
                endpoints_included: [false, false],
            }],
            vec![PolyhedralPathDifferenceInterval {
                parameters: [q(1, 2), q(5, 6)],
                endpoints_included: [false, false],
            }],
        ],
    );
    assert_eq!(
        exact_polyhedral_path_material_difference(&stock, &cutters, &path, report.exact_work)
            .unwrap(),
        report
    );
    assert!(
        exact_polyhedral_path_material_difference(&stock, &cutters, &path, report.exact_work - 1)
            .is_err()
    );
}

#[test]
fn exact_path_material_difference_removes_a_single_point_inside_stock() {
    use brep_core::{
        analysis::{PolyhedralPathDifferenceInterval, exact_polyhedral_path_material_difference},
        planar_cap::build_outward_inferred_section_solid,
    };
    use num_rational::BigRational as Q;
    let (source, _) = open_hollow_prism();
    let result = build_outward_inferred_section_solid(&source, section_solid_options()).unwrap();
    let cutter = &result.section.solid;
    let stock = shifted_oblique_solid(-1.);
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let point = |x, y| [q(x, 2), q(y, 2), q(1, 1)];
    let path = [point(-1, 1), point(1, -1)];
    let interval = |lo, hi, endpoints_included| PolyhedralPathDifferenceInterval {
        parameters: [lo, hi],
        endpoints_included,
    };
    assert_eq!(
        exact_polyhedral_path_material_difference(&stock, &[cutter], &path, 1_000_000)
            .unwrap()
            .span_intervals,
        [vec![
            interval(q(0, 1), q(1, 2), [true, false]),
            interval(q(1, 2), q(1, 1), [false, true]),
        ]]
    );
    assert_eq!(
        exact_polyhedral_path_material_difference(&stock, &[], &path, 1_000_000)
            .unwrap()
            .span_intervals,
        [vec![interval(q(0, 1), q(1, 1), [true, true])]],
    );
}

#[test]
fn exact_path_coverage_preserves_point_membership_at_shared_boundaries() {
    use brep_core::analysis::{PolyhedralPathCoveragePart, exact_polyhedral_path_coverage};
    use num_rational::BigRational as Q;
    let a = shifted_oblique_solid(0.);
    let b = shifted_oblique_solid(3.);
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let point = |x| [q(x, 1), q(0, 1), q(1, 1)];
    let path = [point(-1), point(7)];
    let solids = [&b, &a];
    let report = exact_polyhedral_path_coverage(&solids, &path, 1_000_000).unwrap();
    let part = |lo, hi, endpoints_included, solids: &[usize]| PolyhedralPathCoveragePart {
        parameters: [lo, hi],
        endpoints_included,
        solids: solids.to_vec(),
    };
    assert_eq!(
        report.spans,
        [vec![
            part(q(0, 1), q(1, 8), [true, false], &[]),
            part(q(1, 8), q(1, 2), [true, false], &[1]),
            part(q(1, 2), q(1, 2), [true, true], &[0, 1]),
            part(q(1, 2), q(7, 8), [false, true], &[0]),
            part(q(7, 8), q(1, 1), [false, true], &[]),
        ]],
    );
    assert_eq!(
        exact_polyhedral_path_coverage(&solids, &path, report.exact_work).unwrap(),
        report
    );
    assert!(exact_polyhedral_path_coverage(&solids, &path, report.exact_work - 1).is_err());
}

#[test]
fn exact_path_coverage_handles_overlapping_spans_and_duplicate_solids() {
    use brep_core::analysis::{PolyhedralPathCoveragePart, exact_polyhedral_path_coverage};
    use num_rational::BigRational as Q;
    let a = shifted_oblique_solid(0.);
    let b = shifted_oblique_solid(1.);
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let point = |x| [q(x, 1), q(0, 1), q(1, 1)];
    let path = [point(0), point(2), point(2)];
    let report = exact_polyhedral_path_coverage(&[&a, &b, &a], &path, 1_000_000).unwrap();
    let part = |lo, hi, endpoints_included, solids: &[usize]| PolyhedralPathCoveragePart {
        parameters: [lo, hi],
        endpoints_included,
        solids: solids.to_vec(),
    };
    assert_eq!(
        report.spans,
        [
            vec![
                part(q(0, 1), q(1, 2), [true, false], &[0, 2]),
                part(q(1, 2), q(1, 1), [true, true], &[0, 1, 2]),
            ],
            vec![part(q(0, 1), q(1, 1), [true, true], &[0, 1, 2])],
        ]
    );
}

#[test]
fn bounded_path_coverage_lengths_omit_point_contacts_and_stationary_distance() {
    use brep_core::analysis::{
        PolyhedralPathCoverageLength, bounded_polyhedral_path_coverage_lengths,
    };
    use num_rational::BigRational as Q;
    let a = shifted_oblique_solid(0.);
    let b = shifted_oblique_solid(3.);
    let q = |n: i64| Q::from_integer(n.into());
    let point = |x| [q(x), q(0), q(1)];
    let path = [point(-1), point(3), point(3), point(7), point(-1)];
    let solids = [&a, &b];
    let report = bounded_polyhedral_path_coverage_lengths(&solids, &path, 1e-6, 1_000_000).unwrap();
    assert_eq!(
        report.lengths,
        [(0, 4), (1, 12)].map(|(solid_count, length)| PolyhedralPathCoverageLength {
            solid_count,
            length_bounds_mm: [q(length), q(length)],
        }),
    );
    assert_eq!(
        bounded_polyhedral_path_coverage_lengths(&solids, &path, 1e-6, report.exact_work).unwrap(),
        report
    );
    assert!(
        bounded_polyhedral_path_coverage_lengths(&solids, &path, 1e-6, report.exact_work - 1)
            .is_err()
    );
}

#[test]
fn bounded_path_coverage_lengths_enclose_irrational_distance_at_every_count() {
    use brep_core::analysis::bounded_polyhedral_path_coverage_lengths;
    use num_rational::BigRational as Q;
    let a = shifted_oblique_solid(0.);
    let b = shifted_oblique_solid(2.);
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let path = [[q(-1, 1), q(0, 1), q(1, 1)], [q(7, 1), q(0, 1), q(3, 2)]];
    let tol = 1e-9;
    let report =
        bounded_polyhedral_path_coverage_lengths(&[&a, &b], &path, tol, 1_000_000).unwrap();
    assert_eq!(report.lengths.len(), 3);
    for (count, (entry, width)) in report.lengths.into_iter().zip([3, 4, 1]).enumerate() {
        assert_eq!(entry.solid_count, count);
        let expected_squared = q(257 * width * width, 256);
        let [lo, hi] = entry.length_bounds_mm;
        assert!(lo >= q(0, 1) && lo.pow(2) < expected_squared && hi.pow(2) > expected_squared);
        assert!(hi - lo <= Q::from_float(tol).unwrap());
    }
}

#[test]
fn exact_path_coverage_components_keep_shared_point_and_open_ends() {
    use brep_core::analysis::{
        PolyhedralPathCoverageComponent, exact_polyhedral_path_coverage_components,
    };
    use num_rational::BigRational as Q;
    let a = shifted_oblique_solid(0.);
    let b = shifted_oblique_solid(3.);
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let point = |x| [q(x, 1), q(0, 1), q(1, 1)];
    let path = [point(-1), point(7)];
    let solids = [&a, &b];
    let part = |lo, hi, endpoints_included, points_mm| PolyhedralPathCoverageComponent {
        parameters: [lo, hi],
        endpoints_included,
        points_mm,
    };
    let get = |count| {
        exact_polyhedral_path_coverage_components(&solids, &path, count, 1_000_000).unwrap()
    };
    assert_eq!(
        get(1..=1).components,
        [
            part(q(1, 8), q(1, 2), [true, false], vec![point(0), point(3)]),
            part(q(1, 2), q(7, 8), [false, true], vec![point(3), point(6)]),
        ]
    );
    assert_eq!(
        get(2..=2).components,
        [part(q(1, 2), q(1, 2), [true, true], vec![point(3)])]
    );
    let union = get(1..=2);
    assert_eq!(
        union.components,
        [part(
            q(1, 8),
            q(7, 8),
            [true, true],
            vec![point(0), point(3), point(6)]
        )]
    );
    assert_eq!(
        exact_polyhedral_path_coverage_components(&solids, &path, 1..=2, union.exact_work).unwrap(),
        union
    );
    assert!(
        exact_polyhedral_path_coverage_components(&solids, &path, 1..=2, union.exact_work - 1)
            .is_err()
    );
}

#[test]
fn exact_path_coverage_components_join_at_span_vertex_without_losing_it() {
    use brep_core::analysis::{
        PolyhedralPathCoverageComponent, exact_polyhedral_path_coverage_components,
    };
    use num_rational::BigRational as Q;
    let a = shifted_oblique_solid(0.);
    let b = shifted_oblique_solid(2.);
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let start = [q(0, 1), q(0, 1), q(1, 1)];
    let vertex = [q(5, 2), q(0, 1), q(1, 1)];
    let end = [q(5, 2), q(0, 1), q(2, 1)];
    let path = [start, vertex.clone(), end.clone()];
    let report =
        exact_polyhedral_path_coverage_components(&[&a, &b], &path, 2..=2, 1_000_000).unwrap();
    assert_eq!(
        report.components,
        [PolyhedralPathCoverageComponent {
            parameters: [q(4, 5), q(2, 1)],
            endpoints_included: [true, true],
            points_mm: vec![[q(2, 1), q(0, 1), q(1, 1)], vertex, end],
        }]
    );
}

#[test]
fn exact_spatial_polyline_simplification_preserves_closed_anchor_and_retracing() {
    use brep_core::analysis::simplify_exact_spatial_polyline;
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y| [q(x), q(y), q(x)];
    let path = [
        p(0, 0),
        p(0, 0),
        p(1, 0),
        p(2, 0),
        p(1, 0),
        p(1, 1),
        p(0, 0),
    ];
    let report = simplify_exact_spatial_polyline(&path, 1_000_000).unwrap();
    assert_eq!(report.source_vertices, [0, 3, 4, 5, 6]);
    assert_eq!(
        report.points_mm,
        [p(0, 0), p(2, 0), p(1, 0), p(1, 1), p(0, 0)]
    );
    assert_eq!(
        simplify_exact_spatial_polyline(&path, report.exact_work).unwrap(),
        report
    );
    assert!(simplify_exact_spatial_polyline(&path, report.exact_work - 1).is_err());
}

#[test]
fn exact_spatial_polyline_simplification_keeps_small_rational_bends() {
    use brep_core::analysis::simplify_exact_spatial_polyline;
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let a = [q(0, 1), q(0, 1), q(0, 1)];
    let b = [q(1, 3), q(1, 7), q(1, 1_000_000_000)];
    let c = [q(2, 3), q(2, 7), q(2, 1_000_000_000)];
    let d = [q(1, 1), q(3, 7), q(0, 1)];
    let report =
        simplify_exact_spatial_polyline(&[a.clone(), b, c.clone(), d.clone()], 1_000_000).unwrap();
    assert_eq!(report.source_vertices, [0, 2, 3]);
    assert_eq!(report.points_mm, [a.clone(), c, d]);
    let stationary = simplify_exact_spatial_polyline(&[a.clone(), a.clone()], 1_000_000).unwrap();
    assert_eq!(stationary.points_mm, [a]);
    assert_eq!(stationary.source_vertices, [0]);
    assert!(simplify_exact_spatial_polyline(&[], 1_000_000).is_err());
}

#[test]
fn exact_spatial_polyline_projection_preserves_multiple_visits_and_equidistant_points() {
    use brep_core::analysis::{
        SpatialPolylineProjectionHit, project_point_to_exact_spatial_polyline,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x, y| [q(x, 1), q(y, 1), q(0, 1)];
    let path = [p(-1, -2), p(-1, 2), p(1, 2), p(1, -2), p(1, 2)];
    let query = [q(0, 1), q(1, 2), q(0, 1)];
    let hit = |at: Q, x| SpatialPolylineProjectionHit {
        parameters: [at.clone(), at],
        point_mm: [q(x, 1), q(1, 2), q(0, 1)],
    };
    let report = project_point_to_exact_spatial_polyline(&path, &query, 1_000_000).unwrap();
    assert_eq!(report.distance_squared_mm2, q(1, 1));
    assert_eq!(
        report.hits,
        [hit(q(5, 8), -1), hit(q(19, 8), 1), hit(q(29, 8), 1)]
    );
    assert_eq!(
        project_point_to_exact_spatial_polyline(&path, &query, report.exact_work).unwrap(),
        report
    );
    assert!(project_point_to_exact_spatial_polyline(&path, &query, report.exact_work - 1).is_err());
}

#[test]
fn exact_spatial_polyline_projection_coalesces_stationary_and_shared_endpoint_minima() {
    use brep_core::analysis::{
        SpatialPolylineProjectionHit, project_point_to_exact_spatial_polyline,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y, z| [q(x), q(y), q(z)];
    let vertex = p(0, 0, 0);
    let path = [p(-1, 0, 0), vertex.clone(), vertex.clone(), p(0, 1, 0)];
    let query = p(0, 0, 2);
    let report = project_point_to_exact_spatial_polyline(&path, &query, 1_000_000).unwrap();
    assert_eq!(report.distance_squared_mm2, q(4));
    assert_eq!(
        report.hits,
        [SpatialPolylineProjectionHit {
            parameters: [q(1), q(2)],
            point_mm: vertex.clone()
        }]
    );
    let singleton =
        project_point_to_exact_spatial_polyline(&[vertex.clone()], &query, 1_000_000).unwrap();
    assert_eq!(
        singleton.hits,
        [SpatialPolylineProjectionHit {
            parameters: [q(0), q(0)],
            point_mm: vertex
        }]
    );
}

#[test]
fn exact_spatial_polyline_intersections_reject_skew_pairs_and_keep_crossings() {
    use brep_core::analysis::{SpatialPolylineIntersection, intersect_exact_spatial_polylines};
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x, y, z| [q(x, 1), q(y, 1), q(z, 1)];
    let first = [p(0, 0, 0), p(3, 3, 3)];
    let second = [p(1, 0, 2), p(1, 2, 2), p(1, 0, 0)];
    let report = intersect_exact_spatial_polylines(&first, &second, 1_000_000).unwrap();
    assert_eq!(
        report.intersections,
        [SpatialPolylineIntersection {
            spans: [0, 1],
            parameters: [[q(1, 3), q(1, 3)], [q(1, 2), q(1, 2)]],
            points_mm: [p(1, 1, 1), p(1, 1, 1)],
        }],
    );
    assert_eq!(
        intersect_exact_spatial_polylines(&first, &second, report.exact_work).unwrap(),
        report
    );
    assert!(intersect_exact_spatial_polylines(&first, &second, report.exact_work - 1).is_err());
}

#[test]
fn exact_spatial_polyline_intersections_preserve_reversed_and_stationary_overlap() {
    use brep_core::analysis::{SpatialPolylineIntersection, intersect_exact_spatial_polylines};
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x| [q(x, 1), q(x, 1), q(x, 1)];
    let first = [p(0), p(4), p(4)];
    let second = [p(5), p(2), p(2)];
    let report = intersect_exact_spatial_polylines(&first, &second, 1_000_000).unwrap();
    assert_eq!(
        report.intersections,
        [
            SpatialPolylineIntersection {
                spans: [0, 0],
                parameters: [[q(1, 2), q(1, 1)], [q(1, 1), q(1, 3)]],
                points_mm: [p(2), p(4)],
            },
            SpatialPolylineIntersection {
                spans: [0, 1],
                parameters: [[q(1, 2), q(1, 2)], [q(0, 1), q(1, 1)]],
                points_mm: [p(2), p(2)],
            },
            SpatialPolylineIntersection {
                spans: [1, 0],
                parameters: [[q(0, 1), q(1, 1)], [q(1, 3), q(1, 3)]],
                points_mm: [p(4), p(4)],
            },
        ],
    );
}

#[test]
fn exact_spatial_polyline_self_intersections_ignore_closed_seam_and_stationary_joins() {
    use brep_core::analysis::self_intersections_exact_spatial_polyline;
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y| [q(x), q(y), q(x + y)];
    let square = [
        p(0, 0),
        p(0, 0),
        p(2, 0),
        p(2, 0),
        p(2, 2),
        p(0, 2),
        p(0, 0),
    ];
    let report = self_intersections_exact_spatial_polyline(&square, 1_000_000).unwrap();
    assert!(report.intersections.is_empty());
    let crossing = [p(0, 0), p(0, 0), p(2, 2), p(0, 2), p(2, 0), p(0, 0)];
    let report = self_intersections_exact_spatial_polyline(&crossing, 1_000_000).unwrap();
    assert_eq!(report.intersections.len(), 1);
    let hit = &report.intersections[0];
    assert_eq!(hit.spans, [1, 3]);
    assert_eq!(hit.points_mm, [p(1, 1), p(1, 1)]);
    assert_eq!(
        self_intersections_exact_spatial_polyline(&crossing, report.exact_work).unwrap(),
        report
    );
    assert!(self_intersections_exact_spatial_polyline(&crossing, report.exact_work - 1).is_err());
}

#[test]
fn exact_spatial_polyline_self_intersections_keep_adjacent_retracing_and_remote_touch() {
    use brep_core::analysis::self_intersections_exact_spatial_polyline;
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x, y| [q(x, 1), q(y, 1), q(0, 1)];
    let retracted = [p(0, 0), p(4, 0), p(4, 0), p(2, 0)];
    let report = self_intersections_exact_spatial_polyline(&retracted, 1_000_000).unwrap();
    assert_eq!(report.intersections.len(), 1);
    assert_eq!(report.intersections[0].spans, [0, 2]);
    assert_eq!(
        report.intersections[0].parameters,
        [[q(1, 2), q(1, 1)], [q(1, 1), q(0, 1)]]
    );
    let touching = [p(0, 0), p(4, 0), p(4, 2), p(2, 0)];
    let report = self_intersections_exact_spatial_polyline(&touching, 1_000_000).unwrap();
    assert_eq!(report.intersections.len(), 1);
    assert_eq!(report.intersections[0].spans, [0, 2]);
    assert_eq!(report.intersections[0].points_mm, [p(2, 0), p(2, 0)]);
}

#[test]
fn exact_spatial_polyline_distance_finds_interior_pair_on_skew_spans() {
    use brep_core::analysis::distance_between_exact_spatial_polylines;
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x, y, z| [q(x, 1), q(y, 1), q(z, 1)];
    let a = [p(0, 0, 0), p(3, 0, 0)];
    let b = [p(1, -2, 2), p(2, 2, 2)];
    let report = distance_between_exact_spatial_polylines(&a, &b, 1_000_000).unwrap();
    assert_eq!(report.distance_squared_mm2, q(4, 1));
    assert_eq!(report.spans, [0, 0]);
    assert_eq!(report.parameters, [q(1, 2), q(1, 2)]);
    assert_eq!(
        report.points_mm,
        [[q(3, 2), q(0, 1), q(0, 1)], [q(3, 2), q(0, 1), q(2, 1)]]
    );
    assert_eq!(
        distance_between_exact_spatial_polylines(&a, &b, report.exact_work).unwrap(),
        report
    );
    assert!(distance_between_exact_spatial_polylines(&a, &b, report.exact_work - 1).is_err());
}

#[test]
fn exact_spatial_polyline_distance_handles_parallel_ties_and_stationary_spans() {
    use brep_core::analysis::distance_between_exact_spatial_polylines;
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x, y| [q(x, 1), q(y, 1), q(0, 1)];
    let a = [p(0, 0), p(4, 0), p(4, 0)];
    let b = [p(5, 1), p(2, 1), p(2, 1)];
    let report = distance_between_exact_spatial_polylines(&a, &b, 1_000_000).unwrap();
    assert_eq!(report.distance_squared_mm2, q(1, 1));
    assert_eq!(report.spans, [0, 0]);
    assert_eq!(report.parameters, [q(1, 2), q(1, 1)]);
    assert_eq!(report.points_mm, [p(2, 0), p(2, 1)]);
    let stationary = [p(4, 0), p(4, 0)];
    let report = distance_between_exact_spatial_polylines(&stationary, &a, 1_000_000).unwrap();
    assert_eq!(report.distance_squared_mm2, q(0, 1));
    assert_eq!(report.parameters, [q(0, 1), q(1, 1)]);
}

#[test]
fn bounded_spatial_polyline_simplification_certifies_full_span_deviation() {
    use brep_core::analysis::simplify_bounded_spatial_polyline;
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x, y| [q(x, 1), q(y, 10), q(0, 1)];
    let path = [p(0, 0), p(1, 1), p(2, -1), p(3, 0)];
    let report = simplify_bounded_spatial_polyline(&path, &q(1, 10), 1_000_000).unwrap();
    assert_eq!(report.source_vertices, [0, 3]);
    assert_eq!(report.points_mm, [p(0, 0), p(3, 0)]);
    assert_eq!(report.deviation_squared_mm2, q(1, 100));
    let tighter = simplify_bounded_spatial_polyline(&path, &q(1, 100), 1_000_000).unwrap();
    assert_eq!(tighter.source_vertices, [0, 1, 2, 3]);
    assert_eq!(tighter.deviation_squared_mm2, q(0, 1));
    assert_eq!(
        simplify_bounded_spatial_polyline(&path, &q(1, 10), report.exact_work).unwrap(),
        report
    );
    assert!(simplify_bounded_spatial_polyline(&path, &q(1, 10), report.exact_work - 1).is_err());
}

#[test]
fn bounded_spatial_polyline_simplification_preserves_retracing_and_a_closed_anchor() {
    use brep_core::analysis::simplify_bounded_spatial_polyline;
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x| [q(x), q(0), q(0)];
    let path = [p(0), p(3), p(1), p(4)];
    let report = simplify_bounded_spatial_polyline(&path, &q(0), 1_000_000).unwrap();
    assert_eq!(report.source_vertices, [0, 1, 2, 3]);
    let closed = [p(0), p(1), p(2), p(0)];
    let report = simplify_bounded_spatial_polyline(&closed, &q(0), 1_000_000).unwrap();
    assert_eq!(report.source_vertices, [0, 2, 3]);
    assert_eq!(report.points_mm, [p(0), p(2), p(0)]);
    assert!(simplify_bounded_spatial_polyline(&path, &q(-1), 1_000_000).is_err());
}

#[test]
fn exact_spatial_polyline_noding_shares_crossing_nodes_with_exact_source_parameters() {
    use brep_core::analysis::{NodedSpatialPolylineEdge, node_exact_spatial_polyline};
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x, y| [q(x, 1), q(y, 1), q(x + y, 1)];
    let path = [p(0, 0), p(4, 4), p(0, 4), p(4, 0)];
    let report = node_exact_spatial_polyline(&path, 1_000_000).unwrap();
    assert_eq!(
        report.nodes_mm,
        [p(0, 0), p(2, 2), p(4, 4), p(0, 4), p(4, 0)]
    );
    let edge = |source_span, lo, hi, nodes| NodedSpatialPolylineEdge {
        source_span,
        parameters: [lo, hi],
        nodes,
    };
    assert_eq!(
        report.edges,
        [
            edge(0, q(0, 1), q(1, 2), [0, 1]),
            edge(0, q(1, 2), q(1, 1), [1, 2]),
            edge(1, q(0, 1), q(1, 1), [2, 3]),
            edge(2, q(0, 1), q(1, 2), [3, 1]),
            edge(2, q(1, 2), q(1, 1), [1, 4]),
        ],
    );
    assert_eq!(
        node_exact_spatial_polyline(&path, report.exact_work).unwrap(),
        report
    );
    assert!(node_exact_spatial_polyline(&path, report.exact_work - 1).is_err());
}

#[test]
fn exact_spatial_polyline_noding_preserves_retraced_and_stationary_occurrences() {
    use brep_core::analysis::node_exact_spatial_polyline;
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x| [q(x), q(x), q(0)];
    let path = [p(0), p(4), p(4), p(2), p(6)];
    let report = node_exact_spatial_polyline(&path, 1_000_000).unwrap();
    assert_eq!(report.nodes_mm, [p(0), p(2), p(4), p(6)]);
    assert_eq!(
        report
            .edges
            .iter()
            .map(|edge| (edge.source_span, edge.nodes))
            .collect::<Vec<_>>(),
        [
            (0, [0, 1]),
            (0, [1, 2]),
            (1, [2, 2]),
            (2, [2, 1]),
            (3, [1, 2]),
            (3, [2, 3])
        ],
    );
}

#[test]
fn exact_planar_spatial_polyline_area_measures_tilted_profile_and_reversal() {
    use brep_core::analysis::exact_planar_spatial_polyline_area;
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x: i64, y: i64| [q(x + 5), q(y - 6), q(x + y + 7)];
    let path = [p(0, 0), p(3, 0), p(3, 2), p(3, 2), p(0, 2), p(0, 0)];
    let report = exact_planar_spatial_polyline_area(&path, 1_000_000).unwrap();
    assert_eq!(report.area_vector_mm2, [q(-6), q(-6), q(6)]);
    assert_eq!(report.area_squared_mm4, q(108));
    assert_eq!(
        exact_planar_spatial_polyline_area(&path, report.exact_work).unwrap(),
        report
    );
    assert!(exact_planar_spatial_polyline_area(&path, report.exact_work - 1).is_err());
    let reverse = path.into_iter().rev().collect::<Vec<_>>();
    let reversed = exact_planar_spatial_polyline_area(&reverse, 1_000_000).unwrap();
    assert_eq!(reversed.area_vector_mm2, [q(6), q(6), q(-6)]);
    assert_eq!(reversed.area_squared_mm4, report.area_squared_mm4);
}

#[test]
fn exact_planar_spatial_polyline_area_rejects_warped_and_self_crossing_profiles() {
    use brep_core::analysis::exact_planar_spatial_polyline_area;
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y, z| [q(x), q(y), q(z)];
    let warped = [p(0, 0, 0), p(2, 0, 0), p(2, 2, 0), p(0, 2, 1), p(0, 0, 0)];
    let crossing = [p(0, 0, 0), p(4, 4, 0), p(0, 4, 0), p(2, 0, 0), p(0, 0, 0)];
    let collapsed = [p(0, 0, 0), p(2, 0, 0), p(1, 0, 0), p(0, 0, 0)];
    for path in [
        warped.as_slice(),
        crossing.as_slice(),
        collapsed.as_slice(),
        &warped[..4],
    ] {
        assert!(exact_planar_spatial_polyline_area(path, 1_000_000).is_err());
    }
}

#[test]
fn exact_planar_polyline_centroid_handles_a_concave_tilted_profile() {
    use brep_core::analysis::exact_planar_spatial_polyline_centroid;
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x: i64, y: i64| [q(x + 7, 1), q(y - 8, 1), q(x + 2 * y, 1)];
    let path = [
        p(0, 0),
        p(3, 0),
        p(3, 1),
        p(1, 1),
        p(1, 3),
        p(0, 3),
        p(0, 0),
    ];
    let report = exact_planar_spatial_polyline_centroid(&path, 1_000_000).unwrap();
    assert_eq!(report.centroid_mm, [q(81, 10), q(-69, 10), q(33, 10)]);
    assert_eq!(report.area.area_vector_mm2, [q(-5, 1), q(-10, 1), q(5, 1)]);
    assert_eq!(
        exact_planar_spatial_polyline_centroid(&path, report.exact_work).unwrap(),
        report
    );
    assert!(exact_planar_spatial_polyline_centroid(&path, report.exact_work - 1).is_err());
    let reversed = path.into_iter().rev().collect::<Vec<_>>();
    let reverse = exact_planar_spatial_polyline_centroid(&reversed, 1_000_000).unwrap();
    assert_eq!(reverse.centroid_mm, report.centroid_mm);
}

#[test]
fn exact_planar_polyline_centroid_supports_vertical_profiles_and_refuses_warp() {
    use brep_core::analysis::exact_planar_spatial_polyline_centroid;
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x, y, z| [q(x, 1), q(y, 1), q(z, 1)];
    let path = [p(2, 0, 0), p(2, 1, 0), p(2, 0, 1), p(2, 0, 0)];
    let report = exact_planar_spatial_polyline_centroid(&path, 1_000_000).unwrap();
    assert_eq!(report.centroid_mm, [q(2, 1), q(1, 3), q(1, 3)]);
    let warped = [p(2, 0, 0), p(2, 1, 0), p(2, 1, 1), p(3, 0, 1), p(2, 0, 0)];
    assert!(exact_planar_spatial_polyline_centroid(&warped, 1_000_000).is_err());
}

#[test]
fn exact_planar_polyline_point_locations_distinguish_concavity_boundary_and_plane() {
    use brep_core::analysis::{
        PlanarPolylinePointLocation::*, locate_points_in_exact_planar_spatial_polyline,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x: i64, y: i64| [q(x), q(y), q(x + y)];
    let path = [
        p(0, 0),
        p(4, 0),
        p(4, 2),
        p(2, 2),
        p(2, 4),
        p(0, 4),
        p(0, 0),
    ];
    let queries = [
        p(1, 3),
        p(3, 3),
        p(2, 2),
        p(2, 3),
        [q(1), q(3), q(5)],
        p(-1, 0),
    ];
    let report =
        locate_points_in_exact_planar_spatial_polyline(&path, &queries, 1_000_000).unwrap();
    assert_eq!(
        report.locations,
        [Interior, Exterior, Boundary, Boundary, OffPlane, Exterior]
    );
    assert_eq!(
        locate_points_in_exact_planar_spatial_polyline(&path, &queries, report.exact_work).unwrap(),
        report
    );
    assert!(
        locate_points_in_exact_planar_spatial_polyline(&path, &queries, report.exact_work - 1)
            .is_err()
    );
}

#[test]
fn exact_planar_polyline_point_locations_ignore_winding_and_duplicate_vertices() {
    use brep_core::analysis::{
        PlanarPolylinePointLocation::*, locate_points_in_exact_planar_spatial_polyline,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |y, z| [q(0), q(y), q(z)];
    let path = [p(0, 0), p(0, 4), p(0, 4), p(4, 4), p(4, 0), p(0, 0)];
    let queries = [p(2, 2), p(0, 4), p(0, 5), p(2, 4)];
    let reversed = path.clone().into_iter().rev().collect::<Vec<_>>();
    for contour in [&path[..], &reversed] {
        let report =
            locate_points_in_exact_planar_spatial_polyline(contour, &queries, 1_000_000).unwrap();
        assert_eq!(report.locations, [Interior, Boundary, Exterior, Boundary]);
    }
    assert!(
        locate_points_in_exact_planar_spatial_polyline(&path[..5], &queries, 1_000_000).is_err()
    );
}

#[test]
fn exact_planar_polyline_path_clip_keeps_disjoint_coplanar_intervals() {
    use brep_core::analysis::clip_spatial_path_to_exact_planar_polyline;
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x: i64, y: i64| [q(x, 1), q(y, 1), q(x + y, 1)];
    let contour = [
        p(0, 0),
        p(4, 0),
        p(4, 4),
        p(3, 4),
        p(3, 1),
        p(1, 1),
        p(1, 4),
        p(0, 4),
        p(0, 0),
    ];
    let path = [p(-1, 2), p(5, 2), p(-1, 2)];
    let report = clip_spatial_path_to_exact_planar_polyline(&contour, &path, 1_000_000).unwrap();
    let expected = vec![[q(1, 6), q(1, 3)], [q(2, 3), q(5, 6)]];
    assert_eq!(report.span_intervals, [expected.clone(), expected]);
    assert_eq!(
        clip_spatial_path_to_exact_planar_polyline(&contour, &path, report.exact_work).unwrap(),
        report
    );
    assert!(
        clip_spatial_path_to_exact_planar_polyline(&contour, &path, report.exact_work - 1).is_err()
    );
}

#[test]
fn exact_planar_polyline_path_clip_keeps_transverse_and_stationary_contacts() {
    use brep_core::analysis::clip_spatial_path_to_exact_planar_polyline;
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x, y, z| [q(x, 1), q(y, 1), q(z, 1)];
    let contour = [p(0, 0, 0), p(0, 2, 0), p(0, 2, 2), p(0, 0, 2), p(0, 0, 0)];
    let crossing = [p(-1, 1, 1), p(2, 1, 1)];
    let report =
        clip_spatial_path_to_exact_planar_polyline(&contour, &crossing, 1_000_000).unwrap();
    assert_eq!(report.span_intervals, [vec![[q(1, 3), q(1, 3)]]]);
    let boundary = [
        p(1, 0, 1),
        p(0, 0, 1),
        p(0, 0, 1),
        p(-1, 0, 1),
        p(-1, 2, 3),
        p(1, 2, 3),
    ];
    let report =
        clip_spatial_path_to_exact_planar_polyline(&contour, &boundary, 1_000_000).unwrap();
    assert_eq!(
        report.span_intervals,
        [
            vec![[q(1, 1), q(1, 1)]],
            vec![[q(0, 1), q(1, 1)]],
            vec![[q(0, 1), q(0, 1)]],
            vec![],
            vec![],
        ]
    );
}

#[test]
fn exact_planar_polyline_region_projection_uses_the_filled_tilted_profile() {
    use brep_core::analysis::{
        PlanarPolylinePointLocation::Interior, project_point_to_exact_planar_polyline_region,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x: i64, y: i64| [q(x, 1), q(y, 1), q(x + y, 1)];
    let profile = [p(0, 0), p(3, 0), p(3, 3), p(0, 3), p(0, 0)];
    let query = [q(0, 1), q(0, 1), q(3, 1)];
    let report =
        project_point_to_exact_planar_polyline_region(&profile, &query, 1_000_000).unwrap();
    assert_eq!(report.closest_mm, p(1, 1));
    assert_eq!(report.distance_squared_mm2, q(3, 1));
    assert_eq!(report.location, Interior);
    assert_eq!(
        project_point_to_exact_planar_polyline_region(&profile, &query, report.exact_work).unwrap(),
        report
    );
    assert!(
        project_point_to_exact_planar_polyline_region(&profile, &query, report.exact_work - 1)
            .is_err()
    );
}

#[test]
fn exact_planar_polyline_region_projection_falls_back_to_concave_boundary() {
    use brep_core::analysis::{
        PlanarPolylinePointLocation::Boundary, project_point_to_exact_planar_polyline_region,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y| [q(x), q(y), q(0)];
    let profile = [
        p(0, 0),
        p(4, 0),
        p(4, 2),
        p(2, 2),
        p(2, 4),
        p(0, 4),
        p(0, 0),
    ];
    let query = [q(3), q(3), q(2)];
    let report =
        project_point_to_exact_planar_polyline_region(&profile, &query, 1_000_000).unwrap();
    assert_eq!(report.closest_mm, p(3, 2));
    assert_eq!(report.distance_squared_mm2, q(5));
    assert_eq!(report.location, Boundary);
    let on_edge = [q(2), q(3), q(2)];
    let report =
        project_point_to_exact_planar_polyline_region(&profile, &on_edge, 1_000_000).unwrap();
    assert_eq!(report.closest_mm, p(2, 3));
    assert_eq!(report.distance_squared_mm2, q(4));
    assert_eq!(report.location, Boundary);
}

#[test]
fn exact_planar_polyline_triangulation_preserves_concavity_and_boundary_vertices() {
    use brep_core::analysis::triangulate_exact_planar_spatial_polyline;
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x: i64, y: i64| [q(x), q(y), q(2 * x + y)];
    let contour = [
        p(0, 0),
        p(2, 0),
        p(4, 0),
        p(4, 1),
        p(1, 1),
        p(1, 4),
        p(0, 4),
        p(0, 0),
    ];
    let report = triangulate_exact_planar_spatial_polyline(&contour, 1_000_000).unwrap();
    assert_eq!(report.triangles.len(), 5);
    let mut indices = std::collections::BTreeSet::new();
    let mut area = q(0);
    for [a, b, c] in &report.triangles {
        indices.extend([*a, *b, *c]);
        let [a, b, c] = [&contour[*a], &contour[*b], &contour[*c]];
        let twice = (&b[0] - &a[0]) * (&c[1] - &a[1]) - (&b[1] - &a[1]) * (&c[0] - &a[0]);
        assert!(twice > q(0));
        area += twice;
    }
    assert_eq!(
        indices.into_iter().collect::<Vec<_>>(),
        (0..7).collect::<Vec<_>>()
    );
    assert_eq!(area, q(14));
    assert_eq!(
        triangulate_exact_planar_spatial_polyline(&contour, report.exact_work).unwrap(),
        report
    );
    assert!(triangulate_exact_planar_spatial_polyline(&contour, report.exact_work - 1).is_err());
}

#[test]
fn exact_planar_polyline_triangulation_keeps_reversed_winding_and_ignores_duplicates() {
    use brep_core::analysis::triangulate_exact_planar_spatial_polyline;
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |y, z| [q(2), q(y), q(z)];
    let contour = [
        p(0, 0),
        p(0, 2),
        p(0, 2),
        p(3, 2),
        p(3, 0),
        p(0, 0),
        p(0, 0),
    ];
    let report = triangulate_exact_planar_spatial_polyline(&contour, 1_000_000).unwrap();
    assert_eq!(report.triangles.len(), 2);
    let mut area = q(0);
    for indices in report.triangles {
        assert!(indices.into_iter().all(|i| [0, 1, 3, 4].contains(&i)));
        let [a, b, c] = indices.map(|i| &contour[i]);
        let twice = (&b[1] - &a[1]) * (&c[2] - &a[2]) - (&b[2] - &a[2]) * (&c[1] - &a[1]);
        assert!(twice < q(0));
        area += twice;
    }
    assert_eq!(area, q(-12));
}

#[test]
fn exact_planar_polyline_second_moments_integrate_a_tilted_rectangle() {
    use brep_core::analysis::exact_planar_spatial_polyline_second_moments;
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x: i64, y: i64| [q(x + 5, 1), q(y - 2, 1), q(x + 2 * y, 1)];
    let contour = [p(0, 0), p(2, 0), p(2, 6), p(0, 6), p(0, 0)];
    let report = exact_planar_spatial_polyline_second_moments(&contour, 1_000_000).unwrap();
    assert_eq!(report.centroid_mm, p(1, 3));
    assert_eq!(report.area_squared_mm4, q(864, 1));
    assert_eq!(
        report.normalized_central_moments_mm2,
        [
            [q(1, 3), q(0, 1), q(1, 3)],
            [q(0, 1), q(3, 1), q(6, 1)],
            [q(1, 3), q(6, 1), q(37, 3)],
        ]
    );
    assert_eq!(
        exact_planar_spatial_polyline_second_moments(&contour, report.exact_work).unwrap(),
        report
    );
    assert!(exact_planar_spatial_polyline_second_moments(&contour, report.exact_work - 1).is_err());
}

#[test]
fn exact_planar_polyline_second_moments_preserve_negative_mixed_terms_and_winding() {
    use brep_core::analysis::exact_planar_spatial_polyline_second_moments;
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x, y| [q(x, 1), q(y, 1), q(0, 1)];
    let contour = [p(0, 0), p(1, 0), p(0, 1), p(0, 0)];
    let reversed = contour.clone().into_iter().rev().collect::<Vec<_>>();
    for path in [&contour[..], &reversed] {
        let report = exact_planar_spatial_polyline_second_moments(path, 1_000_000).unwrap();
        assert_eq!(report.centroid_mm, [q(1, 3), q(1, 3), q(0, 1)]);
        assert_eq!(
            report.normalized_central_moments_mm2,
            [
                [q(1, 18), q(-1, 36), q(0, 1)],
                [q(-1, 36), q(1, 18), q(0, 1)],
                [q(0, 1), q(0, 1), q(0, 1)],
            ]
        );
    }
}

#[test]
fn bounded_planar_polyline_principal_moments_are_exact_for_rational_spectra() {
    use brep_core::analysis::bounded_planar_spatial_polyline_principal_moments;
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let square = |tilt: i64| {
        let p = |x: i64, y: i64| [q(x, 1), q(y, 1), q(tilt * (x + y), 1)];
        [p(0, 0), p(2, 0), p(2, 2), p(0, 2), p(0, 0)]
    };
    for (tilt, larger) in [(0, q(1, 3)), (1, q(1, 1))] {
        let contour = square(tilt);
        let report =
            bounded_planar_spatial_polyline_principal_moments(&contour, 1e-9, 1_000_000).unwrap();
        assert_eq!(
            report.principal_bounds_mm2,
            [[q(1, 3), q(1, 3)], [larger.clone(), larger]]
        );
        assert_eq!(
            bounded_planar_spatial_polyline_principal_moments(&contour, 1e-9, report.exact_work)
                .unwrap(),
            report
        );
        assert!(
            bounded_planar_spatial_polyline_principal_moments(
                &contour,
                1e-9,
                report.exact_work - 1
            )
            .is_err()
        );
    }
}

#[test]
fn bounded_planar_polyline_principal_moments_enclose_irrational_eigenvalues() {
    use brep_core::analysis::bounded_planar_spatial_polyline_principal_moments;
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x, y| [q(x, 1), q(y, 1), q(0, 1)];
    let path = [p(0, 0), p(2, 0), p(0, 1), p(0, 0)];
    let tolerance = 1e-10;
    let report =
        bounded_planar_spatial_polyline_principal_moments(&path, tolerance, 1_000_000).unwrap();
    let [[small_lo, small_hi], [large_lo, large_hi]] = report.principal_bounds_mm2;
    let from_small = |value: &Q| q(5, 1) - q(36, 1) * value;
    let from_large = |value: &Q| q(36, 1) * value - q(5, 1);
    assert!(from_small(&small_hi) > q(0, 1) && from_small(&small_hi).pow(2) < q(13, 1));
    assert!(from_small(&small_lo).pow(2) > q(13, 1));
    assert!(from_large(&large_lo) > q(0, 1) && from_large(&large_lo).pow(2) < q(13, 1));
    assert!(from_large(&large_hi).pow(2) > q(13, 1));
    let width = Q::from_float(tolerance).unwrap();
    assert!(small_hi - small_lo <= width);
    assert!(large_hi - large_lo <= width);
}

#[test]
fn exact_planar_polyline_region_mass_subtracts_disjoint_holes_independent_of_winding() {
    use brep_core::analysis::exact_planar_spatial_polyline_region_mass;
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x: i64, y: i64| [q(x, 1), q(y, 1), q(x + y, 1)];
    let rect = |x, y, a, b| [p(x, y), p(a, y), p(a, b), p(x, b), p(x, y)];
    let outer = rect(0, 0, 6, 6);
    let h1 = rect(1, 1, 2, 3);
    let h2 = rect(3, 1, 5, 2).into_iter().rev().collect::<Vec<_>>();
    let holes = [&h1[..], &h2[..]];
    let report = exact_planar_spatial_polyline_region_mass(&outer, &holes, 1_000_000).unwrap();
    assert_eq!(report.area_vector_mm2, [q(-32, 1), q(-32, 1), q(32, 1)]);
    assert_eq!(report.area_squared_mm4, q(3072, 1));
    assert_eq!(report.centroid_mm, [q(97, 32), q(101, 32), q(99, 16)]);
    assert_eq!(
        exact_planar_spatial_polyline_region_mass(&outer, &holes, report.exact_work).unwrap(),
        report
    );
    assert!(
        exact_planar_spatial_polyline_region_mass(&outer, &holes, report.exact_work - 1).is_err()
    );
}

#[test]
fn exact_planar_polyline_region_mass_refuses_touching_nested_and_off_plane_holes() {
    use brep_core::analysis::exact_planar_spatial_polyline_region_mass;
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let rect = |x, y, a, b, z| {
        [
            [q(x), q(y), q(z)],
            [q(a), q(y), q(z)],
            [q(a), q(b), q(z)],
            [q(x), q(b), q(z)],
            [q(x), q(y), q(z)],
        ]
    };
    let outer = rect(0, 0, 6, 6, 0);
    let hole = rect(1, 1, 5, 5, 0);
    let nested = rect(2, 2, 3, 3, 0);
    let touching = rect(0, 1, 2, 3, 0);
    let off_plane = rect(1, 1, 2, 3, 1);
    assert!(
        exact_planar_spatial_polyline_region_mass(&outer, &[&hole, &nested], 1_000_000).is_err()
    );
    assert!(
        exact_planar_spatial_polyline_region_mass(&outer, &[&nested, &hole], 1_000_000).is_err()
    );
    assert!(exact_planar_spatial_polyline_region_mass(&outer, &[&touching], 1_000_000).is_err());
    assert!(exact_planar_spatial_polyline_region_mass(&outer, &[&off_plane], 1_000_000).is_err());
}

#[test]
fn exact_planar_polyline_region_path_clip_preserves_holes_and_their_edges() {
    use brep_core::analysis::clip_spatial_path_to_exact_planar_polyline_region;
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x: i64, y: i64| [q(x, 1), q(y, 1), q(x + y, 1)];
    let rect = |x, y, a, b| [p(x, y), p(a, y), p(a, b), p(x, b), p(x, y)];
    let outer = rect(0, 0, 6, 6);
    let a = rect(1, 1, 2, 3);
    let b = rect(4, 1, 5, 3).into_iter().rev().collect::<Vec<_>>();
    let holes = [&a[..], &b[..]];
    let path = [p(-1, 2), p(7, 2), p(7, 1), p(-1, 1)];
    let report =
        clip_spatial_path_to_exact_planar_polyline_region(&outer, &holes, &path, 1_000_000)
            .unwrap();
    assert_eq!(
        report.span_intervals,
        [
            vec![[q(1, 8), q(1, 4)], [q(3, 8), q(5, 8)], [q(3, 4), q(7, 8)]],
            vec![],
            vec![[q(1, 8), q(7, 8)]],
        ],
    );
    assert_eq!(
        clip_spatial_path_to_exact_planar_polyline_region(&outer, &holes, &path, report.exact_work)
            .unwrap(),
        report
    );
    assert!(
        clip_spatial_path_to_exact_planar_polyline_region(
            &outer,
            &holes,
            &path,
            report.exact_work - 1
        )
        .is_err()
    );
}

#[test]
fn exact_planar_polyline_region_path_clip_checks_transverse_hole_intersections() {
    use brep_core::analysis::clip_spatial_path_to_exact_planar_polyline_region;
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x, y, z| [q(x, 1), q(y, 1), q(z, 1)];
    let rect = |lo, hi| {
        [
            p(lo, lo, 0),
            p(hi, lo, 0),
            p(hi, hi, 0),
            p(lo, hi, 0),
            p(lo, lo, 0),
        ]
    };
    let outer = rect(0, 4);
    let hole = rect(1, 3);
    for (x, expected) in [
        (0, true),
        (1, true),
        (2, false),
        (3, true),
        (4, true),
        (5, false),
    ] {
        let path = [p(x, 2, -1), p(x, 2, 1)];
        let report =
            clip_spatial_path_to_exact_planar_polyline_region(&outer, &[&hole], &path, 1_000_000)
                .unwrap();
        let intervals = if expected {
            vec![[q(1, 2), q(1, 2)]]
        } else {
            vec![]
        };
        assert_eq!(report.span_intervals, [intervals]);
    }
    let path = [p(0, 2, -1), p(0, 2, 1)];
    assert!(
        clip_spatial_path_to_exact_planar_polyline_region(&outer, &[&outer], &path, 1_000_000)
            .is_err()
    );
}

#[test]
fn exact_planar_polyline_region_second_moments_account_for_an_eccentric_hole() {
    use brep_core::analysis::exact_planar_spatial_polyline_region_second_moments;
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x, y| [q(x, 1), q(y, 1), q(0, 1)];
    let rect = |lo, hi| [p(lo, lo), p(hi, lo), p(hi, hi), p(lo, hi), p(lo, lo)];
    let outer = rect(0, 4);
    let hole = rect(1, 2);
    let report =
        exact_planar_spatial_polyline_region_second_moments(&outer, &[&hole], 1_000_000).unwrap();
    assert_eq!(report.area_squared_mm4, q(225, 1));
    assert_eq!(report.centroid_mm, [q(61, 30), q(61, 30), q(0, 1)]);
    assert_eq!(
        report.normalized_central_moments_mm2,
        [
            [q(1259, 900), q(-4, 225), q(0, 1)],
            [q(-4, 225), q(1259, 900), q(0, 1)],
            [q(0, 1), q(0, 1), q(0, 1)],
        ],
    );
    assert_eq!(
        exact_planar_spatial_polyline_region_second_moments(&outer, &[&hole], report.exact_work)
            .unwrap(),
        report
    );
    assert!(
        exact_planar_spatial_polyline_region_second_moments(
            &outer,
            &[&hole],
            report.exact_work - 1
        )
        .is_err()
    );
}

#[test]
fn exact_planar_polyline_region_second_moments_are_winding_independent_and_validate_holes() {
    use brep_core::analysis::exact_planar_spatial_polyline_region_second_moments;
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x: i64, y: i64| [q(x), q(y), q(x + y)];
    let rect = |lo, hi| [p(lo, lo), p(hi, lo), p(hi, hi), p(lo, hi), p(lo, lo)];
    let outer = rect(0, 6);
    let hole = rect(1, 4);
    let reversed_outer = outer.clone().into_iter().rev().collect::<Vec<_>>();
    let reversed_hole = hole.clone().into_iter().rev().collect::<Vec<_>>();
    let forward =
        exact_planar_spatial_polyline_region_second_moments(&outer, &[&hole], 1_000_000).unwrap();
    for (outline, cut) in [
        (&outer[..], &reversed_hole[..]),
        (&reversed_outer[..], &hole[..]),
    ] {
        let reversed =
            exact_planar_spatial_polyline_region_second_moments(outline, &[cut], 1_000_000)
                .unwrap();
        assert_eq!(
            reversed.normalized_central_moments_mm2,
            forward.normalized_central_moments_mm2
        );
    }
    assert!(
        exact_planar_spatial_polyline_region_second_moments(&outer, &[&outer], 1_000_000).is_err()
    );
}

#[test]
fn exact_transverse_planar_profiles_intersect_in_segments_separated_by_holes() {
    use brep_core::analysis::intersect_transverse_exact_planar_polyline_regions;
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y, z| [q(x), q(y), q(z)];
    let a = [p(0, 0, 0), p(6, 0, 0), p(6, 6, 0), p(0, 6, 0), p(0, 0, 0)];
    let hole = [p(2, 1, 0), p(4, 1, 0), p(4, 5, 0), p(2, 5, 0), p(2, 1, 0)];
    let b = [
        p(1, 3, -1),
        p(5, 3, -1),
        p(5, 3, 1),
        p(1, 3, 1),
        p(1, 3, -1),
    ];
    let report =
        intersect_transverse_exact_planar_polyline_regions(&a, &[&hole], &b, &[], 1_000_000)
            .unwrap();
    assert_eq!(
        report.segments_mm,
        [[p(1, 3, 0), p(2, 3, 0)], [p(4, 3, 0), p(5, 3, 0)]]
    );
    assert_eq!(
        intersect_transverse_exact_planar_polyline_regions(&b, &[], &a, &[&hole], 1_000_000)
            .unwrap()
            .segments_mm,
        report.segments_mm
    );
    assert_eq!(
        intersect_transverse_exact_planar_polyline_regions(
            &a,
            &[&hole],
            &b,
            &[],
            report.exact_work
        )
        .unwrap(),
        report
    );
    assert!(
        intersect_transverse_exact_planar_polyline_regions(
            &a,
            &[&hole],
            &b,
            &[],
            report.exact_work - 1
        )
        .is_err()
    );
}

#[test]
fn exact_transverse_planar_profiles_keep_isolated_contacts_and_refuse_parallel_supports() {
    use brep_core::analysis::intersect_transverse_exact_planar_polyline_regions;
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y, z| [q(x), q(y), q(z)];
    let a = [p(0, 0, 0), p(3, 0, 0), p(0, 3, 0), p(0, 0, 0)];
    let b = [p(1, 0, 0), p(2, 0, 1), p(0, 0, 1), p(1, 0, 0)];
    let report =
        intersect_transverse_exact_planar_polyline_regions(&a, &[], &b, &[], 1_000_000).unwrap();
    assert_eq!(report.segments_mm, [[p(1, 0, 0), p(1, 0, 0)]]);
    assert!(
        intersect_transverse_exact_planar_polyline_regions(&a, &[], &a, &[], 1_000_000).is_err()
    );
    let outside = b.map(|[x, y, z]| [x + q(5), y, z]);
    let report =
        intersect_transverse_exact_planar_polyline_regions(&a, &[], &outside, &[], 1_000_000)
            .unwrap();
    assert!(report.segments_mm.is_empty());
}

#[test]
fn exact_coplanar_profile_overlap_area_accounts_for_overlapping_holes() {
    use brep_core::analysis::exact_coplanar_planar_polyline_region_overlap_area;
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x: i64, y: i64| [q(x), q(y), q(x + y)];
    let rect = |x, y, a, b| [p(x, y), p(a, y), p(a, b), p(x, b), p(x, y)];
    let a = rect(0, 0, 6, 6);
    let b = rect(2, 0, 7, 5);
    let h1 = rect(1, 1, 4, 4);
    let h2 = rect(3, 2, 5, 4).into_iter().rev().collect::<Vec<_>>();
    let report =
        exact_coplanar_planar_polyline_region_overlap_area(&a, &[&h1], &b, &[&h2], 1_000_000)
            .unwrap();
    assert_eq!(report.area_vector_mm2, [q(-12), q(-12), q(12)]);
    assert_eq!(report.area_squared_mm4, q(432));
    assert_eq!(
        exact_coplanar_planar_polyline_region_overlap_area(
            &a,
            &[&h1],
            &b,
            &[&h2],
            report.exact_work
        )
        .unwrap(),
        report
    );
    assert!(
        exact_coplanar_planar_polyline_region_overlap_area(
            &a,
            &[&h1],
            &b,
            &[&h2],
            report.exact_work - 1
        )
        .is_err()
    );
}

#[test]
fn exact_coplanar_profile_overlap_area_handles_disconnected_overlap_and_edge_contact() {
    use brep_core::analysis::exact_coplanar_planar_polyline_region_overlap_area;
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y| [q(x), q(y), q(0)];
    let u = [
        p(0, 0),
        p(5, 0),
        p(5, 4),
        p(4, 4),
        p(4, 1),
        p(1, 1),
        p(1, 4),
        p(0, 4),
        p(0, 0),
    ];
    let bar = [p(-1, 2), p(-1, 3), p(6, 3), p(6, 2), p(-1, 2)];
    let report =
        exact_coplanar_planar_polyline_region_overlap_area(&bar, &[], &u, &[], 1_000_000).unwrap();
    assert_eq!(report.area_vector_mm2, [q(0), q(0), q(-2)]);
    let contact = bar.clone().map(|[x, y, z]| [x, y + q(2), z]);
    let report =
        exact_coplanar_planar_polyline_region_overlap_area(&contact, &[], &u, &[], 1_000_000)
            .unwrap();
    assert_eq!(report.area_squared_mm4, q(0));
    let raised = bar.map(|[x, y, z]| [x, y, z + q(1)]);
    assert!(
        exact_coplanar_planar_polyline_region_overlap_area(&raised, &[], &u, &[], 1_000_000)
            .is_err()
    );
}

#[test]
fn exact_coplanar_profile_overlap_mass_subtracts_hole_moments_in_three_dimensions() {
    use brep_core::analysis::exact_coplanar_planar_polyline_region_overlap_mass;
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x: i64, y: i64| [q(x, 1), q(y, 1), q(x + y + 7, 1)];
    let rect = |x, y, a, b| [p(x, y), p(a, y), p(a, b), p(x, b), p(x, y)];
    let a = rect(0, 0, 6, 6);
    let b = rect(2, 0, 7, 5);
    let h1 = rect(1, 1, 4, 4);
    let h2 = rect(3, 2, 5, 4).into_iter().rev().collect::<Vec<_>>();
    let report =
        exact_coplanar_planar_polyline_region_overlap_mass(&a, &[&h1], &b, &[&h2], 1_000_000)
            .unwrap();
    assert_eq!(report.area_vector_mm2, [q(-12, 1), q(-12, 1), q(12, 1)]);
    assert_eq!(report.centroid_mm, Some([q(53, 12), q(29, 12), q(83, 6)]));
    assert_eq!(
        exact_coplanar_planar_polyline_region_overlap_mass(
            &a,
            &[&h1],
            &b,
            &[&h2],
            report.exact_work
        )
        .unwrap(),
        report
    );
    assert!(
        exact_coplanar_planar_polyline_region_overlap_mass(
            &a,
            &[&h1],
            &b,
            &[&h2],
            report.exact_work - 1
        )
        .is_err()
    );
}

#[test]
fn exact_coplanar_profile_overlap_mass_handles_disconnected_and_zero_area_intersections() {
    use brep_core::analysis::exact_coplanar_planar_polyline_region_overlap_mass;
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x, y| [q(x, 1), q(y, 1), q(0, 1)];
    let u = [
        p(0, 0),
        p(5, 0),
        p(5, 4),
        p(4, 4),
        p(4, 1),
        p(2, 1),
        p(2, 4),
        p(0, 4),
        p(0, 0),
    ];
    let bar = [p(-1, 2), p(-1, 3), p(6, 3), p(6, 2), p(-1, 2)];
    let report =
        exact_coplanar_planar_polyline_region_overlap_mass(&bar, &[], &u, &[], 1_000_000).unwrap();
    assert_eq!(report.centroid_mm, Some([q(13, 6), q(5, 2), q(0, 1)]));
    assert_eq!(report.area_vector_mm2, [q(0, 1), q(0, 1), q(-3, 1)]);
    let contact = bar.map(|[x, y, z]| [x, y + q(2, 1), z]);
    let report =
        exact_coplanar_planar_polyline_region_overlap_mass(&contact, &[], &u, &[], 1_000_000)
            .unwrap();
    assert_eq!(report.centroid_mm, None);
    assert_eq!(report.area_squared_mm4, q(0, 1));
}

#[test]
fn exact_coplanar_profile_relation_distinguishes_all_overlap_and_touch_classes() {
    use brep_core::analysis::{
        CoplanarPolylineRegionRelation::*, classify_exact_coplanar_planar_polyline_regions,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x: i64, y: i64| [q(x), q(y), q(x + y)];
    let rect = |x, y, a, b| [p(x, y), p(a, y), p(a, b), p(x, b), p(x, y)];
    let outer = rect(0, 0, 4, 4);
    for (other, expected) in [
        (rect(5, 0, 6, 1), Disjoint),
        (rect(4, 4, 6, 6), TouchingAtPoints),
        (rect(4, 1, 6, 3), TouchingAlongEdges),
        (rect(2, 2, 6, 6), PartialOverlap),
        (rect(1, 1, 2, 2), SecondInsideFirst),
        (rect(0, 0, 6, 6), FirstInsideSecond),
        (outer.clone(), Equal),
    ] {
        let report =
            classify_exact_coplanar_planar_polyline_regions(&outer, &[], &other, &[], 1_000_000)
                .unwrap();
        assert_eq!(report.relation, expected);
        assert_eq!(
            classify_exact_coplanar_planar_polyline_regions(
                &outer,
                &[],
                &other,
                &[],
                report.exact_work
            )
            .unwrap(),
            report
        );
        assert!(
            classify_exact_coplanar_planar_polyline_regions(
                &outer,
                &[],
                &other,
                &[],
                report.exact_work - 1
            )
            .is_err()
        );
    }
}

#[test]
fn exact_coplanar_profile_relation_respects_voids_and_hole_boundary_contact() {
    use brep_core::analysis::{
        CoplanarPolylineRegionRelation::*, classify_exact_coplanar_planar_polyline_regions,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y| [q(x), q(y), q(0)];
    let rect = |lo, hi| [p(lo, lo), p(hi, lo), p(hi, hi), p(lo, hi), p(lo, lo)];
    let outline = rect(0, 6);
    let hole = rect(1, 5);
    let inside_void = rect(2, 4);
    let reversed = hole.clone().into_iter().rev().collect::<Vec<_>>();
    for (other, expected) in [
        (&inside_void[..], Disjoint),
        (&reversed[..], TouchingAlongEdges),
        (&outline[..], FirstInsideSecond),
    ] {
        let report = classify_exact_coplanar_planar_polyline_regions(
            &outline,
            &[&hole],
            other,
            &[],
            1_000_000,
        )
        .unwrap();
        assert_eq!(report.relation, expected);
    }
}

#[test]
fn exact_coplanar_overlap_cells_partition_a_holed_tilted_profile() {
    use brep_core::analysis::decompose_exact_coplanar_planar_polyline_region_overlap;
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |u: i64, v: i64| [q(-u - v + 3), q(u), q(v)];
    let rect = |a, b, c, d| [p(a, b), p(c, b), p(c, d), p(a, d), p(a, b)];
    let a = rect(0, 0, 6, 6);
    let hole = rect(2, 2, 4, 4);
    let b = rect(1, 1, 5, 5);
    let report =
        decompose_exact_coplanar_planar_polyline_region_overlap(&a, &[&hole], &b, &[], 1_000_000)
            .unwrap();
    assert_eq!(
        report.cells_mm,
        [
            rect(1, 1, 2, 5)[..4].to_vec(),
            rect(2, 1, 4, 2)[..4].to_vec(),
            rect(2, 4, 4, 5)[..4].to_vec(),
            rect(4, 1, 5, 5)[..4].to_vec(),
        ]
    );
    assert_eq!(
        decompose_exact_coplanar_planar_polyline_region_overlap(
            &a,
            &[&hole],
            &b,
            &[],
            report.exact_work
        )
        .unwrap(),
        report
    );
    assert!(
        decompose_exact_coplanar_planar_polyline_region_overlap(
            &a,
            &[&hole],
            &b,
            &[],
            report.exact_work - 1
        )
        .is_err()
    );
}

#[test]
fn exact_coplanar_overlap_cells_split_at_crossings_and_ignore_edge_only_contact() {
    use brep_core::analysis::decompose_exact_coplanar_planar_polyline_region_overlap;
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x, y| [q(x, 1), q(y, 1), q(0, 1)];
    let a = [p(0, 0), p(5, 0), p(5, 2), p(0, 2), p(0, 0)];
    let b = [p(1, -1), p(4, 2), p(1, 2), p(1, -1)];
    let report =
        decompose_exact_coplanar_planar_polyline_region_overlap(&a, &[], &b, &[], 1_000_000)
            .unwrap();
    assert_eq!(
        report.cells_mm,
        [
            vec![p(1, 0), p(2, 0), p(2, 2), p(1, 2)],
            vec![p(2, 0), p(4, 2), p(2, 2)],
        ]
    );
    let contact = [p(0, 2), p(5, 2), p(5, 4), p(0, 4), p(0, 2)];
    assert!(
        decompose_exact_coplanar_planar_polyline_region_overlap(&a, &[], &contact, &[], 1_000_000)
            .unwrap()
            .cells_mm
            .is_empty()
    );
}

#[test]
fn exact_coplanar_boolean_cells_preserve_holes_and_material_for_all_operations() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::*, decompose_exact_coplanar_planar_polyline_region_boolean,
        exact_planar_spatial_polyline_area,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y| [q(x), q(y), q(0)];
    let rect = |a, b, c, d| [p(a, b), p(c, b), p(c, d), p(a, d), p(a, b)];
    let a = rect(0, 0, 6, 6);
    let hole = rect(2, 2, 4, 4);
    let b = rect(1, 1, 5, 5);
    for (operation, expected) in [
        (Intersection, 12),
        (Union, 36),
        (Difference, 20),
        (SymmetricDifference, 24),
    ] {
        let report = decompose_exact_coplanar_planar_polyline_region_boolean(
            &a,
            &[&hole],
            &b,
            &[],
            operation,
            1_000_000,
        )
        .unwrap();
        let area: Q = report
            .cells_mm
            .iter()
            .map(|cell| {
                let mut ring = cell.clone();
                ring.push(cell[0].clone());
                exact_planar_spatial_polyline_area(&ring, 1_000_000)
                    .unwrap()
                    .area_vector_mm2[2]
                    .clone()
            })
            .sum();
        assert_eq!(area, q(expected));
        assert_eq!(
            decompose_exact_coplanar_planar_polyline_region_boolean(
                &a,
                &[&hole],
                &b,
                &[],
                operation,
                report.exact_work
            )
            .unwrap(),
            report
        );
        assert!(
            decompose_exact_coplanar_planar_polyline_region_boolean(
                &a,
                &[&hole],
                &b,
                &[],
                operation,
                report.exact_work - 1
            )
            .is_err()
        );
    }
}

#[test]
fn exact_coplanar_boolean_cells_merge_simultaneous_boundary_events() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::*, decompose_exact_coplanar_planar_polyline_region_boolean,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y| [q(x), q(y), q(0)];
    let a = [p(0, 0), p(4, 0), p(4, 1), p(0, 1), p(0, 0)];
    let b = [p(0, 1), p(4, 1), p(4, 2), p(0, 2), p(0, 1)];
    for op in [Union, SymmetricDifference] {
        let report = decompose_exact_coplanar_planar_polyline_region_boolean(
            &a,
            &[],
            &b,
            &[],
            op,
            1_000_000,
        )
        .unwrap();
        assert_eq!(report.cells_mm, [vec![p(0, 0), p(4, 0), p(4, 2), p(0, 2)]]);
    }
    let same = decompose_exact_coplanar_planar_polyline_region_boolean(
        &a,
        &[],
        &a,
        &[],
        Difference,
        1_000_000,
    )
    .unwrap();
    assert!(same.cells_mm.is_empty());
}

#[test]
fn exact_coplanar_boolean_boundary_cancels_partial_shared_seams() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::Union, boundary_of_exact_coplanar_planar_polyline_region_boolean,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x: i64, y: i64| [q(x), q(y), q(0)];
    let rect = |a, b, c, d| [p(a, b), p(c, b), p(c, d), p(a, d), p(a, b)];
    let a = rect(0, 0, 2, 4);
    let b = rect(2, 1, 4, 3);
    let report = boundary_of_exact_coplanar_planar_polyline_region_boolean(
        &a,
        &[],
        &b,
        &[],
        Union,
        1_000_000,
    )
    .unwrap();
    let corners = [
        p(0, 0),
        p(2, 0),
        p(2, 1),
        p(4, 1),
        p(4, 3),
        p(2, 3),
        p(2, 4),
        p(0, 4),
    ];
    let expected: std::collections::BTreeSet<_> = corners
        .iter()
        .zip(corners.iter().cycle().skip(1))
        .map(|(a, b)| [a.clone(), b.clone()])
        .collect();
    assert_eq!(
        report
            .edges_mm
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>(),
        expected
    );
    assert_eq!(
        boundary_of_exact_coplanar_planar_polyline_region_boolean(
            &a,
            &[],
            &b,
            &[],
            Union,
            report.exact_work
        )
        .unwrap(),
        report
    );
    assert!(
        boundary_of_exact_coplanar_planar_polyline_region_boolean(
            &a,
            &[],
            &b,
            &[],
            Union,
            report.exact_work - 1
        )
        .is_err()
    );
}

#[test]
fn exact_coplanar_boolean_boundary_keeps_opposite_hole_orientation() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::Difference,
        boundary_of_exact_coplanar_planar_polyline_region_boolean,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x: i64, y: i64| [q(x), q(y), q(x + y)];
    let rect = |lo, hi| [p(lo, lo), p(hi, lo), p(hi, hi), p(lo, hi), p(lo, lo)];
    let outer = rect(0, 6);
    let cut = rect(2, 4);
    let report = boundary_of_exact_coplanar_planar_polyline_region_boolean(
        &outer,
        &[],
        &cut,
        &[],
        Difference,
        1_000_000,
    )
    .unwrap();
    let hole_edges = [
        [p(2, 2), p(2, 4)],
        [p(2, 4), p(4, 4)],
        [p(4, 4), p(4, 2)],
        [p(4, 2), p(2, 2)],
    ];
    assert!(hole_edges.iter().all(|edge| report.edges_mm.contains(edge)));
    let twice_area: Q = report
        .edges_mm
        .iter()
        .map(|[a, b]| &a[0] * &b[1] - &a[1] * &b[0])
        .sum();
    assert_eq!(twice_area, q(64));
    assert_eq!(report.edges_mm.len(), 12);
}

#[test]
fn exact_coplanar_boolean_loops_keep_point_touching_components_separate() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::Union, exact_planar_spatial_polyline_area,
        loops_of_exact_coplanar_planar_polyline_region_boolean,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y| [q(x), q(y), q(0)];
    let rect = |lo, hi| [p(lo, lo), p(hi, lo), p(hi, hi), p(lo, hi), p(lo, lo)];
    let a = rect(0, 2);
    let b = rect(2, 4);
    let mut reverse = a.clone();
    reverse.reverse();
    for (contour, expected) in [(a, q(4)), (reverse, q(-4))] {
        let report = loops_of_exact_coplanar_planar_polyline_region_boolean(
            &contour,
            &[],
            &b,
            &[],
            Union,
            1_000_000,
        )
        .unwrap();
        assert_eq!(report.loops_mm.len(), 2);
        for ring in &report.loops_mm {
            assert_eq!(ring.first(), ring.last());
            assert!(ring.contains(&p(2, 2)));
            assert_eq!(
                exact_planar_spatial_polyline_area(ring, 1_000_000)
                    .unwrap()
                    .area_vector_mm2[2],
                expected
            );
        }
        assert_eq!(
            loops_of_exact_coplanar_planar_polyline_region_boolean(
                &contour,
                &[],
                &b,
                &[],
                Union,
                report.exact_work
            )
            .unwrap(),
            report
        );
        assert!(
            loops_of_exact_coplanar_planar_polyline_region_boolean(
                &contour,
                &[],
                &b,
                &[],
                Union,
                report.exact_work - 1
            )
            .is_err()
        );
    }
}

#[test]
fn exact_coplanar_boolean_loops_keep_opposite_winding_for_holes() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::Difference, exact_planar_spatial_polyline_area,
        loops_of_exact_coplanar_planar_polyline_region_boolean,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y| [q(x), q(y), q(0)];
    let rect = |lo, hi| [p(lo, lo), p(lo, hi), p(hi, hi), p(hi, lo), p(lo, lo)];
    let a = rect(0, 6);
    let b = rect(2, 4);
    let report = loops_of_exact_coplanar_planar_polyline_region_boolean(
        &a,
        &[],
        &b,
        &[],
        Difference,
        1_000_000,
    )
    .unwrap();
    let mut areas = report
        .loops_mm
        .iter()
        .map(|ring| {
            exact_planar_spatial_polyline_area(ring, 1_000_000)
                .unwrap()
                .area_vector_mm2[2]
                .clone()
        })
        .collect::<Vec<_>>();
    areas.sort();
    assert_eq!(areas, [q(-36), q(4)]);
}

#[test]
fn exact_coplanar_boolean_mesh_is_conforming_at_partially_shared_edges() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::Union, boundary_of_exact_coplanar_planar_polyline_region_boolean,
        triangulate_exact_coplanar_planar_polyline_region_boolean,
    };
    use num_rational::BigRational as Q;
    use std::collections::{BTreeMap, BTreeSet};
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y| [q(x), q(y), q(0)];
    let rect = |a, b, c, d| [p(a, b), p(c, b), p(c, d), p(a, d), p(a, b)];
    let a = rect(0, 0, 2, 4);
    let b = rect(2, 1, 4, 3);
    let mesh = triangulate_exact_coplanar_planar_polyline_region_boolean(
        &a,
        &[],
        &b,
        &[],
        Union,
        1_000_000,
    )
    .unwrap();
    assert_eq!(
        mesh.vertices_mm.iter().collect::<BTreeSet<_>>().len(),
        mesh.vertices_mm.len()
    );
    let mut incidence = BTreeMap::new();
    let mut twice_area = q(0);
    for [a, b, c] in mesh.triangles {
        for (from, to) in [(a, b), (b, c), (c, a)] {
            *incidence.entry((from.min(to), from.max(to))).or_insert(0) +=
                if from < to { 1 } else { -1 };
        }
        let [a, b, c] = [
            &mesh.vertices_mm[a],
            &mesh.vertices_mm[b],
            &mesh.vertices_mm[c],
        ];
        let signed = (&b[0] - &a[0]) * (&c[1] - &a[1]) - (&b[1] - &a[1]) * (&c[0] - &a[0]);
        assert!(signed > q(0));
        twice_area += signed;
    }
    assert_eq!(twice_area, q(24));
    let actual: BTreeSet<_> = incidence
        .into_iter()
        .filter_map(|((a, b), sign)| match sign {
            0 => None,
            1 => Some([mesh.vertices_mm[a].clone(), mesh.vertices_mm[b].clone()]),
            -1 => Some([mesh.vertices_mm[b].clone(), mesh.vertices_mm[a].clone()]),
            _ => panic!("invalid incidence"),
        })
        .collect();
    let expected = boundary_of_exact_coplanar_planar_polyline_region_boolean(
        &a,
        &[],
        &b,
        &[],
        Union,
        1_000_000,
    )
    .unwrap();
    assert_eq!(actual, expected.edges_mm.into_iter().collect());
    assert!(
        triangulate_exact_coplanar_planar_polyline_region_boolean(
            &a,
            &[],
            &b,
            &[],
            Union,
            mesh.exact_work
        )
        .is_ok()
    );
    assert!(
        triangulate_exact_coplanar_planar_polyline_region_boolean(
            &a,
            &[],
            &b,
            &[],
            Union,
            mesh.exact_work - 1
        )
        .is_err()
    );
}

#[test]
fn exact_coplanar_boolean_mesh_preserves_tilted_hole_and_winding() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::Difference,
        triangulate_exact_coplanar_planar_polyline_region_boolean,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x: i64, y: i64| [q(x), q(y), q(7 - x - y)];
    let rect = |lo, hi| [p(lo, lo), p(lo, hi), p(hi, hi), p(hi, lo), p(lo, lo)];
    let outer = rect(0, 6);
    let cut = rect(2, 4);
    let mesh = triangulate_exact_coplanar_planar_polyline_region_boolean(
        &outer,
        &[],
        &cut,
        &[],
        Difference,
        1_000_000,
    )
    .unwrap();
    let mut twice_area = q(0);
    for triangle in mesh.triangles {
        let [a, b, c] = triangle.map(|i| &mesh.vertices_mm[i]);
        let signed = (&b[0] - &a[0]) * (&c[1] - &a[1]) - (&b[1] - &a[1]) * (&c[0] - &a[0]);
        assert!(signed < q(0));
        twice_area += signed;
        let x = &a[0] + &b[0] + &c[0];
        let y = &a[1] + &b[1] + &c[1];
        assert!(!(q(6) < x && x < q(12) && q(6) < y && y < q(12)));
    }
    assert_eq!(twice_area, q(-64));
}

#[test]
fn exact_coplanar_boolean_extrusion_builds_a_closed_holed_shell_with_exact_volume() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::Difference, extrude_exact_coplanar_planar_polyline_region_boolean,
    };
    use num_rational::BigRational as Q;
    use std::collections::BTreeMap;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x: i64, y: i64| [q(x), q(y), q(x + y)];
    let rect = |lo, hi| [p(lo, lo), p(lo, hi), p(hi, hi), p(hi, lo), p(lo, lo)];
    let a = rect(0, 6);
    let b = rect(2, 4);
    let sweep = [q(1), q(0), q(4)];
    let report = extrude_exact_coplanar_planar_polyline_region_boolean(
        &a,
        &[],
        &b,
        &[],
        Difference,
        &sweep,
        1_000_000,
    )
    .unwrap();
    assert_eq!(report.volume_mm3, q(96));
    let mut incidence = BTreeMap::<_, Vec<_>>::new();
    let mut six_volume = q(0);
    for &[a, b, c] in &report.triangles {
        for [from, to] in [[a, b], [b, c], [c, a]] {
            incidence
                .entry([from.min(to), from.max(to)])
                .or_default()
                .push(from < to);
        }
        let [a, b, c] = [a, b, c].map(|i| &report.vertices_mm[i]);
        for (i, j, k) in [(0, 1, 2), (1, 2, 0), (2, 0, 1)] {
            six_volume += &a[i] * (&b[j] * &c[k] - &b[k] * &c[j]);
        }
    }
    assert!(
        incidence
            .values()
            .all(|pair| pair.len() == 2 && pair[0] != pair[1])
    );
    assert_eq!(six_volume, q(576));
    assert_eq!(
        extrude_exact_coplanar_planar_polyline_region_boolean(
            &a,
            &[],
            &b,
            &[],
            Difference,
            &sweep,
            report.exact_work
        )
        .unwrap(),
        report
    );
    assert!(
        extrude_exact_coplanar_planar_polyline_region_boolean(
            &a,
            &[],
            &b,
            &[],
            Difference,
            &sweep,
            report.exact_work - 1
        )
        .is_err()
    );
}

#[test]
fn exact_coplanar_boolean_extrusion_rejects_pinched_and_tangential_sweeps() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::*, extrude_exact_coplanar_planar_polyline_region_boolean,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y| [q(x), q(y), q(0)];
    let rect = |lo, hi| [p(lo, lo), p(hi, lo), p(hi, hi), p(lo, hi), p(lo, lo)];
    let a = rect(0, 2);
    let touching = rect(2, 4);
    let distant = rect(3, 5);
    let normal = [q(0), q(0), q(-1)];
    assert!(
        extrude_exact_coplanar_planar_polyline_region_boolean(
            &a,
            &[],
            &touching,
            &[],
            Union,
            &normal,
            1_000_000
        )
        .is_err()
    );
    assert!(
        extrude_exact_coplanar_planar_polyline_region_boolean(
            &a,
            &[],
            &distant,
            &[],
            Union,
            &p(1, 0),
            1_000_000
        )
        .is_err()
    );
    let report = extrude_exact_coplanar_planar_polyline_region_boolean(
        &a,
        &[],
        &distant,
        &[],
        Union,
        &normal,
        1_000_000,
    )
    .unwrap();
    assert_eq!(report.volume_mm3, q(8));
    let empty = extrude_exact_coplanar_planar_polyline_region_boolean(
        &a,
        &[],
        &distant,
        &[],
        Intersection,
        &normal,
        1_000_000,
    )
    .unwrap();
    assert!(empty.triangles.is_empty());
    assert_eq!(empty.volume_mm3, q(0));
}

#[test]
fn exact_coplanar_boolean_brep_extrusion_qualifies_compact_holed_caps() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::Difference, extrude_exact_coplanar_polyline_boolean_brep,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y| [q(x), q(y), q(0)];
    let rect = |lo, hi| [p(lo, lo), p(hi, lo), p(hi, hi), p(lo, hi), p(lo, lo)];
    let outer = rect(0, 6);
    let cut = rect(2, 4);
    let result = extrude_exact_coplanar_polyline_boolean_brep(
        &outer,
        &[],
        &cut,
        &[],
        Difference,
        &[q(0), q(0), q(2)],
        1e-8,
        section_solid_options().admission,
        1_000_000,
    )
    .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(result.volume_mm3, q(64));
    assert!(result.solid.geometry().proven);
    assert_eq!(
        result
            .solid
            .model()
            .faces
            .iter()
            .filter(|face| face.holes.len() == 1)
            .count(),
        2
    );
}

#[test]
fn skew_boolean_extrusion_retains_exact_side_faces_and_inertia() {
    use brep_core::analysis::{CoplanarPolylineBoolean::Intersection, exact_polyhedral_inertia,
        exact_coplanar_polyline_boolean_extrusion_inertia, extrude_exact_coplanar_polyline_boolean_brep};
    use num_rational::BigRational as Q;
    let point = |x, y| [x, y, 0].map(|n: i64| Q::from_integer(n.into()));
    let square = |lo, hi| [point(lo,lo),point(hi,lo),point(hi,hi),point(lo,hi),point(lo,lo)];
    let first = square(0,3);
    let second = square(1,4);
    let sweep = [1i64,0,3].map(|n| Q::from_integer(n.into()));
    let solid = extrude_exact_coplanar_polyline_boolean_brep(&first,&[],&second,&[],Intersection,
        &sweep,1e-8,section_solid_options().admission,1_000_000).unwrap();
    assert!(solid.solid.geometry().proven);
    let measured = exact_polyhedral_inertia(&solid.solid,1_000_000).unwrap();
    let analytic = exact_coplanar_polyline_boolean_extrusion_inertia(&first,&[],&second,&[],
        Intersection,&sweep,1_000_000).unwrap();
    assert_eq!(solid.volume_mm3.to_string(),"12");
    assert_eq!(measured.centroid_mm,analytic.centroid_mm);
    assert_eq!(measured.inertia_mm5,analytic.inertia_mm5);
}

#[test]
fn exact_coplanar_boolean_brep_extrusion_refuses_rounded_vertices() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::Intersection, extrude_exact_coplanar_polyline_boolean_brep,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y| [q(x), q(y), q(0)];
    let a = [p(0, 0), p(2, 0), p(2, 2), p(0, 2), p(0, 0)];
    let error = extrude_exact_coplanar_polyline_boolean_brep(
        &a,
        &[],
        &a,
        &[],
        Intersection,
        &[q(0), q(0), Q::new(1.into(), 3.into())],
        1e-8,
        section_solid_options().admission,
        1_000_000,
    )
    .err()
    .expect("nonbinary coordinate must be refused");
    assert!(error.message.contains("binary64"));
}

#[test]
fn exact_coplanar_boolean_extrusion_inertia_integrates_an_oblique_rational_sweep() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::Intersection, exact_coplanar_polyline_boolean_extrusion_inertia,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x, y| [q(x, 1), q(y, 1), q(0, 1)];
    let a = [p(0, 0), p(2, 0), p(2, 4), p(0, 4), p(0, 0)];
    let sweep = [q(1, 3), q(0, 1), q(3, 1)];
    let report = exact_coplanar_polyline_boolean_extrusion_inertia(
        &a,
        &[],
        &a,
        &[],
        Intersection,
        &sweep,
        1_000_000,
    )
    .unwrap();
    assert_eq!(report.volume_mm3, q(24, 1));
    assert_eq!(report.centroid_mm, [q(7, 6), q(2, 1), q(3, 2)]);
    assert_eq!(
        report.inertia_mm5,
        [
            [q(50, 1), q(0, 1), q(-2, 1)],
            [q(0, 1), q(236, 9), q(0, 1)],
            [q(-2, 1), q(0, 1), q(362, 9)],
        ]
    );
    assert!(
        exact_coplanar_polyline_boolean_extrusion_inertia(
            &a,
            &[],
            &a,
            &[],
            Intersection,
            &sweep,
            report.exact_work
        )
        .is_ok()
    );
    assert!(
        exact_coplanar_polyline_boolean_extrusion_inertia(
            &a,
            &[],
            &a,
            &[],
            Intersection,
            &sweep,
            report.exact_work - 1
        )
        .is_err()
    );
}

#[test]
fn exact_coplanar_boolean_extrusion_inertia_subtracts_a_through_hole() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::Difference, exact_coplanar_polyline_boolean_extrusion_inertia,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y| [q(x), q(y), q(0)];
    let ring = |lo, hi| [p(lo, lo), p(hi, lo), p(hi, hi), p(lo, hi), p(lo, lo)];
    let a = ring(-2, 2);
    let b = ring(-1, 1);
    let report = exact_coplanar_polyline_boolean_extrusion_inertia(
        &a,
        &[],
        &b,
        &[],
        Difference,
        &[q(0), q(0), q(-2)],
        1_000_000,
    )
    .unwrap();
    assert_eq!(report.centroid_mm, [q(0), q(0), q(-1)]);
    assert_eq!(report.volume_mm3, q(24));
    assert_eq!(
        report.inertia_mm5,
        [
            [q(48), q(0), q(0)],
            [q(0), q(48), q(0)],
            [q(0), q(0), q(80)]
        ]
    );
}

#[test]
fn exact_boolean_extrusion_locations_distinguish_caps_holes_and_internal_seams() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::Difference, PolyhedralPathLocation::*,
        locate_points_in_exact_coplanar_polyline_boolean_extrusion,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x, y| [q(x, 1), q(y, 1), q(0, 1)];
    let rect = |lo, hi| [p(lo, lo), p(hi, lo), p(hi, hi), p(lo, hi), p(lo, lo)];
    let a = rect(0, 6);
    let b = rect(2, 4);
    let sweep = [q(1, 3), q(0, 1), q(-2, 1)];
    let at = |x, y, t: Q| {
        let base = p(x, y);
        std::array::from_fn(|i| &base[i] + &t * &sweep[i])
    };
    let queries = [
        at(2, 1, q(1, 2)),
        at(2, 3, q(1, 2)),
        at(3, 3, q(1, 2)),
        at(1, 1, q(0, 1)),
        at(1, 1, q(1, 1)),
        at(1, 1, q(3, 2)),
        at(3, 3, q(1, 1)),
        at(0, 1, q(1, 2)),
    ];
    let report = locate_points_in_exact_coplanar_polyline_boolean_extrusion(
        &a,
        &[],
        &b,
        &[],
        Difference,
        &sweep,
        &queries,
        1_000_000,
    )
    .unwrap();
    assert_eq!(
        report.locations,
        [
            Interior, Boundary, Exterior, Boundary, Boundary, Exterior, Exterior, Boundary
        ]
    );
    assert_eq!(
        locate_points_in_exact_coplanar_polyline_boolean_extrusion(
            &a,
            &[],
            &b,
            &[],
            Difference,
            &sweep,
            &queries,
            report.exact_work
        )
        .unwrap(),
        report
    );
    assert!(
        locate_points_in_exact_coplanar_polyline_boolean_extrusion(
            &a,
            &[],
            &b,
            &[],
            Difference,
            &sweep,
            &queries,
            report.exact_work - 1
        )
        .is_err()
    );
}

#[test]
fn exact_boolean_extrusion_locations_handle_point_contacts_and_empty_profiles() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::*, PolyhedralPathLocation::*,
        locate_points_in_exact_coplanar_polyline_boolean_extrusion,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y| [q(x), q(y), q(0)];
    let rect = |lo, hi| [p(lo, lo), p(lo, hi), p(hi, hi), p(hi, lo), p(lo, lo)];
    let a = rect(0, 2);
    let b = rect(2, 4);
    let queries = [[q(2), q(2), q(1)], [q(1), q(1), q(1)], [q(3), q(3), q(1)]];
    let sweep = [q(0), q(0), q(2)];
    let union = locate_points_in_exact_coplanar_polyline_boolean_extrusion(
        &a,
        &[],
        &b,
        &[],
        Union,
        &sweep,
        &queries,
        1_000_000,
    )
    .unwrap();
    assert_eq!(union.locations, [Boundary, Interior, Interior]);
    let empty = locate_points_in_exact_coplanar_polyline_boolean_extrusion(
        &a,
        &[],
        &b,
        &[],
        Intersection,
        &sweep,
        &queries,
        1_000_000,
    )
    .unwrap();
    assert_eq!(empty.locations, [Exterior; 3]);
    assert!(
        locate_points_in_exact_coplanar_polyline_boolean_extrusion(
            &a,
            &[],
            &b,
            &[],
            Union,
            &p(1, 0),
            &queries,
            1_000_000
        )
        .is_err()
    );
}

#[test]
fn exact_boolean_extrusion_path_clip_preserves_axial_limits_and_footprint_holes() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::Difference, clip_path_to_exact_coplanar_polyline_boolean_extrusion,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x, y| [q(x, 1), q(y, 1), q(0, 1)];
    let rect = |lo, hi| [p(lo, lo), p(hi, lo), p(hi, hi), p(lo, hi), p(lo, lo)];
    let a = rect(0, 6);
    let b = rect(2, 4);
    let sweep = [q(1, 3), q(0, 1), q(-2, 1)];
    let at = |x, y, t: Q| {
        let base = p(x, y);
        std::array::from_fn(|i| &base[i] + &t * &sweep[i])
    };
    let path = [at(-1, 3, q(-1, 4)), at(7, 3, q(5, 4))];
    let report = clip_path_to_exact_coplanar_polyline_boolean_extrusion(
        &a,
        &[],
        &b,
        &[],
        Difference,
        &sweep,
        &path,
        1_000_000,
    )
    .unwrap();
    assert_eq!(
        report.span_intervals,
        [vec![[q(1, 6), q(3, 8)], [q(5, 8), q(5, 6)]]]
    );
    assert_eq!(
        clip_path_to_exact_coplanar_polyline_boolean_extrusion(
            &a,
            &[],
            &b,
            &[],
            Difference,
            &sweep,
            &path,
            report.exact_work
        )
        .unwrap(),
        report
    );
    assert!(
        clip_path_to_exact_coplanar_polyline_boolean_extrusion(
            &a,
            &[],
            &b,
            &[],
            Difference,
            &sweep,
            &path,
            report.exact_work - 1
        )
        .is_err()
    );
}

#[test]
fn exact_boolean_extrusion_path_clip_keeps_isolated_cap_contact_and_wall_travel() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::Difference, clip_path_to_exact_coplanar_polyline_boolean_extrusion,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x, y, z| [q(x, 1), q(y, 1), q(z, 1)];
    let rect = |lo, hi| {
        [
            p(lo, lo, 0),
            p(hi, lo, 0),
            p(hi, hi, 0),
            p(lo, hi, 0),
            p(lo, lo, 0),
        ]
    };
    let a = rect(0, 6);
    let b = rect(2, 4);
    let sweep = p(0, 0, 2);
    let path = [
        p(-1, 1, 1),
        p(1, 1, -1),
        p(2, 3, -1),
        p(2, 3, 3),
        p(2, 3, 3),
    ];
    let report = clip_path_to_exact_coplanar_polyline_boolean_extrusion(
        &a,
        &[],
        &b,
        &[],
        Difference,
        &sweep,
        &path,
        1_000_000,
    )
    .unwrap();
    assert_eq!(
        report.span_intervals,
        [
            vec![[q(1, 2), q(1, 2)]],
            vec![],
            vec![[q(1, 4), q(3, 4)]],
            vec![],
        ]
    );
}

#[test]
fn bounded_boolean_extrusion_area_counts_both_caps_and_hole_walls() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::Difference, bounded_coplanar_polyline_boolean_extrusion_area,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y| [q(x), q(y), q(0)];
    let rect = |lo, hi| [p(lo, lo), p(hi, lo), p(hi, hi), p(lo, hi), p(lo, lo)];
    let a = rect(0, 6);
    let b = rect(2, 4);
    let sweep = [q(0), q(0), q(-2)];
    let get = |budget| {
        bounded_coplanar_polyline_boolean_extrusion_area(
            &a,
            &[],
            &b,
            &[],
            Difference,
            &sweep,
            1e-9,
            budget,
        )
    };
    let report = get(1_000_000).unwrap();
    assert_eq!(report.cap_bounds_mm2, [q(64), q(64)]);
    assert_eq!(report.wall_bounds_mm2, [q(64), q(64)]);
    assert_eq!(report.total_bounds_mm2, [q(128), q(128)]);
    assert_eq!(get(report.exact_work).unwrap(), report);
    assert!(get(report.exact_work - 1).is_err());
}

#[test]
fn bounded_boolean_extrusion_area_encloses_irrational_oblique_walls() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::Intersection, bounded_coplanar_polyline_boolean_extrusion_area,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y| [q(x), q(y), q(0)];
    let a = [p(0, 0), p(2, 0), p(2, 3), p(0, 3), p(0, 0)];
    let tolerance = 1e-9;
    let report = bounded_coplanar_polyline_boolean_extrusion_area(
        &a,
        &[],
        &a,
        &[],
        Intersection,
        &[q(1), q(0), q(2)],
        tolerance,
        1_000_000,
    )
    .unwrap();
    assert_eq!(report.cap_bounds_mm2, [q(12), q(12)]);
    let [lo, hi] = report.total_bounds_mm2;
    assert!(lo > q(20) && (&lo - q(20)).pow(2) < q(180));
    assert!((&hi - q(20)).pow(2) > q(180));
    assert!(hi - lo <= Q::from_float(tolerance).unwrap());
}

#[test]
fn exact_boolean_extrusion_section_clips_against_both_caps_and_keeps_holes() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::Difference, exact_planar_spatial_polyline_area,
        section_exact_coplanar_polyline_boolean_extrusion,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y, z| [q(x), q(y), q(z)];
    let rect = |lo, hi| {
        [
            p(lo, lo, 0),
            p(hi, lo, 0),
            p(hi, hi, 0),
            p(lo, hi, 0),
            p(lo, lo, 0),
        ]
    };
    let a = rect(0, 6);
    let b = rect(2, 4);
    let section = |budget| {
        section_exact_coplanar_polyline_boolean_extrusion(
            &a,
            &[],
            &b,
            &[],
            Difference,
            &p(0, 0, 2),
            &p(1, 0, 1),
            &q(4),
            budget,
        )
    };
    let report = section(1_000_000).unwrap();
    assert_eq!(report.cells_mm.len(), 2);
    let mut area_z = q(0);
    for mut ring in report.cells_mm.clone() {
        for [x, y, z] in &ring {
            assert_eq!(x + z, q(4));
            assert!(q(0) <= *z && *z <= q(2));
            assert!(*y <= q(2) || *y >= q(4));
        }
        ring.push(ring[0].clone());
        let area = exact_planar_spatial_polyline_area(&ring, 1_000_000).unwrap();
        assert_eq!(area.area_vector_mm2, p(4, 0, 4));
        area_z += &area.area_vector_mm2[2];
    }
    assert_eq!(area_z, q(8));
    assert_eq!(section(report.exact_work).unwrap(), report);
    assert!(section(report.exact_work - 1).is_err());
}

#[test]
fn exact_boolean_extrusion_section_preserves_a_coincident_cap_and_refuses_parallel_planes() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::Intersection, section_exact_coplanar_polyline_boolean_extrusion,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x, y| [q(x, 1), q(y, 1), q(0, 1)];
    let a = [p(0, 0), p(2, 0), p(2, 3), p(0, 3), p(0, 0)];
    let sweep = [q(1, 3), q(0, 1), q(-2, 1)];
    let normal = [q(0, 1), q(0, 1), q(1, 1)];
    let result = section_exact_coplanar_polyline_boolean_extrusion(
        &a,
        &[],
        &a,
        &[],
        Intersection,
        &sweep,
        &normal,
        &q(-2, 1),
        1_000_000,
    )
    .unwrap();
    assert_eq!(
        result.cells_mm,
        [a[..4]
            .iter()
            .map(|point| std::array::from_fn(|i| &point[i] + &sweep[i]))
            .collect::<Vec<_>>()]
    );
    let parallel = [q(0, 1), q(1, 1), q(0, 1)];
    assert!(
        section_exact_coplanar_polyline_boolean_extrusion(
            &a,
            &[],
            &a,
            &[],
            Intersection,
            &sweep,
            &parallel,
            &q(0, 1),
            1_000_000
        )
        .is_err()
    );
}

#[test]
fn exact_parallel_boolean_extrusion_section_preserves_gaps_through_holes() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::Difference, exact_planar_spatial_polyline_area,
        section_parallel_exact_coplanar_polyline_boolean_extrusion,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y, z| [q(x), q(y), q(z)];
    let rect = |lo, hi| {
        [
            p(lo, lo, 0),
            p(hi, lo, 0),
            p(hi, hi, 0),
            p(lo, hi, 0),
            p(lo, lo, 0),
        ]
    };
    let a = rect(0, 6);
    let b = rect(2, 4);
    let section = |budget| {
        section_parallel_exact_coplanar_polyline_boolean_extrusion(
            &a,
            &[],
            &b,
            &[],
            Difference,
            &p(1, 0, 2),
            &p(0, 1, 0),
            &q(3),
            budget,
        )
    };
    let report = section(1_000_000).unwrap();
    assert_eq!(
        report.cells_mm,
        [
            vec![p(1, 3, 2), p(3, 3, 2), p(2, 3, 0), p(0, 3, 0)],
            vec![p(5, 3, 2), p(7, 3, 2), p(6, 3, 0), p(4, 3, 0)],
        ]
    );
    for mut ring in report.cells_mm.clone() {
        ring.push(ring[0].clone());
        assert_eq!(
            exact_planar_spatial_polyline_area(&ring, 1_000_000)
                .unwrap()
                .area_vector_mm2,
            p(0, 4, 0)
        );
    }
    assert_eq!(section(report.exact_work).unwrap(), report);
    assert!(section(report.exact_work - 1).is_err());
}

#[test]
fn exact_parallel_boolean_extrusion_section_keeps_coincident_walls_but_not_edge_only_contacts() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::Intersection,
        section_parallel_exact_coplanar_polyline_boolean_extrusion,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y, z| [q(x), q(y), q(z)];
    let a = [p(0, 0, 0), p(3, 0, 0), p(0, 3, 0), p(0, 0, 0)];
    let section = |normal, offset| {
        section_parallel_exact_coplanar_polyline_boolean_extrusion(
            &a,
            &[],
            &a,
            &[],
            Intersection,
            &p(0, 0, 2),
            &normal,
            &offset,
            1_000_000,
        )
    };
    assert_eq!(section(p(0, 1, 0), q(0)).unwrap().cells_mm.len(), 1);
    assert!(section(p(0, 1, 0), q(3)).unwrap().cells_mm.is_empty());
    assert!(section(p(0, 1, 0), q(4)).unwrap().cells_mm.is_empty());
    assert!(section(p(0, 1, 1), q(0)).is_err());
}

#[test]
fn exact_boolean_extrusion_volume_split_integrates_an_oblique_wedge_and_coincident_cap() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::Intersection,
        split_volume_of_exact_coplanar_polyline_boolean_extrusion,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y, z| [q(x), q(y), q(z)];
    let a = [p(0, 0, 0), p(2, 0, 0), p(2, 2, 0), p(0, 2, 0), p(0, 0, 0)];
    let volume = |normal, offset, budget| {
        split_volume_of_exact_coplanar_polyline_boolean_extrusion(
            &a,
            &[],
            &a,
            &[],
            Intersection,
            &p(0, 0, 2),
            &normal,
            &offset,
            budget,
        )
    };
    let report = volume(p(1, 0, 1), q(1), 1_000_000).unwrap();
    assert_eq!(
        (&report.below_mm3, &report.above_mm3, &report.total_mm3),
        (&q(1), &q(7), &q(8))
    );
    assert_eq!(volume(p(1, 0, 1), q(1), report.exact_work).unwrap(), report);
    assert!(volume(p(1, 0, 1), q(1), report.exact_work - 1).is_err());
    let reversed = volume(p(-1, 0, -1), q(-1), 1_000_000).unwrap();
    assert_eq!((reversed.below_mm3, reversed.above_mm3), (q(7), q(1)));
    let cap = volume(p(0, 0, 1), q(2), 1_000_000).unwrap();
    assert_eq!((cap.below_mm3, cap.above_mm3), (q(8), q(0)));
}

#[test]
fn exact_boolean_extrusion_volume_split_cuts_holes_with_a_plane_parallel_to_sweep() {
    use brep_core::analysis::{
        CoplanarPolylineBoolean::Difference,
        split_volume_of_exact_coplanar_polyline_boolean_extrusion,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y, z| [q(x), q(y), q(z)];
    let rect = |lo, hi| {
        [
            p(lo, lo, 0),
            p(hi, lo, 0),
            p(hi, hi, 0),
            p(lo, hi, 0),
            p(lo, lo, 0),
        ]
    };
    let a = rect(0, 6);
    let b = rect(2, 4);
    let report = split_volume_of_exact_coplanar_polyline_boolean_extrusion(
        &a,
        &[],
        &b,
        &[],
        Difference,
        &p(0, 1, 2),
        &p(1, 0, 0),
        &q(3),
        1_000_000,
    )
    .unwrap();
    assert_eq!(
        (report.below_mm3, report.above_mm3, report.total_mm3),
        (q(32), q(32), q(64))
    );
}

#[test]
fn exact_parallel_profile_extrusion_overlap_accounts_for_holes_and_opposite_oblique_sweeps() {
    use brep_core::analysis::exact_parallel_planar_profile_extrusion_overlap;
    use num_rational::BigRational as Q;
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    let p = |x, y| [q(x, 1), q(y, 1), q(0, 1)];
    let rect = |lo, hi| [p(lo, lo), p(hi, lo), p(hi, hi), p(lo, hi), p(lo, lo)];
    let a = rect(0, 6);
    let hole = rect(2, 4);
    let b = rect(1, 5).map(|[x, y, z]| [x + q(2, 1), y, z + q(8, 1)]);
    let sweep_a = [q(1, 1), q(0, 1), q(4, 1)];
    let sweep_b = [q(-3, 2), q(0, 1), q(-6, 1)];
    let overlap = |budget| {
        exact_parallel_planar_profile_extrusion_overlap(
            &a,
            &[&hole],
            &sweep_a,
            &b,
            &[],
            &sweep_b,
            budget,
        )
    };
    let report = overlap(1_000_000).unwrap();
    assert_eq!(report.common_sweep_range, Some([q(1, 2), q(1, 1)]));
    assert_eq!(report.volume_mm3, q(24, 1));
    assert_eq!(overlap(report.exact_work).unwrap(), report);
    assert!(overlap(report.exact_work - 1).is_err());
    let reverse = exact_parallel_planar_profile_extrusion_overlap(
        &b,
        &[],
        &sweep_b,
        &a,
        &[&hole],
        &sweep_a,
        1_000_000,
    )
    .unwrap();
    assert_eq!(reverse.volume_mm3, report.volume_mm3);
}

#[test]
fn exact_parallel_profile_extrusion_overlap_distinguishes_contacts_and_disjoint_ranges() {
    use brep_core::analysis::exact_parallel_planar_profile_extrusion_overlap;
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let ring = |z| {
        [
            [q(0), q(0), q(z)],
            [q(2), q(0), q(z)],
            [q(2), q(2), q(z)],
            [q(0), q(2), q(z)],
            [q(0), q(0), q(z)],
        ]
    };
    let a = ring(0);
    let sweep = [q(0), q(0), q(2)];
    for (z, expected) in [(2, Some([q(1), q(1)])), (3, None)] {
        let b = ring(z);
        let report = exact_parallel_planar_profile_extrusion_overlap(
            &a,
            &[],
            &sweep,
            &b,
            &[],
            &sweep,
            1_000_000,
        )
        .unwrap();
        assert_eq!(report.common_sweep_range, expected);
        assert_eq!(report.volume_mm3, q(0));
    }
    assert!(
        exact_parallel_planar_profile_extrusion_overlap(
            &a,
            &[],
            &sweep,
            &a,
            &[],
            &[q(1), q(0), q(2)],
            1_000_000
        )
        .is_err()
    );
}

#[test]
fn exact_parallel_extrusion_overlap_mesh_is_closed_and_placed_in_the_common_slab() {
    use brep_core::analysis::mesh_exact_parallel_planar_profile_extrusion_overlap;
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y, z| [q(x), q(y), q(z)];
    let rect = |lo, hi, z| {
        [
            p(lo, lo, z),
            p(hi, lo, z),
            p(hi, hi, z),
            p(lo, hi, z),
            p(lo, lo, z),
        ]
    };
    let a = rect(0, 6, 0);
    let hole = rect(2, 4, 0);
    let b = rect(1, 5, 3);
    let mesh = |budget| {
        mesh_exact_parallel_planar_profile_extrusion_overlap(
            &a,
            &[&hole],
            &p(0, 0, 4),
            &b,
            &[],
            &p(0, 0, -2),
            budget,
        )
    };
    let report = mesh(1_000_000).unwrap();
    assert_eq!(report.volume_mm3, q(24));
    assert!(
        report
            .vertices_mm
            .iter()
            .all(|point| [q(1), q(3)].contains(&point[2]))
    );
    let mut incidence = std::collections::BTreeMap::<_, Vec<_>>::new();
    let mut six_volume = q(0);
    for &[a, b, c] in &report.triangles {
        for (i, j) in [(a, b), (b, c), (c, a)] {
            incidence
                .entry([i.min(j), i.max(j)])
                .or_default()
                .push(i < j);
        }
        let [a, b, c] = [a, b, c].map(|index| &report.vertices_mm[index]);
        for (i, j, k) in [(0, 1, 2), (1, 2, 0), (2, 0, 1)] {
            six_volume += &a[i] * (&b[j] * &c[k] - &b[k] * &c[j]);
        }
    }
    assert!(
        incidence
            .values()
            .all(|pair| pair.len() == 2 && pair[0] != pair[1])
    );
    assert_eq!(six_volume, q(144));
    assert_eq!(mesh(report.exact_work).unwrap(), report);
    assert!(mesh(report.exact_work - 1).is_err());
}

#[test]
fn exact_parallel_extrusion_overlap_mesh_is_empty_for_cap_contact_or_separate_footprints() {
    use brep_core::analysis::mesh_exact_parallel_planar_profile_extrusion_overlap;
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y, z| [q(x), q(y), q(z)];
    let ring = |x, z| {
        [
            p(x, 0, z),
            p(x + 2, 0, z),
            p(x + 2, 2, z),
            p(x, 2, z),
            p(x, 0, z),
        ]
    };
    let a = ring(0, 0);
    for b in [ring(0, 2), ring(3, 0)] {
        let report = mesh_exact_parallel_planar_profile_extrusion_overlap(
            &a,
            &[],
            &p(0, 0, 2),
            &b,
            &[],
            &p(0, 0, 2),
            1_000_000,
        )
        .unwrap();
        assert!(report.vertices_mm.is_empty() && report.triangles.is_empty());
        assert_eq!(report.volume_mm3, q(0));
    }
}

#[test]
fn exact_parallel_extrusion_overlap_brep_qualifies_holed_result_in_the_shared_slab() {
    use brep_core::analysis::{
        PolyhedralPointLocation, brep_exact_parallel_planar_profile_extrusion_overlap,
        exact_polyhedral_point_location,
    };
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let p = |x, y, z| [q(x), q(y), q(z)];
    let rect = |lo, hi, z| {
        [
            p(lo, lo, z),
            p(hi, lo, z),
            p(hi, hi, z),
            p(lo, hi, z),
            p(lo, lo, z),
        ]
    };
    let a = rect(0, 6, 0);
    let hole = rect(2, 4, 0);
    let b = rect(1, 5, 3);
    let get = |budget| {
        brep_exact_parallel_planar_profile_extrusion_overlap(
            &a,
            &[&hole],
            &p(0, 0, 4),
            &b,
            &[],
            &p(0, 0, -2),
            1e-8,
            section_solid_options().admission,
            budget,
        )
    };
    let report = get(1_000_000).unwrap();
    assert_eq!(report.volume_mm3, q(24));
    let solid = report.solid.as_ref().unwrap();
    assert!(solid.geometry().proven);
    assert_eq!(
        solid
            .model()
            .faces
            .iter()
            .filter(|face| face.holes.len() == 1)
            .count(),
        2
    );
    assert!(
        solid
            .model()
            .vertices
            .iter()
            .all(|vertex| [1.0, 3.0].contains(&vertex.point[2]))
    );
    assert_eq!(
        exact_polyhedral_point_location(solid, &p(3, 3, 2), 1_000_000)
            .unwrap()
            .location,
        PolyhedralPointLocation::Exterior
    );
    assert_eq!(
        get(report.exact_work).unwrap().volume_mm3,
        report.volume_mm3
    );
    assert!(get(report.exact_work - 1).is_err());
}

#[test]
fn exact_parallel_extrusion_overlap_brep_keeps_zero_volume_contact_empty() {
    use brep_core::analysis::brep_exact_parallel_planar_profile_extrusion_overlap;
    use num_rational::BigRational as Q;
    let q = |n: i64| Q::from_integer(n.into());
    let ring = |z| {
        [
            [q(0), q(0), q(z)],
            [q(3), q(0), q(z)],
            [q(3), q(3), q(z)],
            [q(0), q(3), q(z)],
            [q(0), q(0), q(z)],
        ]
    };
    let a = ring(0);
    let b = ring(2);
    let sweep = [q(0), q(0), q(2)];
    let result = brep_exact_parallel_planar_profile_extrusion_overlap(
        &a,
        &[],
        &sweep,
        &b,
        &[],
        &sweep,
        1e-8,
        section_solid_options().admission,
        1_000_000,
    )
    .unwrap();
    assert!(result.solid.is_none());
    assert_eq!(result.volume_mm3, q(0));
}

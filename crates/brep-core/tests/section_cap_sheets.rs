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

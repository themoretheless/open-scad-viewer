use nurbs_core::{
    surface::Surface,
    surface_offset::contact_curve::chain::{
        self, Limits, Span,
        region::{self},
    },
};
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn outer(drive: [f64; 2]) -> region::Region {
    outer_on(drive, 0)
}
fn outer_on(drive: [f64; 2], side: usize) -> region::Region {
    let a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p.swap(1, 2);
        p[0] = 3. * p[0] - 1.;
    }
    let span = Span {
        owner: 0,
        axis: 0,
        drive,
        free: [0.7, 0.9],
        other: [[0.2, 0.8], [0.7, 0.9]],
        reversed: false,
    };
    chain::build(
        [&a, &b],
        [0.8, -0.8],
        &[span],
        1e-7,
        1e-7,
        Limits::default(),
    )
    .unwrap()
    .close_to_boundary(side, 0, region::Limits::default())
    .unwrap()
}
use brep_core::shell_thickness::contact_pair::cap::{self as cap_sheet, regions};
fn sheet(drive: [f64; 2]) -> cap_sheet::Sheet {
    let region = outer(drive).with_nested(&[], Default::default()).unwrap();
    cap_sheet::build(
        region.cap(0.005, 1e-6, Default::default()).unwrap(),
        Default::default(),
    )
    .unwrap()
}

use nurbs_core::curve::Curve;
fn line(a: [f64; 2], b: [f64; 2], clock: [f64; 2]) -> Curve {
    let mut c = Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap();
    c.knots = vec![clock[0], clock[0], clock[1], clock[1]];
    c
}
#[test]
fn actual_cap_sectors_and_uv_material_witnesses_replay_without_deleting_faces() {
    let s = sheet([0.1, 0.9]);
    let before = s.model().clone();
    let ends = [
        regions::vertex_uv(s.model(), 0, 0).unwrap(),
        regions::vertex_uv(s.model(), 0, 2).unwrap(),
    ];
    let path = vec![line(ends[0], ends[1], [-3., 2.])];
    let r = regions::build(s)
        .split_uv(0, [0, 2], &path, Default::default())
        .unwrap();
    assert_eq!(r.model().faces.len(), 2);
    assert_eq!(r.cuts().len(), 1);
    assert_eq!(r.source().model(), &before);
    let c = &r.cuts()[0];
    assert!(c.boundary.complete && c.trim.all_valid);
    assert_eq!(c.edges.len(), 1);
    assert_eq!(r.model().edges[c.edges[0]].curve.domain(), [-3., 2.]);
    let selection = r.select(&[[0.2, 0.2]], 100000).unwrap();
    assert_eq!(selection.retained_faces().len(), 1);
    assert_eq!(selection.discard_faces().len(), 1);
    assert_eq!(r.model().faces.len(), 2);
    assert!(r.model().bodies.is_empty());
    selection.reverify().unwrap();
    assert!(r.select(&[[0.5, 0.4]], 100000).is_err());
    assert!(r.select(&[[0.99, 0.99]], 100000).is_err());
    assert!(r.select(&[[0.2, 0.2]], 1).is_err());
    let both = r.select(&[[0.2, 0.2], [0.8, 0.6]], 100000).unwrap();
    assert_eq!(both.retained_faces().len(), 2);
    assert!(both.discard_faces().is_empty());
}
#[test]
fn segmented_c0_cut_and_descendant_cut_keep_original_carrier_and_face_lineage() {
    let s = sheet([0.1, 0.9]);
    let p = [
        regions::vertex_uv(s.model(), 0, 0).unwrap(),
        regions::vertex_uv(s.model(), 0, 2).unwrap(),
    ];
    let uv = vec![
        line(p[0], [0.35, 0.3], [-3., 2.]),
        line([0.35, 0.3], p[1], [-9., 7.]),
    ];
    let r = regions::build(s)
        .split_uv(0, [0, 2], &uv, Default::default())
        .unwrap();
    let vertex = 4;
    let candidates = (0..r.model().faces.len())
        .filter(|&f| {
            regions::vertex_uv(r.model(), f, vertex).is_ok()
                && regions::vertex_uv(r.model(), f, 3).is_ok()
        })
        .collect::<Vec<_>>();
    assert_eq!(candidates.len(), 1);
    let f = candidates[0];
    let p = [
        regions::vertex_uv(r.model(), f, vertex).unwrap(),
        regions::vertex_uv(r.model(), f, 3).unwrap(),
    ];
    let r = r
        .split_uv(
            f,
            [vertex, 3],
            &[line(p[0], p[1], [4., 9.])],
            Default::default(),
        )
        .unwrap();
    assert_eq!(r.model().faces.len(), 3);
    assert_eq!(r.cuts().len(), 2);
    assert!(r.cuts()[0].identities.contains(&r.cuts()[1].parent));
    assert!(
        r.model()
            .faces
            .iter()
            .all(|f| f.surface == *r.source().cap().surface())
    );
    r.reverify().unwrap();
}
fn hole_sheet(center: [f64; 2]) -> cap_sheet::Sheet {
    let p = [[1., 0.], [1., 1.], [0., 1.]];
    let arcs = [
        p,
        [[0., 1.], [-1., 1.], [-1., 0.]],
        [[-1., 0.], [-1., -1.], [0., -1.]],
        [[0., -1.], [1., -1.], [1., 0.]],
    ];
    let hole = arcs
        .map(|p| Curve {
            degree: 2,
            knots: vec![-3., -3., -3., 2., 2., 2.],
            control_points: p
                .map(|p| vec![center[0] + 0.025 * p[0], center[1] + 0.025 * p[1]])
                .to_vec(),
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        })
        .to_vec();
    let r = outer([0.1, 0.9])
        .with_nested(&[hole], Default::default())
        .unwrap();
    cap_sheet::build(
        r.cap(0.005, 1e-6, Default::default()).unwrap(),
        Default::default(),
    )
    .unwrap()
}
#[test]
fn rational_hole_is_assigned_to_one_actual_sector_but_crossed_hole_refuses() {
    let s = hole_sheet([0.25, 0.2]);
    let raw = s.cap().region().uv_domain().loops().to_vec();
    let p = [
        regions::vertex_uv(s.model(), 0, 0).unwrap(),
        regions::vertex_uv(s.model(), 0, 2).unwrap(),
    ];
    let r = regions::build(s)
        .split_uv(0, [0, 2], &[line(p[0], p[1], [0., 1.])], Default::default())
        .unwrap();
    assert_eq!(
        r.model().faces.iter().map(|f| f.holes.len()).sum::<usize>(),
        1
    );
    assert_eq!(r.source().cap().region().uv_domain().loops(), raw);
    r.reverify().unwrap();
    let s = hole_sheet([0.5, 0.4]);
    let p = [
        regions::vertex_uv(s.model(), 0, 0).unwrap(),
        regions::vertex_uv(s.model(), 0, 2).unwrap(),
    ];
    assert!(
        regions::build(s)
            .split_uv(0, [0, 2], &[line(p[0], p[1], [0., 1.])], Default::default())
            .is_err()
    );
}
#[test]
fn malformed_uv_paths_and_stage_exhaustion_publish_no_partial_regions() {
    for kind in 0..5 {
        let s = sheet([0.1, 0.9]);
        let p = [
            regions::vertex_uv(s.model(), 0, 0).unwrap(),
            regions::vertex_uv(s.model(), 0, 2).unwrap(),
        ];
        let mut l = regions::Limits::default();
        match kind {
            0 => l.fitting.evaluations = 1,
            1 => l.cut_cells = 1,
            2 => l.boundary_cells = 1,
            3 => l.trim_pairs = 1,
            _ => l.fitting.proof_cells = 0,
        }
        assert!(
            regions::build(s)
                .split_uv(0, [0, 2], &[line(p[0], p[1], [0., 1.])], l)
                .is_err(),
            "kind={kind}"
        );
    }
    let s = sheet([0.1, 0.9]);
    let p = [
        regions::vertex_uv(s.model(), 0, 0).unwrap(),
        regions::vertex_uv(s.model(), 0, 2).unwrap(),
    ];
    let mut c = line(p[0], p[1], [0., 1.]);
    c.control_points[0][0] += 0.01;
    assert!(
        regions::build(s)
            .split_uv(0, [0, 2], &[c], Default::default())
            .is_err()
    );
}

#[test]
fn malformed_public_model_lookup_and_late_uv_input_refuse_before_fitting() {
    let s = sheet([0.1, 0.9]);
    let mut invalid = s.model().clone();
    invalid.faces[0].outer = usize::MAX;
    assert!(regions::vertex_uv(&invalid, 0, 0).is_err());
    let p = [
        regions::vertex_uv(s.model(), 0, 0).unwrap(),
        regions::vertex_uv(s.model(), 0, 2).unwrap(),
    ];
    let a = line(p[0], [0.35, 0.3], [0., 1.]);
    let mut b = line([0.35, 0.3], p[1], [0., 1.]);
    b.weights[0] = f64::NAN;
    let l = regions::Limits {
        fitting: nurbs_core::surface_offset::contact_curve::lift::FitLimits {
            evaluations: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    let error = regions::build(s)
        .split_uv(0, [0, 2], &[a, b], l)
        .err()
        .unwrap();
    assert!(
        error.to_string().contains("weight") || error.to_string().contains("finite"),
        "{error}"
    );
}

#[test]
fn exact_linear_trim_proof_can_succeed_with_one_interval_cell() {
    let s = sheet([0.1, 0.9]);
    let p = [
        regions::vertex_uv(s.model(), 0, 0).unwrap(),
        regions::vertex_uv(s.model(), 0, 2).unwrap(),
    ];
    let r = regions::build(s)
        .split_uv(
            0,
            [0, 2],
            &[line(p[0], p[1], [0., 1.])],
            regions::Limits {
                trim_cells: 1,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(r.cuts()[0].trim.all_valid && r.cuts()[0].trim.cells <= 1);
    r.reverify().unwrap();
}
#[test]
fn partitioned_cap_sectors_sew_with_original_internal_edge_incidence() {
    let mut parts = vec![];
    for drive in [[0.1, 0.5], [0.5, 0.9]] {
        let s = sheet(drive);
        let p = [
            regions::vertex_uv(s.model(), 0, 0).unwrap(),
            regions::vertex_uv(s.model(), 0, 2).unwrap(),
        ];
        parts.push(
            regions::build(s)
                .split_uv(0, [0, 2], &[line(p[0], p[1], [0., 1.])], Default::default())
                .unwrap(),
        );
    }
    let r = regions::sewing::build(parts, Default::default()).unwrap();
    assert_eq!(r.joins().len(), 1);
    assert_eq!(r.model().faces.len(), 4);
    assert_eq!(r.model().edges.len(), 9);
    assert_eq!(r.model().vertices.len(), 6);
    assert_eq!(r.model().shells.len(), 1);
    assert!(r.model().bodies.is_empty());
    assert!(r.boundary_proof().complete);
    assert!(r.regions().iter().all(|r| r.cuts().len() == 1));
    r.reverify().unwrap();
}

fn sectors(drive: [f64; 2]) -> regions::Regions {
    let s = sheet(drive);
    let p = [
        regions::vertex_uv(s.model(), 0, 0).unwrap(),
        regions::vertex_uv(s.model(), 0, 2).unwrap(),
    ];
    regions::build(s)
        .split_uv(0, [0, 2], &[line(p[0], p[1], [0., 1.])], Default::default())
        .unwrap()
}
#[test]
fn internal_sector_mates_and_incomplete_joint_boundaries_refuse_sewing() {
    let error = regions::sewing::build(
        vec![sectors([0.1, 0.9]), sectors([0.1, 0.9])],
        Default::default(),
    )
    .err()
    .unwrap();
    assert!(error.to_string().contains("internal sector"), "{error}");
    let limits = cap_sheet::sewing::Limits {
        boundary_cells: 1,
        ..Default::default()
    };
    assert!(
        regions::sewing::build(vec![sectors([0.1, 0.5]), sectors([0.5, 0.9])], limits).is_err()
    );
}
#[test]
fn world_cut_projection_preserves_authored_curve_and_cannot_waive_interior_support_error() {
    let s = sheet([0.1, 0.9]);
    let p = [s.model().vertices[0].point, s.model().vertices[2].point];
    let mut c = Curve::from_polyline(p.map(|p| p.to_vec()).to_vec()).unwrap();
    c.knots = vec![-5., -5., 3., 3.];
    let r = regions::build(s)
        .split_world(
            0,
            [0, 2],
            &[c.clone()],
            regions::Limits {
                exact_work: 0,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(r.model().faces.len(), 2);
    assert_eq!(r.model().edges[r.cuts()[0].edges[0]].curve, c);
    assert_eq!(r.cuts()[0].exact_work, 0);
    assert!(r.cuts()[0].boundary.complete);
    r.reverify().unwrap();
    let s = sheet([0.1, 0.9]);
    let p = [s.model().vertices[0].point, s.model().vertices[2].point];
    let middle = (0..3)
        .map(|k| (p[0][k] + p[1][k]) * 0.5 + if k == 2 { 0.01 } else { 0. })
        .collect();
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![p[0].to_vec(), middle, p[1].to_vec()],
        weights: vec![1.; 3],
        periodic: false,
    };
    assert!(
        regions::build(s)
            .split_world(0, [0, 2], &[c], Default::default())
            .is_err()
    );
}

fn curved_outer() -> region::Region {
    let mut a = plane();
    a.degree_u = 2;
    a.knots_u = vec![0., 0., 0., 1., 1., 1.];
    a.control_points = [[1., 0.], [1., 1.], [0., 1.]]
        .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
        .to_vec();
    a.weights = [1., std::f64::consts::FRAC_1_SQRT_2, 1.]
        .map(|w| vec![w; 2])
        .to_vec();
    let mut b = plane();
    b.control_points = vec![
        vec![vec![0., 1., 0.], vec![0., 1., 1.]],
        vec![vec![0., 2., 0.], vec![0.5, 2., 1.]],
    ];
    let span = Span {
        owner: 0,
        axis: 1,
        drive: [0.35, 0.55],
        free: [0.88, 0.96],
        other: [[0.07, 0.22], [0.3, 0.6]],
        reversed: false,
    };
    chain::build([&a, &b], [0.1; 2], &[span], 1e-7, 1e-7, Limits::default())
        .unwrap()
        .close_to_boundary(0, 3, region::Limits::default())
        .unwrap()
}

#[test]
fn nonplanar_rational_offset_carrier_is_cut_into_actual_sectors() {
    let r = curved_outer().with_nested(&[], Default::default()).unwrap();
    let s = cap_sheet::build(
        r.cap(0.005, 1e-6, Default::default()).unwrap(),
        Default::default(),
    )
    .unwrap();
    let surface = s.cap().surface().clone();
    let p = [
        regions::vertex_uv(s.model(), 0, 0).unwrap(),
        regions::vertex_uv(s.model(), 0, 2).unwrap(),
    ];
    let r = regions::build(s)
        .split_uv(
            0,
            [0, 2],
            &[line(p[0], p[1], [-3., 2.])],
            Default::default(),
        )
        .unwrap();
    assert_eq!(r.model().faces.len(), 2);
    assert!(r.model().faces.iter().all(|f| f.surface == surface));
    assert!(r.cuts()[0].fitting[0].curve.as_ref().unwrap().degree > 1);
    assert!(r.cuts()[0].boundary.complete && r.cuts()[0].trim.all_valid);
    let selection = r.select(&[[0.2, 0.38], [0.2, 0.52]], 100000).unwrap();
    assert_eq!(selection.retained_faces().len(), 2);
    selection.reverify().unwrap();
}

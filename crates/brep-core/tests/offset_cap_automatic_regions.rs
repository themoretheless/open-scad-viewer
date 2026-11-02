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

use brep_core::shell_thickness::contact_pair::cap::{self as cap_sheet, regions};
use nurbs_core::curve::Curve;
use regions::automatic::{Limits as AutoLimits, Method};
fn sheet(cutter: bool, hole: bool) -> cap_sheet::Sheet {
    sheet_side(cutter, hole, 0)
}
fn sheet_side(cutter: bool, hole: bool, side: usize) -> cap_sheet::Sheet {
    let mut a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p.swap(1, 2);
        p[0] = 3. * p[0] - 1.;
        if cutter {
            p[1] = 0.4;
        }
    }
    if cutter {
        for s in [&mut a, &mut b] {
            for p in s.control_points.iter_mut().flatten() {
                let old = p.clone();
                p[0] = old[2] - 0.3;
                p[1] = 2. * old[0] - 0.5;
                p[2] = 2. * old[1] - 0.5;
            }
        }
    }
    let span = Span {
        owner: 0,
        axis: 0,
        drive: [0.1, 0.9],
        free: [0.7, 0.9],
        other: [[0.2, 0.8], [0.7, 0.9]],
        reversed: false,
    };
    let r = chain::build(
        [&a, &b],
        [0.8, -0.8],
        &[span],
        1e-7,
        1e-7,
        Limits::default(),
    )
    .unwrap()
    .close_to_boundary(0, side, region::Limits::default())
    .unwrap();
    let holes = if hole {
        let p = if cutter {
            [[0.4, 0.6], [0.5, 0.6], [0.5, 0.7], [0.4, 0.7]]
        } else {
            [[0.2, 0.2], [0.3, 0.2], [0.3, 0.3], [0.2, 0.3]]
        };
        vec![
            (0..4)
                .map(|i| {
                    Curve::from_polyline(vec![p[i].to_vec(), p[(i + 1) % 4].to_vec()]).unwrap()
                })
                .collect(),
        ]
    } else {
        vec![]
    };
    let c = r
        .with_nested(&holes, Default::default())
        .unwrap()
        .cap(0.005, 1e-6, Default::default())
        .unwrap();
    cap_sheet::build(c, Default::default()).unwrap()
}
#[test]
fn actual_face_intersection_generates_owned_endpoints_and_trace_without_geometry_input() {
    for method in [Method::Fiber, Method::Ruled] {
        let a = regions::build(sheet(false, false));
        let b = regions::build(sheet(true, false));
        let source = a.model().clone();
        let cutter = b.model().clone();
        let r = a.intersect(b, 0, 0, method, AutoLimits::default()).unwrap();
        assert_eq!(r.model().faces.len(), 2);
        assert_eq!(r.model().vertices.len(), 6);
        assert_eq!(r.cuts().len(), 1);
        let e = r.cuts()[0].intersection.as_ref().unwrap();
        assert_eq!(e.method, method);
        assert_eq!(e.original_face, source.1.faces[0]);
        assert!(
            e.boundary_coverage
                .iter()
                .all(|(_, r)| r.complete && r.source_is_owned)
        );
        assert_eq!(
            e.boundary_coverage
                .iter()
                .map(|(_, r)| r.eligible.len())
                .sum::<usize>(),
            2
        );
        assert_eq!(r.source().model(), &source);
        assert_eq!(r.cuts()[0].cutter().unwrap().model(), &cutter);
        assert!(r.cuts()[0].boundary.complete && r.cuts()[0].trim.all_valid);
        assert!(r.model().bodies.is_empty());
        let selection = r.select(&[[0.25, 0.4], [0.75, 0.4]], 100000).unwrap();
        assert_eq!(selection.retained_faces().len(), 2);
        selection.reverify().unwrap();
    }
}
#[test]
fn source_hole_is_preserved_and_assigned_to_one_automatic_sector() {
    let a = regions::build(sheet(false, true));
    let b = regions::build(sheet(true, false));
    let r = a
        .intersect(b, 0, 0, Method::Fiber, AutoLimits::default())
        .unwrap();
    assert_eq!(
        r.model().faces.iter().map(|f| f.holes.len()).sum::<usize>(),
        1
    );
    r.reverify().unwrap();
}
#[test]
fn actual_cutter_hole_cannot_be_ignored_between_valid_boundary_roots() {
    for method in [Method::Fiber, Method::Ruled] {
        let a = regions::build(sheet(false, false));
        let b = regions::build(sheet(true, true));
        let mut roots = 0;
        for c in &a.model().loops[a.model().faces[0].outer].coedges {
            let proof =
                brep_core::face_crossings::inspect(a.model(), c.edge, b.model(), 0, 1e-7, 100000)
                    .unwrap();
            assert!(proof.complete);
            roots += proof.eligible.len();
        }
        assert_eq!(
            roots, 2,
            "Both endpoint roots must be retained before testing interior cutter-hole refusal"
        );
        assert!(a.intersect(b, 0, 0, method, AutoLimits::default()).is_err());
    }
}
#[test]
fn original_construction_and_final_proof_budgets_publish_no_partial_plan() {
    for kind in 0..4 {
        let a = regions::build(sheet(false, false));
        let b = regions::build(sheet(true, false));
        let mut l = AutoLimits::default();
        match kind {
            0 => l.construction_cells = 32,
            1 => l.boundary_cells = 1,
            2 => l.trim_pairs = 1,
            _ => l.trim_domain_cells = 0,
        }
        assert!(
            a.intersect(b, 0, 0, Method::Fiber, l).is_err(),
            "kind={kind}"
        );
    }
}
#[test]
fn automatic_cut_replays_a_cutter_with_its_own_prior_c0_sectors() {
    let b = regions::build(sheet(true, false));
    let p = [
        regions::vertex_uv(b.model(), 0, 2).unwrap(),
        regions::vertex_uv(b.model(), 0, 3).unwrap(),
    ];
    let uv = [
        Curve::from_polyline(vec![p[0].to_vec(), vec![0.5, 0.1]]).unwrap(),
        Curve::from_polyline(vec![vec![0.5, 0.1], p[1].to_vec()]).unwrap(),
    ];
    let b = b.split_uv(0, [2, 3], &uv, Default::default()).unwrap();
    let face = b.select(&[[0.45, 0.65]], 100000).unwrap().seed_faces()[0];
    let a = regions::build(sheet(false, false));
    let r = a
        .intersect(b, 0, face, Method::Fiber, AutoLimits::default())
        .unwrap();
    assert_eq!(r.cuts()[0].cutter().unwrap().cuts().len(), 1);
    r.reverify().unwrap();
}

#[test]
fn independently_generated_automatic_sectors_sew_across_the_original_shared_root() {
    let mut sources = vec![];
    for side in [0, 2] {
        let a = regions::build(sheet_side(false, false, side));
        let b = regions::build(sheet(true, false));
        sources.push(
            a.intersect(b, 0, 0, Method::Fiber, AutoLimits::default())
                .unwrap(),
        );
    }
    let r = regions::sewing::build(sources, Default::default()).unwrap();
    assert_eq!(r.joins().len(), 2);
    assert_eq!(r.model().faces.len(), 4);
    assert_eq!(r.model().edges.len(), 12);
    assert_eq!(r.model().vertices.len(), 9);
    assert_eq!(r.model().shells.len(), 1);
    assert!(r.model().bodies.is_empty());
    assert!(r.boundary_proof().complete);
    r.reverify().unwrap();
}
#[test]
fn invalid_face_requests_refuse_without_indexing_unowned_geometry() {
    for (a, b) in [(usize::MAX, 0), (0, usize::MAX)] {
        let source = regions::build(sheet(false, false));
        let cutter = regions::build(sheet(true, false));
        assert!(
            source
                .intersect(cutter, a, b, Method::Fiber, AutoLimits::default())
                .is_err()
        );
    }
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

fn horizontal_cutter() -> regions::Regions {
    let mut a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p.swap(1, 2);
        p[0] = 3. * p[0] - 1.;
        p[1] = 0.48;
    }
    for t in [&mut a, &mut b] {
        for p in t.control_points.iter_mut().flatten() {
            p[0] = 2.5 * p[0] - 0.75;
            p[1] = 2.5 * p[1] - 0.5;
            p[2] -= 0.35;
        }
    }
    let span = Span {
        owner: 0,
        axis: 0,
        drive: [0.1, 0.9],
        free: [0.7, 0.9],
        other: [[0.2, 0.8], [0.7, 0.9]],
        reversed: false,
    };
    let r = chain::build(
        [&a, &b],
        [0.8, -0.8],
        &[span],
        1e-7,
        1e-7,
        Limits::default(),
    )
    .unwrap()
    .close_to_boundary(0, 0, region::Limits::default())
    .unwrap()
    .with_nested(&[], Default::default())
    .unwrap();
    regions::build(
        cap_sheet::build(
            r.cap(0.005, 1e-6, Default::default()).unwrap(),
            Default::default(),
        )
        .unwrap(),
    )
}
#[test]
fn rational_cylindrical_actual_cap_intersects_automatically_without_authored_trace() {
    for method in [Method::Fiber, Method::Ruled] {
        let r = curved_outer().with_nested(&[], Default::default()).unwrap();
        let s = cap_sheet::build(
            r.cap(0.005, 1e-6, Default::default()).unwrap(),
            Default::default(),
        )
        .unwrap();
        let surface = s.cap().surface().clone();
        let r = regions::build(s)
            .intersect(horizontal_cutter(), 0, 0, method, AutoLimits::default())
            .unwrap();
        assert_eq!(r.model().faces.len(), 2);
        assert!(r.model().faces.iter().all(|f| f.surface == surface));
        let c = &r.cuts()[0];
        assert!(c.edges.iter().any(|&e| r.model().edges[e].curve.degree > 1));
        assert!(c.boundary.complete && c.trim.all_valid);
        let s = r.select(&[[0.2, 0.38], [0.2, 0.52]], 100000).unwrap();
        assert_eq!(s.retained_faces().len(), 2);
        s.reverify().unwrap();
    }
}

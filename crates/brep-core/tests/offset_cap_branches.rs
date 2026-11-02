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
use regions::automatic::{Limits as AutoLimits, Method};

use nurbs_core::surface_offset::contact_curve::chain::region::compound::cap as carrier;
fn source() -> regions::Regions {
    let mut a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p.swap(1, 2);
        p[0] = 3. * p[0] - 1.;
        p[1] = -2.4;
    }
    for s in [&mut a, &mut b] {
        for p in s.control_points.iter_mut().flatten() {
            p[1] = 0.25 * p[1] + 0.3;
            p[2] += 0.00002;
        }
    }
    make(a, b, None)
}
fn cutter() -> regions::Regions {
    let a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p.swap(1, 2);
        p[0] = 3. * p[0] - 1.;
    }
    let mut w = plane();
    w.degree_u = 2;
    w.degree_v = 2;
    w.knots_u = vec![0., 0., 0., 1., 1., 1.];
    w.knots_v = w.knots_u.clone();
    let u = [0., 0.5, 1.];
    let p = [0.09, -0.41, 0.09];
    let q = [0., -0.4, 0.2];
    w.control_points = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| vec![u[i], u[j], 0.8 + 0.001 * p[i] * q[j]])
                .collect()
        })
        .collect();
    w.weights = vec![vec![1.; 3]; 3];
    make(a, b, Some(w))
}
fn make(a: Surface, b: Surface, actual: Option<Surface>) -> regions::Regions {
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
    let c = if let Some(s) = actual {
        carrier::with_candidate(r, &s, 0.005, 1e-6, Default::default()).unwrap()
    } else {
        r.cap(0.005, 1e-6, Default::default()).unwrap()
    };
    regions::build(cap_sheet::build(c, Default::default()).unwrap())
}
#[test]
fn four_owned_roots_generate_two_verified_actual_branches_and_three_sectors() {
    let a = source();
    let b = cutter();
    let before = (a.model().clone(), b.model().clone());
    let r = a
        .intersect(b, 0, 0, Method::Branches, AutoLimits::default())
        .unwrap();
    let e = r.cuts()[0].intersection.as_ref().unwrap();
    assert_eq!(e.branches.len(), 2);
    assert_eq!(r.model().faces.len(), 3);
    assert_eq!(r.cuts()[0].faces.len(), 3);
    assert_eq!(
        e.boundary_coverage
            .iter()
            .map(|(_, r)| r.eligible.len())
            .sum::<usize>(),
        4
    );
    assert!(e.branches.iter().all(|b| b.root_fibers > 0));
    for branch in &e.branches {
        for &edge in &branch.edges {
            let curve = &r.model().edges[edge].curve;
            let [a, b] = curve.domain();
            for i in 0..=32 {
                let p = curve.evaluate(a + (b - a) * i as f64 / 32.).unwrap().point;
                assert!((p[2] - 0.80002).abs() <= r.model().tolerance_mm);
                let graph = 0.8 + 0.001 * (p[0] - 0.1) * (p[0] - 0.9) * p[1] * (p[1] - 0.8);
                assert!((p[2] - graph).abs() <= r.model().tolerance_mm);
            }
        }
    }
    assert!(e.rejected_pairs > 0);
    assert_eq!(e.refusals.len(), e.rejected_pairs);
    assert_eq!(r.source().model(), &before.0);
    assert_eq!(r.cuts()[0].cutter().unwrap().model(), &before.1);
    assert!(r.cuts()[0].boundary.complete && r.cuts()[0].trim.all_valid);
    assert!(r.model().bodies.is_empty());
    let s = r
        .select(&[[0.2, 0.37], [0.5, 0.37], [0.8, 0.37]], 100000)
        .unwrap();
    assert_eq!(s.retained_faces().len(), 3);
    s.reverify().unwrap();
}
#[test]
fn single_branch_method_does_not_silently_drop_other_owned_roots() {
    let error = source()
        .intersect(cutter(), 0, 0, Method::Fiber, AutoLimits::default())
        .err()
        .unwrap();
    assert!(error.to_string().contains("two roots"), "{error}");
}
#[test]
fn multibranch_work_and_postproof_exhaustion_publish_no_partial_regions() {
    for kind in 0..3 {
        let mut l = AutoLimits::default();
        match kind {
            0 => l.construction_cells = 32,
            1 => l.boundary_cells = 1,
            _ => l.trim_pairs = 1,
        }
        assert!(
            source()
                .intersect(cutter(), 0, 0, Method::Branches, l)
                .is_err(),
            "kind={kind}"
        );
    }
}

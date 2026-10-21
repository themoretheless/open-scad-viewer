use nurbs_core::{
    parallel_sections::{self, Status, Stop},
    polynomial,
    surface::Surface,
};
fn graph(swapped: bool) -> Surface {
    let mut c = vec![vec![[0.; 3]; 3]; 3];
    c[1][0][0] = 1.;
    c[0][1][1] = 1.;
    if swapped {
        c[1][0][2] = 1.;
        c[0][2][2] = 1.;
    } else {
        c[0][1][2] = 1.;
        c[2][0][2] = 1.;
    }
    let d = if swapped {
        [-1., 2., -1., 1.]
    } else {
        [-1., 1., -1., 2.]
    };
    let mut surface = polynomial::parametric_surface(d, &c).unwrap();
    // The polynomial constructor normalizes its output parameters. Restore
    // these exact authored domains so x=u, y=v in the independent oracle.
    for k in &mut surface.knots_u {
        *k = d[0] + (d[1] - d[0]) * *k;
    }
    for k in &mut surface.knots_v {
        *k = d[2] + (d[3] - d[2]) * *k;
    }
    surface.validate().unwrap();
    surface
}
#[test]
fn parallel_sections_enclose_entire_original_graph_and_prove_empty_levels() {
    let s = graph(false);
    let before = s.clone();
    let report =
        parallel_sections::intersect(&s, [0., 0., 1.], &[0.4, 0.8, 4.], 0.05, 100000).unwrap();
    assert!(report.complete, "{report:?}");
    assert!(report.work <= 100000);
    for section in &report.sections[..2] {
        assert_eq!(section.status, Status::Approximate);
        assert_eq!(section.solved_axis, Some(1));
        assert!(section.error_upper.unwrap() <= 0.05);
        assert!(section.unresolved_domain.is_none());
        let mut start = -1.;
        for curve in &section.curves {
            assert_eq!(curve.domain()[0], start);
            start = curve.domain()[1];
            for i in 0..=50 {
                let d = curve.domain();
                let u = d[0] + (d[1] - d[0]) * i as f64 / 50.;
                let p = curve.evaluate(u).unwrap().point;
                let expected = [u, section.level - u * u, section.level];
                let error = (0..3)
                    .map(|k| (p[k] - expected[k]).powi(2))
                    .sum::<f64>()
                    .sqrt();
                assert!(error <= section.error_upper.unwrap() + 1e-11);
            }
        }
        assert_eq!(start, 1.);
    }
    assert_eq!(report.sections[2].status, Status::Empty);
    assert!(report.sections[2].curves.is_empty());
    assert_eq!(s, before);
}
#[test]
fn axis_swap_and_nonunit_normal_preserve_plane_equation() {
    let s = graph(true);
    let report = parallel_sections::intersect(&s, [0., 0., -2.], &[-0.8], 0.05, 100000).unwrap();
    let section = &report.sections[0];
    assert_eq!(section.status, Status::Approximate, "{report:?}");
    assert_eq!(section.solved_axis, Some(0));
    for c in &section.curves {
        for i in 0..=30 {
            let d = c.domain();
            let v = d[0] + (d[1] - d[0]) * i as f64 / 30.;
            let p = c.evaluate(v).unwrap().point;
            let expected = [0.4 - v * v, v, 0.4];
            let error = (0..3)
                .map(|k| (p[k] - expected[k]).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(error <= section.error_upper.unwrap() + 1e-11);
        }
    }
}
#[test]
fn budget_tangencies_and_coincidence_never_become_missing_sections() {
    let s = graph(false);
    let low = parallel_sections::intersect(&s, [0., 0., 1.], &[0.4, 4., 0.8], 0.01, 2).unwrap();
    assert!(!low.complete);
    assert_eq!(low.work, 2);
    assert_eq!(low.sections[0].status, Status::Unresolved);
    assert_eq!(low.sections[0].stop, Stop::WorkLimit);
    assert_eq!(
        low.sections[0].unresolved_domain,
        Some([[-1., 1.], [-1., 2.]])
    );
    assert_eq!(low.sections[1].status, Status::Empty);
    assert_eq!(low.sections[2].status, Status::Unresolved);
    let mut c = vec![vec![[0.; 3]; 3]; 3];
    c[1][0][0] = 1.;
    c[0][1][1] = 1.;
    c[2][0][2] = 1.;
    c[0][2][2] = 1.;
    let bowl = polynomial::parametric_surface([-1., 1., -1., 1.], &c).unwrap();
    let tangent = parallel_sections::intersect(&bowl, [0., 0., 1.], &[0.], 0.01, 1000).unwrap();
    assert_eq!(tangent.sections[0].status, Status::Unresolved);
    c[2][0][2] = 0.;
    c[0][2][2] = 0.;
    let plane = polynomial::parametric_surface([-1., 1., -1., 1.], &c).unwrap();
    let coincident = parallel_sections::intersect(&plane, [0., 0., 1.], &[0.], 0.01, 1000).unwrap();
    assert_eq!(coincident.sections[0].status, Status::Unresolved);
}
#[test]
fn invalid_section_inputs_are_refused() {
    let s = graph(false);
    for (normal, levels, tolerance, budget) in [
        ([0., 0., 0.], vec![0.], 0.1, 100),
        ([0., 0., 1.], vec![], 0.1, 100),
        ([0., 0., 1.], vec![f64::NAN], 0.1, 100),
        ([0., 0., 1.], vec![0.], 0., 100),
        ([0., 0., 1.], vec![0.], 0.1, 0),
    ] {
        assert!(parallel_sections::intersect(&s, normal, &levels, tolerance, budget).is_err());
    }
}

#[test]
fn rational_source_is_verified_without_replacing_it_by_a_sampled_proxy() {
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 1.]],
            vec![vec![0.5, 0., 0.], vec![0.5, 1., 1.]],
            vec![vec![1., 0., 0.], vec![1., 1., 1.]],
        ],
        weights: vec![vec![1.; 2], vec![1.2; 2], vec![1.; 2]],
        periodic_u: false,
        periodic_v: false,
    };
    let before = s.clone();
    let report = parallel_sections::intersect(&s, [0., 0., 1.], &[0.4], 0.05, 100000).unwrap();
    let section = &report.sections[0];
    assert_eq!(section.status, Status::Approximate, "{report:?}");
    for curve in &section.curves {
        for i in 0..=40 {
            let d = curve.domain();
            let u = d[0] + (d[1] - d[0]) * i as f64 / 40.;
            let w = (1. - u) * (1. - u) + 2.4 * u * (1. - u) + u * u;
            let x = (1.2 * u * (1. - u) + u * u) / w;
            let p = curve.evaluate(u).unwrap().point;
            let e = (p[0] - x).hypot(p[1] - 0.4).hypot(p[2] - 0.4);
            assert!(e <= section.error_upper.unwrap() + 1e-11);
        }
    }
    assert_eq!(s, before);
}

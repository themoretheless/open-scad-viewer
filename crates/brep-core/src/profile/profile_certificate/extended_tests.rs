use super::*;

#[test]
fn one_stored_multispan_curve_is_certified_without_rounding_or_replacement() {
    let curve = Curve::from_polyline(vec![
        vec![0., 0.],
        vec![8., 0.],
        vec![8., 8.],
        vec![0., 8.],
        vec![0., 0.],
    ])
    .unwrap();
    let loops = vec![vec![curve]];
    let before = loops.clone();
    let c = certify(&loops, 1e-7).unwrap();
    assert_eq!(verify(&loops, 1e-7, &c).unwrap().pieces(), 4);
    assert_eq!(loops, before);
    let mut altered = loops.clone();
    altered[0][0].knots[2] = 0.5;
    assert!(verify(&altered, 1e-7, &c).is_err());
}

#[test]
fn periodic_cubic_active_domain_has_an_exact_closed_certificate() {
    let curve = Curve {
        degree: 3,
        knots: (0..=10).map(f64::from).collect(),
        control_points: vec![
            vec![0., 0.],
            vec![8., 0.],
            vec![8., 8.],
            vec![0., 8.],
            vec![0., 0.],
            vec![8., 0.],
            vec![8., 8.],
        ],
        weights: vec![1.; 7],
        periodic: true,
    };
    let loops = vec![vec![curve]];
    let before = loops.clone();
    let c = certify(&loops, 1e-7).unwrap();
    verify(&loops, 1e-7, &c).unwrap();
    assert_eq!(loops, before);
    let m = certify_measure(&loops, 1e-7, 1e-9).unwrap();
    verify_measure(&loops, 1e-7, &m).unwrap();
}

#[test]
fn higher_degree_rotated_boundaries_use_extended_separator_codes() {
    for degree in [4, 8] {
        let vertices = [[0., 0.], [8., 8.], [0., 16.], [-8., 8.]];
        let wire = (0..4)
            .map(|i| {
                let (a, b) = (vertices[i], vertices[(i + 1) % 4]);
                Curve {
                    degree,
                    knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
                    control_points: (0..=degree)
                        .map(|j| {
                            let t = j as f64 / degree as f64;
                            vec![a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
                        })
                        .collect(),
                    weights: vec![1.; degree + 1],
                    periodic: false,
                }
            })
            .collect::<Vec<_>>();
        let loops = vec![wire];
        let c = certify(&loops, 1e-7).unwrap();
        assert!(c.separating_normals.iter().any(|n| *n >= 256 && *n < 1280));
        verify(&loops, 1e-7, &c).unwrap();
        let m = certify_measure(&loops, 1e-7, 1e-9).unwrap();
        assert_eq!(m.area_mm2.lower, ["128".to_string(), "1".to_string()]);
        assert_eq!(m.area_mm2.lower, m.area_mm2.upper);
    }
}

#[test]
fn extraction_does_not_accept_crossings_open_seams_or_excessive_work() {
    for points in [
        vec![
            vec![0., 0.],
            vec![4., 4.],
            vec![0., 4.],
            vec![4., 0.],
            vec![0., 0.],
        ],
        vec![vec![0., 0.], vec![4., 0.], vec![4., 4.], vec![0., 4.]],
    ] {
        assert!(certify(&[vec![Curve::from_polyline(points).unwrap()]], 1e-7).is_err());
    }
    let many = Curve::from_polyline((0..130).map(|i| vec![i as f64, 0.]).collect()).unwrap();
    assert!(certify(&[vec![many]], 1e-7).is_err());
}

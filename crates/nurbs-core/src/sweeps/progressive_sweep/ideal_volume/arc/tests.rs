use super::*;
fn curve(points: Vec<Vec<f64>>, domain: [f64; 2]) -> Curve {
    Curve {
        degree: 2,
        knots: [vec![domain[0]; 3], vec![domain[1]; 3]].concat(),
        control_points: points,
        weights: vec![1.; 3],
        periodic: false,
    }
}
fn contains(i: I, x: f64) {
    assert!(i.lo - 1e-12 <= x && x <= i.hi + 1e-12, "{x} outside {i:?}");
}
#[test]
fn inverse_normalized_length_derivatives_preserve_original_domain_units() {
    let p = curve(
        vec![vec![0.; 3], vec![0., 0., 16.], vec![0., 0., 64.]],
        [-3., 7.],
    );
    let before = p.clone();
    let mut used = 0;
    let map = ArcMap::build(&p, &mut used, 100000).unwrap().unwrap();
    assert!(used > 0);
    contains(map.total, 64.);
    for s in [0., 0.13, 0.5, 0.97, 1.] {
        let mut work = 0;
        let phase = map.phase([s, s], &mut work, 10000).unwrap().unwrap();
        let t = ((1. + 8. * s).sqrt() - 1.) / 2.;
        assert!(phase.traversal[0] <= t + 1e-12 && t <= phase.traversal[1] + 1e-12);
        contains(phase.first, 2. / (1. + 2. * t));
        contains(phase.second, -8. / (1. + 2. * t).powi(3));
        let [first, second] = phase.position_jets.unwrap();
        contains(first[2], 64.);
        contains(second[2], 0.);
        let mut short = 0;
        assert!(map.phase([s, s], &mut short, work - 1).unwrap().is_none());
        assert!(short <= work - 1);
    }
    let mut zero = 0;
    assert!(ArcMap::build(&p, &mut zero, 0).unwrap().is_none());
    assert_eq!(zero, 0);
    assert_eq!(p, before);
}
#[test]
fn curved_inverse_phase_matches_an_independent_exact_parabola_length_formula() {
    let p = curve(
        vec![vec![0.; 3], vec![0.5, 0., 0.], vec![1., 0., 1.]],
        [17., 19.],
    );
    let mut work = 0;
    let map = ArcMap::build(&p, &mut work, 100000).unwrap().unwrap();
    let length = |t: f64| t / 2. * (1. + 4. * t * t).sqrt() + (2. * t).asinh() / 4.;
    let total = length(1.);
    contains(map.total, total);
    for t in [0.125, 0.375, 0.625, 0.875] {
        let station = length(t) / total;
        let phase = map
            .phase([station - 1e-8, station + 1e-8], &mut work, 100000)
            .unwrap()
            .unwrap();
        assert!(phase.traversal[0] <= t && t <= phase.traversal[1]);
        let speed = (1. + 4. * t * t).sqrt();
        contains(phase.first, total / speed);
        contains(phase.second, -4. * t * total * total / speed.powi(4));
        let [first, second] = phase.position_jets.unwrap();
        contains(first[0], total / speed);
        contains(first[2], 2. * t * total / speed);
        contains(second[0], -4. * t * total * total / speed.powi(4));
        contains(second[2], 2. * total * total / speed.powi(4));
    }
}
#[test]
fn an_original_interior_zero_speed_never_produces_an_inverse_map() {
    let p = curve(vec![vec![0.; 3], vec![1., 0., 0.], vec![0.; 3]], [0., 1.]);
    let mut work = 0;
    assert!(ArcMap::build(&p, &mut work, 200).unwrap().is_none());
    assert!(work <= 200);
}

#[test]
fn refined_original_arc_jets_remove_fictitious_bending_on_a_nonuniform_line() {
    // Original normalized P(t)=(0,0,t+t²), with a different source knot domain.
    // Arc station makes P(s)=(0,0,2s): its second spatial jet is exactly zero.
    let p = curve(
        vec![vec![0.; 3], vec![0., 0., 0.5], vec![0., 0., 2.]],
        [-3., 5.],
    );
    let original = p.clone();
    let mut build = 0;
    let map = ArcMap::build(&p, &mut build, 100000).unwrap().unwrap();
    for station in [[0.13, 0.1301], [0.5, 0.5001], [0.97, 0.9701]] {
        let mut work = 0;
        let phase = map.phase(station, &mut work, 10000).unwrap().unwrap();
        let [first, second] = phase.position_jets.unwrap();
        for k in 0..3 {
            contains(first[k], if k == 2 { 2. } else { 0. });
            contains(second[k], 0.);
            assert!(second[k].hi - second[k].lo < 1e-10, "{second:?}");
        }
        for s in station {
            let t = ((1. + 8. * s).sqrt() - 1.) / 2.;
            assert!(phase.traversal[0] <= t && t <= phase.traversal[1]);
            contains(phase.first, 2. / (1. + 2. * t));
            contains(phase.second, -8. / (1. + 2. * t).powi(3));
        }
        let mut short = 0;
        assert!(map.phase(station, &mut short, work - 1).unwrap().is_none());
        assert!(short <= work - 1);
    }
    assert_eq!(p, original);
}

use super::*;
fn graph(degree: usize, domain: [[f64; 2]; 2]) -> Surface {
    Surface {
        degree_u: degree,
        degree_v: degree,
        knots_u: [
            vec![domain[0][0]; degree + 1],
            vec![domain[0][1]; degree + 1],
        ]
        .concat(),
        knots_v: [
            vec![domain[1][0]; degree + 1],
            vec![domain[1][1]; degree + 1],
        ]
        .concat(),
        control_points: (0..=degree)
            .map(|i| {
                (0..=degree)
                    .map(|j| {
                        vec![
                            i as f64 / degree as f64,
                            j as f64 / degree as f64,
                            if i == degree && j == degree { 1. } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; degree + 1]; degree + 1],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn mixed_bilinear_flux_has_zero_tensor_midpoint_remainder() {
    let s = graph(1, [[0., 1.], [0., 1.]]);
    let b = bound(&s, [[0., 1.], [0., 1.]], 2, 0., 1)
        .unwrap()
        .bound
        .unwrap();
    assert!(b.integral[0] <= 0.25 && b.integral[1] >= 0.25);
    assert!(b.integral[1] - b.integral[0] < 1e-12);
}
#[test]
fn pure_second_derivative_bounds_enclose_nonseparable_quartic_on_nonunit_clocks() {
    let s = graph(2, [[2., 5.], [-3., 1.]]);
    for uv in [[[2., 5.], [-3., 1.]], [[2.75, 4.25], [-2., 0.]]] {
        let b = bound(&s, uv, 2, 0., 1).unwrap().bound.unwrap();
        let u = uv[0].map(|x| (x - 2.) / 3.);
        let v = uv[1].map(|x| (x + 3.) / 4.);
        let exact = (u[1].powi(3) - u[0].powi(3)) * (v[1].powi(3) - v[0].powi(3)) / 9.;
        assert!(b.integral[0] <= exact && exact <= b.integral[1]);
    }
}

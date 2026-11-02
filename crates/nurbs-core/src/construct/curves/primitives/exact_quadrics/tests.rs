use super::*;
#[test]
fn literal_damage_wrong_basis_and_local_gauges_are_rejected() {
    let axes = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let s = ellipsoid([0.; 3], axes, 1_000_000)
        .unwrap()
        .surface
        .unwrap();
    let reference = Template::load(&mut Work {
        used: 0,
        limit: 1_000_000,
    })
    .unwrap();
    let run = |s: &Surface| {
        proof::verify(
            [0.; 3],
            axes,
            &reference,
            s,
            &mut Work {
                used: 0,
                limit: 1_000_000,
            },
        )
    };
    assert_eq!(run(&s), Ok(()));
    let mut wrong = s.clone();
    wrong.control_points[1][1][0] = 1f64.next_up();
    assert_eq!(run(&wrong), Err(Refusal::UnprovedIdentity));
    let mut wrong = s.clone();
    wrong.knots_v[3] = 0.25;
    wrong.knots_v[4] = 0.25;
    assert_eq!(run(&wrong), Err(Refusal::UnprovedIdentity));
    let mut wrong = s.clone();
    wrong.periodic_u = true;
    assert_eq!(run(&wrong), Err(Refusal::UnprovedIdentity));
    let mut wrong = s.clone();
    for row in &mut wrong.weights {
        row[2] *= 2.;
    }
    assert_eq!(run(&wrong), Err(Refusal::UnprovedIdentity));
    let mut wrong = s.clone();
    wrong.control_points[4][0][0] = f64::from_bits(1);
    assert_eq!(run(&wrong), Err(Refusal::UnprovedIdentity));
    let mut scaled = s;
    for weight in scaled.weights.iter_mut().flatten() {
        *weight *= 2.;
    }
    assert_eq!(run(&scaled), Ok(()));
}

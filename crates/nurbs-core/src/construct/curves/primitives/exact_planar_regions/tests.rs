use super::*;
#[test]
fn out_of_plane_damage_center_damage_clock_and_invalid_radial_laws_are_rejected() {
    let axes = [[1., 0., 0.], [0., 1., 0.]];
    let reference = Template::load(&mut Work {
        used: 0,
        limit: 1_000_000,
    })
    .unwrap();
    let s = radial([0.; 3], axes, [0., 2.], 1_000_000)
        .unwrap()
        .surface
        .unwrap();
    let run = |s: &Surface| {
        proof::verify(
            [0.; 3],
            axes,
            [0., 2.],
            &reference,
            s,
            &mut Work {
                used: 0,
                limit: 1_000_000,
            },
        )
    };
    assert_eq!(run(&s), Ok(()));
    let mut rotated = s.clone();
    for point in rotated.control_points.iter_mut().flatten() {
        let (x, y) = (point[0], point[1]);
        point[0] = -y;
        point[1] = x;
    }
    assert_eq!(run(&rotated), Err(Refusal::UnprovedIdentity));
    let mut wrong = s.clone();
    wrong.control_points[1][1][2] = f64::from_bits(1);
    assert_eq!(run(&wrong), Err(Refusal::UnprovedIdentity));
    let mut wrong = s.clone();
    wrong.control_points[1][1][0] = 2f64.next_up();
    assert_eq!(run(&wrong), Err(Refusal::UnprovedIdentity));
    let mut wrong = s.clone();
    wrong.control_points[3][0][0] = f64::from_bits(1);
    assert_eq!(run(&wrong), Err(Refusal::UnprovedIdentity));
    let mut wrong = s.clone();
    for row in &mut wrong.weights {
        row[1] *= 2.;
    }
    assert_eq!(run(&wrong), Err(Refusal::UnprovedIdentity));
    let mut wrong = s.clone();
    wrong.knots_v = vec![0., 0., 2., 2.];
    assert_eq!(run(&wrong), Err(Refusal::UnprovedIdentity));
    let mut wrong = s.clone();
    wrong.periodic_u = true;
    assert_eq!(run(&wrong), Err(Refusal::UnprovedIdentity));
    for radii in [[0., 0.], [1., 1.], [2., 1.], [-1., 1.]] {
        let bad = construct(
            [0.; 3],
            axes,
            radii,
            &reference,
            &mut Work {
                used: 0,
                limit: 1_000_000,
            },
        )
        .unwrap();
        assert_eq!(
            proof::verify(
                [0.; 3],
                axes,
                radii,
                &reference,
                &bad,
                &mut Work {
                    used: 0,
                    limit: 1_000_000
                }
            ),
            Err(Refusal::UnprovedIdentity)
        );
    }
    let mut scaled = s;
    for weight in scaled.weights.iter_mut().flatten() {
        *weight *= 2.;
    }
    assert_eq!(run(&scaled), Ok(()));
}

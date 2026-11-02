use super::*;
#[test]
fn literal_damage_clock_local_gauges_apex_damage_and_invalid_radii_are_refused() {
    let axes = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let reference = Template::load(&mut Work {
        used: 0,
        limit: 1_000_000,
    })
    .unwrap();
    let s = side([0.; 3], axes, [2., 1.], 1_000_000)
        .unwrap()
        .surface
        .unwrap();
    let run = |s: &Surface| {
        proof::verify(
            [0.; 3],
            axes,
            [2., 1.],
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
    wrong.knots_v = vec![0., 0., 2., 2.];
    assert_eq!(run(&wrong), Err(Refusal::UnprovedIdentity));
    let mut wrong = s.clone();
    for row in &mut wrong.weights {
        row[1] *= 2.;
    }
    assert_eq!(run(&wrong), Err(Refusal::UnprovedIdentity));
    let mut wrong = s.clone();
    wrong.periodic_u = true;
    assert_eq!(run(&wrong), Err(Refusal::UnprovedIdentity));
    let mut cone = side([0.; 3], axes, [2., 0.], 1_000_000)
        .unwrap()
        .surface
        .unwrap();
    cone.control_points[3][1][0] = f64::from_bits(1);
    assert_eq!(
        proof::verify(
            [0.; 3],
            axes,
            [2., 0.],
            &reference,
            &cone,
            &mut Work {
                used: 0,
                limit: 1_000_000
            }
        ),
        Err(Refusal::UnprovedIdentity)
    );
    for radii in [[0., 0.], [-1., 1.], [1., -1.]] {
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

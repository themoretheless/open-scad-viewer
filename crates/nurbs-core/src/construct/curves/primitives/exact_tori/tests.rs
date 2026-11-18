use super::*;
#[test]
fn literal_damage_wrong_basis_local_gauge_and_non_ring_radius_get_no_credit() {
    let axes = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let s = ring([0.; 3], axes, 2., 1., 1_000_000)
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
            2.,
            1.,
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
    wrong.control_points[1][1][0] = 3f64.next_up();
    assert_eq!(run(&wrong), Err(Refusal::UnprovedIdentity));
    let mut wrong = s.clone();
    wrong.knots_v[3] = 0.2;
    wrong.knots_v[4] = 0.2;
    assert_eq!(run(&wrong), Err(Refusal::UnprovedIdentity));
    let mut wrong = s.clone();
    wrong.periodic_v = true;
    assert_eq!(run(&wrong), Err(Refusal::UnprovedIdentity));
    let mut wrong = s.clone();
    for row in &mut wrong.weights {
        row[2] *= 2.;
    }
    assert_eq!(run(&wrong), Err(Refusal::UnprovedIdentity));
    let mut wrong = s.clone();
    wrong.control_points[8][3][0] = wrong.control_points[8][3][0].next_up();
    assert_eq!(run(&wrong), Err(Refusal::UnprovedIdentity));
    for (major, minor) in [(1., 1.), (0.5, 1.), (1., 0.)] {
        let outside_domain = construct(
            [0.; 3],
            axes,
            major,
            minor,
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
                major,
                minor,
                &reference,
                &outside_domain,
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
